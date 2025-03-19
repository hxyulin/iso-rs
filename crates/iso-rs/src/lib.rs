use std::{
    collections::{BTreeMap, VecDeque},
    io::{Read, Seek, SeekFrom, Write},
};

use directory::{DirectoryRecord, DirectoryRecordHeader, DirectoryRef};
use path::PathTableEntry;
use types::{Endian, IsoStringFile};
use volume::{PrimaryVolumeDescriptor, VolumeDescriptor, VolumeDescriptorList};

pub mod boot;
pub mod directory;
pub mod path;
pub mod types;
pub mod volume;

#[derive(Debug, Clone)]
pub enum IsoFile {
    Directory { name: String, entries: Vec<IsoFile> },
    File { name: String, data: Vec<u8> },
}

impl IsoFile {
    pub fn name(&self) -> &str {
        match self {
            Self::Directory { name, .. } => name,
            Self::File { name, .. } => name,
        }
    }

    pub fn set_name(&mut self, new_name: String) {
        match self {
            Self::Directory { name, .. } => *name = new_name,
            Self::File { name, .. } => *name = new_name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FormatOptions {
    //el_torito: Option<ElToritoOptions>,
    pub files: Vec<IsoFile>,
}

#[derive(Debug, Clone)]
pub struct ElToritoOptions {
    // Emulating is not supported
    load_size: u16,
    boot_image: Option<Vec<u8>>,
}

pub trait ReadWriteSeek: Read + Write + Seek {}
impl<T: Read + Write + Seek> ReadWriteSeek for T {}

fn to_sectors_ceil(size: usize) -> usize {
    (size + 2047) / 2048
}

#[derive(Debug)]
pub struct IsoImage<'a, T: ReadWriteSeek> {
    data: &'a mut T,
    size: u64,

    volume_descriptors: VolumeDescriptorList,
    root_directory: DirectoryRef,
    path_table: PathTableRef,
}

pub struct IsoDirectory<'a, T: ReadWriteSeek> {
    reader: &'a mut T,
    directory: DirectoryRef,
}

#[derive(Debug, Clone, Copy)]
pub struct PathTableRef {
    lpath_table_offset: u64,
    mpath_table_offset: u64,
    size: u64,
}

pub struct IsoPathTable<'a, T: ReadWriteSeek> {
    reader: &'a mut T,
    path_table: PathTableRef,
}

impl<'a, T: ReadWriteSeek> IsoPathTable<'a, T> {
    pub fn entries(&mut self) -> Result<Vec<PathTableEntry>, std::io::Error> {
        self.reader
            .seek(SeekFrom::Start(self.path_table.lpath_table_offset))?;
        let mut entries = Vec::new();
        let mut idx = 0;
        while idx < self.path_table.size as usize {
            let entry = PathTableEntry::parse(self.reader)?;
            if entry.name.is_empty() {
                break;
            }
            idx += entry.size();
            entries.push(entry);
        }
        Ok(entries)
    }
}

impl<'a, T: ReadWriteSeek> IsoDirectory<'a, T> {
    pub fn entries(&mut self) -> Result<Vec<DirectoryRecord>, std::io::Error> {
        self.reader
            .seek(SeekFrom::Start(self.directory.offset * 2048))?;
        // This is the easiest implementation, but it's not the most efficient
        // because we are storing the entire directory in memory.
        let mut bytes = vec![0; self.directory.size as usize];
        self.reader.read_exact(&mut bytes)?;
        let mut entries = Vec::new();
        let mut idx = 0;
        while idx < bytes.len() {
            let entry = DirectoryRecordHeader::from_bytes(
                &bytes[idx..idx + size_of::<DirectoryRecordHeader>()],
            );
            if entry.len == 0 {
                break;
            }
            let name = IsoStringFile::from_bytes(
                &bytes[idx + size_of::<DirectoryRecordHeader>()
                    ..idx
                        + size_of::<DirectoryRecordHeader>()
                        + entry.file_identifier_len as usize],
            );
            idx += entry.len as usize;
            entries.push(DirectoryRecord {
                header: *entry,
                name,
            });
        }
        Ok(entries)
    }
}

impl<'a, T: ReadWriteSeek> IsoImage<'a, T> {
    pub fn format_new(data: &'a mut T, ops: FormatOptions) -> Result<(), std::io::Error> {
        let size_bytes = data.seek(SeekFrom::End(0))?;
        let size_sectors = size_bytes / 2048;

        let mut volume_descriptors = VolumeDescriptorList::empty();

        volume_descriptors.push(VolumeDescriptor::Primary(PrimaryVolumeDescriptor::new(
            size_sectors as u32,
        )));

        let mut current_index = 16 * 2048;
        current_index += volume_descriptors.size_required();
        let pvd = volume_descriptors.primary_mut();

        // Now we write all the files
        data.seek(SeekFrom::Start(current_index as u64))?;

        let mut directories: VecDeque<IsoFile> = VecDeque::new();
        directories.push_back(IsoFile::Directory {
            name: "".to_string(),
            // Cloning here is super slow
            entries: ops.files.clone(),
        });

        data.seek(SeekFrom::Start(current_index as u64))?;
        let mut written_files: BTreeMap<String, DirectoryRef> = BTreeMap::new();

        while let Some(entry) = directories.pop_front() {
            let (files, dir_name) = match entry {
                IsoFile::Directory { entries, name } => (entries, name),
                _ => unreachable!(),
            };

            for mut file in files {
                let (name, contents) = match &file {
                    IsoFile::Directory { name, .. } => {
                        file.set_name(format!("{}/{}", dir_name, name));
                        directories.push_back(file);
                        continue;
                    }
                    IsoFile::File { name, data } => (format!("{}/{}", dir_name, name), data),
                };

                assert!(current_index % 2048 == 0);
                written_files.insert(
                    name.clone(),
                    DirectoryRef {
                        offset: (current_index / 2048) as u64,
                        size: contents.len() as u64,
                    },
                );
                data.write_all(&contents)?;
                current_index += contents.len();
                if current_index % 2048 != 0 {
                    let offset = 2048 - (current_index % 2048);
                    current_index += offset;
                    data.seek(SeekFrom::Current(offset as i64))?;
                }
            }
        }

        const BYTES_PER_ENTRY: usize =
            size_of::<DirectoryRecordHeader>() + size_of::<IsoStringFile>();

        assert!(current_index % 2048 == 0);
        pvd.dir_record
            .header
            .extent
            .write((current_index / 2048) as u32);

        let mut parent_dir: Option<DirectoryRef> = None;
        let mut directories: VecDeque<IsoFile> = VecDeque::new();
        directories.push_back(IsoFile::Directory {
            name: "".to_string(),
            entries: ops.files,
        });

        let root_dir_start = current_index;
        data.seek(SeekFrom::Start(root_dir_start as u64))?;
        let root_dir_sector = root_dir_start / 2048;

        while let Some(entry) = directories.pop_front() {
            let files = match entry {
                IsoFile::Directory { entries, .. } => entries,
                _ => unreachable!(),
            };

            let mut current_dir = DirectoryRecord::directory(&[0x01]);
            let mut parent_dir_ent = DirectoryRecord::directory(&[0x02]);
            current_dir.header.extent.write(root_dir_sector as u32);
            current_dir.header.data_len.write(2048);
            match parent_dir {
                None => {
                    parent_dir_ent.header.extent.write(root_dir_sector as u32);
                    parent_dir_ent.header.data_len.write(2048);

                    pvd.dir_record.header.extent.write(root_dir_sector as u32);
                    pvd.dir_record.header.data_len.write(2048);
                    // TODO: Write to parent dir
                }
                Some(..) => unimplemented!(),
            }

            current_index += current_dir.write(data)?;
            current_index += parent_dir_ent.write(data)?;

            for mut file in files {
                // TODO: To support writing to nested directories, we probably need to use
                // post-order traversal, and a second pass to populate the directory records
                // (parent directory sizes, and extents)
                let (name, orig_name, _contents) = match &mut file {
                    IsoFile::Directory { name, .. } => unimplemented!(),
                    IsoFile::File { name, data } => (format!("/{}", name), name, data),
                };

                let dir_ref = written_files.get(&name).unwrap();
                let entry = DirectoryRecord::file(orig_name.as_bytes(), *dir_ref);
                println!("Writing {:?}", entry);
                current_index += entry.write(data)?;

                if current_index - root_dir_start > 2048 {
                    panic!("Only 1 sector root directory is supported");
                }
            }

            if current_index % 2048 != 0 {
                let offset = 2048 - (current_index % 2048);
                current_index += offset;
                data.seek(SeekFrom::Current(offset as i64))?;
            }
        }

        assert!(current_index % 2048 == 0);
        let current_cluster = current_index / 2048;
        pvd.path_table_size.write(2048);
        pvd.type_l_path_table.set(current_cluster as u32);
        pvd.type_m_path_table.set(current_cluster as u32);

        data.seek(SeekFrom::Start(16 * 2048))?;
        volume_descriptors.write(data)?;

        Ok(())
    }

    pub fn new(data: &'a mut T) -> Result<Self, std::io::Error> {
        data.seek(SeekFrom::Start(16 * 2048))?;
        let volume_descriptors = VolumeDescriptorList::parse(data)?;
        let size = data.seek(SeekFrom::End(0))?;

        let pvd = volume_descriptors.primary();
        let root_entry = pvd.dir_record;
        let root_directory = DirectoryRef {
            offset: root_entry.header.extent.read() as u64,
            size: root_entry.header.data_len.read() as u64,
        };

        let path_table = PathTableRef {
            lpath_table_offset: pvd.type_l_path_table.get() as u64,
            mpath_table_offset: pvd.type_m_path_table.get() as u64,
            size: pvd.path_table_size.read() as u64,
        };

        Ok(Self {
            data,
            size,

            volume_descriptors,
            root_directory,
            path_table,
        })
    }

    pub fn root_directory(&mut self) -> IsoDirectory<T> {
        IsoDirectory {
            reader: &mut self.data,
            directory: self.root_directory,
        }
    }

    pub fn path_table(&mut self) -> IsoPathTable<T> {
        IsoPathTable {
            reader: &mut self.data,
            path_table: self.path_table,
        }
    }

    fn current_sector(&mut self) -> usize {
        let seek = self.data.seek(std::io::SeekFrom::Current(0)).unwrap();
        assert!(seek % 2048 == 0, "Seek must be a multiple of 2048");
        (seek / 2048) as usize
    }

    fn align(&mut self) -> Result<u64, std::io::Error> {
        let current_seek = self.data.seek(std::io::SeekFrom::Current(0))?;
        let padded_end = (current_seek + 2047) & !2047;
        self.data.seek(std::io::SeekFrom::Start(padded_end))?;
        Ok(padded_end)
    }
}
