use std::{
    collections::{BTreeMap, VecDeque},
    fmt::Debug,
    io::{Read, Seek, SeekFrom, Write},
};

use directory::{DirectoryRecord, DirectoryRecordHeader, DirectoryRef, FileFlags};
use path::PathTableEntry;
use types::{Endian, IsoStringFile};
use volume::{PrimaryVolumeDescriptor, VolumeDescriptor, VolumeDescriptorList};

pub mod boot;
pub mod directory;
pub mod path;
pub mod types;
pub mod volume;

#[derive(Clone)]
pub enum IsoFile {
    Directory { name: String, entries: Vec<IsoFile> },
    File { name: String, data: Vec<u8> },
}

impl Debug for IsoFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IsoFile::Directory { name, entries } => f
                .debug_struct("Directory")
                .field("name", &name)
                .field("entries", &entries)
                .finish(),
            IsoFile::File { name, data } => f
                .debug_struct("File")
                .field("name", &name)
                .field("data_len", &data.len())
                .finish(),
        }
    }
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
        // TODO: Some sort of strict check that checks both tables?

        // We always read from the native endian table
        let offset = if cfg!(target_endian = "little") {
            self.path_table.lpath_table_offset
        } else {
            self.path_table.mpath_table_offset
        };
        self.reader.seek(SeekFrom::Start(offset * 2048))?;
        let mut entries = Vec::new();
        let mut idx = 0;
        while idx < self.path_table.size as usize {
            let entry = PathTableEntry::parse(self.reader, types::EndianType::NativeEndian)?;
            if entry.length == 0 {
                break;
            }
            idx += entry.size();
            entries.push(entry);
        }
        Ok(entries)
    }
}

impl<'a, T: ReadWriteSeek> IsoDirectory<'a, T> {
    // TODO: Make this private after testing
    /// Returns a list of all entries in the directory, along with their offset in the directory
    pub fn entries(&mut self) -> Result<Vec<(u64, DirectoryRecord)>, std::io::Error> {
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
            entries.push((
                idx as u64,
                DirectoryRecord {
                    header: *entry,
                    name,
                },
            ));
            idx += entry.len as usize;
        }
        Ok(entries)
    }

    pub fn find_directory(
        &mut self,
        name: &str,
    ) -> Result<Option<IsoDirectory<T>>, std::io::Error> {
        let entry = self.entries()?.iter().find_map(|(_offset, entry)| {
            if entry.name.to_str() == name
                && FileFlags::from_bits_retain(entry.header.flags).contains(FileFlags::DIRECTORY)
            {
                Some(entry.clone())
            } else {
                None
            }
        });
        match entry {
            Some(entry) => Ok(Some(IsoDirectory {
                reader: self.reader,
                directory: DirectoryRef {
                    offset: entry.header.extent.read() as u64,
                    size: entry.header.data_len.read() as u64,
                },
            })),
            None => Ok(None),
        }
    }

    pub fn read_file(&mut self, name: &str) -> Result<Vec<u8>, std::io::Error> {
        let entry = self.entries()?.iter().find_map(|(_offset, entry)| {
            if entry.name.to_str() == name {
                Some(entry.clone())
            } else {
                None
            }
        });
        match entry {
            Some(entry) => {
                let mut bytes = vec![0; entry.header.data_len.read() as usize];
                self.reader
                    .seek(SeekFrom::Start(entry.header.extent.read() as u64))?;
                self.reader.read_exact(&mut bytes)?;
                Ok(bytes)
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not found",
            )),
        }
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

        let mut current_index: u64 = 16 * 2048;
        current_index += volume_descriptors.size_required() as u64;
        let pvd = volume_descriptors.primary_mut();

        // Now we write all the files
        data.seek(SeekFrom::Start(current_index as u64))?;

        let mut file_writer = FileWriter::new(data, ops.files);
        let (root_dir, path_table) = file_writer.write()?;

        pvd.dir_record.header.extent.write(root_dir.offset as u32);
        pvd.dir_record.header.data_len.write(root_dir.size as u32);
        pvd.path_table_size.write(path_table.size as u32);
        pvd.type_l_path_table.set(path_table.offset as u32);
        pvd.type_m_path_table
            .set(path_table.offset as u32 + (path_table.size / 2048) as u32);

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

    fn current_sector(data: &mut T) -> usize {
        let seek = data.seek(std::io::SeekFrom::Current(0)).unwrap();
        assert!(seek % 2048 == 0, "Seek must be a multiple of 2048");
        (seek / 2048) as usize
    }

    fn align(data: &mut T) -> Result<u64, std::io::Error> {
        let current_seek = data.seek(std::io::SeekFrom::Current(0))?;
        let padded_end = (current_seek + 2047) & !2047;
        data.seek(std::io::SeekFrom::Start(padded_end))?;
        Ok(padded_end)
    }
}

#[derive(Debug)]
struct FileWriter<'a, W: ReadWriteSeek> {
    writer: &'a mut W,

    /// A flat-map of the files
    files: Vec<IsoFile>,
    written_files: BTreeMap<String, DirectoryRef>,
}

impl<'a, W: ReadWriteSeek> FileWriter<'a, W> {
    pub fn new(writer: &'a mut W, file_tree: Vec<IsoFile>) -> Self {
        let mut files = Vec::new();

        Self::flatmap_recursive(
            &mut files,
            IsoFile::Directory {
                name: "".to_string(),
                entries: file_tree,
            },
            "",
        );
        // TODO: Optimize algorithm to not require this
        files.reverse();

        Self {
            writer,
            files,
            written_files: BTreeMap::new(),
        }
    }

    /// Writes the file data, directory data, and the path table to the given writer, returning a
    /// tuple containing the root directory and the path table.
    pub fn write(&mut self) -> Result<(DirectoryRef, DirectoryRef), std::io::Error> {
        self.write_file_data()?;
        let root_dir = self.write_directory_data()?;
        let path_table = self.write_path_table(&root_dir)?;
        Ok((root_dir, path_table))
    }

    fn write_file_data(&mut self) -> Result<(), std::io::Error> {
        for file in &self.files {
            if let IsoFile::File { name, data } = file {
                let size_aligned = (data.len() + 2047) & !2047;
                self.written_files.insert(
                    name.clone(),
                    DirectoryRef {
                        offset: IsoImage::current_sector(self.writer) as u64,
                        size: size_aligned as u64,
                    },
                );
                self.writer.write_all(data)?;
                IsoImage::align(self.writer)?;
            }
        }
        Ok(())
    }

    fn write_directory_data(&mut self) -> Result<DirectoryRef, std::io::Error> {
        let current_dir_ent = DirectoryRecord::directory(&[0x00], DirectoryRef::default());
        let parent_dir_ent = DirectoryRecord::directory(&[0x01], DirectoryRef::default());

        // In the first pass, we just write all of the directories from the leaves
        for file in &self.files {
            if let IsoFile::Directory { name, entries } = file {
                let start_sector = IsoImage::current_sector(self.writer);
                // We can just leave these as default, we modify them in a second pass
                current_dir_ent.write(self.writer)?;
                parent_dir_ent.write(self.writer)?;

                for entry in entries {
                    let orig_name = entry.name().split('/').last().unwrap();
                    let file_ref = self.written_files.get(entry.name()).unwrap();
                    let ent = match entry {
                        IsoFile::Directory { .. } => {
                            DirectoryRecord::directory(orig_name.as_bytes(), *file_ref)
                        }
                        IsoFile::File { .. } => {
                            DirectoryRecord::file(orig_name.as_bytes(), *file_ref)
                        }
                    };
                    ent.write(self.writer)?;
                }

                let end = IsoImage::align(self.writer)?;
                let directory_ref = DirectoryRef {
                    offset: start_sector as u64,
                    size: end - start_sector as u64 * 2048,
                };
                self.written_files.insert(name.clone(), directory_ref);
            }
        }

        let root_dir = self.written_files.get("").unwrap().clone();
        let mut stack = vec![(&root_dir, &root_dir, "".to_string())];

        while let Some((dir_ref, parent_ref, cur_path)) = stack.pop() {
            let start = dir_ref.offset * 2048;
            self.writer.seek(SeekFrom::Start(start))?;

            DirectoryRecord::directory(&[0x00], *dir_ref).write(&mut self.writer)?;
            DirectoryRecord::directory(&[0x01], *parent_ref).write(&mut self.writer)?;

            let mut reader = IsoDirectory {
                reader: self.writer,
                directory: *dir_ref,
            };
            for (offset, directory) in reader
                .entries()?
                .iter()
                .filter(|(_offset, entry)| entry.header.is_directory())
            {
                // Special cases for the current and parent directories
                if directory.name.bytes() == b"\x00" || directory.name.bytes() == b"\x01" {
                    continue;
                }
                let dirname = format!("{}/{}", cur_path, directory.name);
                let dir_ref_inner = self.written_files.get(dirname.as_str()).unwrap();
                let mut new_entry = directory.clone();
                new_entry.header.extent.write(dir_ref_inner.offset as u32);
                new_entry.header.data_len.write(dir_ref_inner.size as u32);
                self.writer.seek(SeekFrom::Start(start + offset))?;
                new_entry.write(&mut self.writer)?;
                stack.push((dir_ref_inner, dir_ref, dirname));
            }
        }

        // We need to seek back to the end of the directory record list, which is the root directory
        self.writer
            .seek(SeekFrom::Start(root_dir.offset * 2048 + root_dir.size))?;

        Ok(root_dir)
    }

    fn write_path_table(
        &mut self,
        root_dir: &DirectoryRef,
    ) -> Result<DirectoryRef, std::io::Error> {
        let start_sector = IsoImage::current_sector(self.writer);
        let mut entries = Vec::new();
        let mut index = 1; // Root directory is always index 1
        let mut parent_map = std::collections::HashMap::new();

        // Write the root directory
        entries.push(PathTableEntry {
            length: 1,
            extended_attr_record: 0,
            parent_lba: root_dir.offset as u32,
            parent_index: 1,
            name: "\0".to_string(),
        });

        parent_map.insert("".to_string(), 1);

        for file in &self.files {
            if let IsoFile::Directory { name, .. } = file {
                if name.is_empty() {
                    // We already wrote the root directory
                    continue;
                }
                let directory_ref = self.written_files.get(name).unwrap();
                let parent_name = name.rsplit_once('/').map(|(p, _)| p).unwrap_or("");

                let parent_index = *parent_map.get(parent_name).unwrap_or(&1);
                parent_map.insert(name.clone(), index);

                entries.push(PathTableEntry {
                    length: name.len() as u8,
                    name: name.clone(),
                    extended_attr_record: 0,
                    parent_lba: directory_ref.offset as u32,
                    parent_index,
                });

                index += 1;
            }
        }

        // Write L-Table (Little-Endian)
        for entry in &entries {
            self.writer
                .write_all(&entry.to_bytes(types::EndianType::LittleEndian))?;
        }

        // Align to sector boundary
        let end = IsoImage::align(self.writer)?;

        // We only store the L-table ref, but the M-table can be found by just adding the size of
        // the L-table to the offset of the L-table to find the offset of the M-table.
        let path_table_ref = DirectoryRef {
            offset: start_sector as u64,
            size: end - start_sector as u64 * 2048,
        };

        // Write M-Table (Big-Endian)
        for entry in &entries {
            self.writer
                .write_all(&entry.to_bytes(types::EndianType::BigEndian))?;
        }

        let mtable_end = IsoImage::align(self.writer)?;
        assert_eq!(mtable_end - end, path_table_ref.size);

        Ok(path_table_ref)
    }

    fn flatmap_recursive(files: &mut Vec<IsoFile>, file: IsoFile, cur_path: &str) {
        match file {
            IsoFile::Directory { name, entries } => {
                let mut path = format!("{}/{}", cur_path, name);
                if path.ends_with('/') {
                    path.pop();
                }

                files.push(IsoFile::Directory {
                    name: path.clone(),
                    // We create new entries, with just the name, and no data
                    entries: entries
                        .iter()
                        .map(|e| match e {
                            IsoFile::File { name, data: _ } => IsoFile::File {
                                name: format!("{}/{}", path, name),
                                data: Vec::new(),
                            },
                            IsoFile::Directory { name, entries: _ } => IsoFile::Directory {
                                name: format!("{}/{}", path, name),
                                entries: Vec::new(),
                            },
                        })
                        .collect(),
                });
                for entry in entries {
                    Self::flatmap_recursive(files, entry, &path);
                }
            }
            IsoFile::File { name, data } => {
                let mut path = format!("{}/{}", cur_path, name);
                if path.ends_with('/') {
                    path.pop();
                }
                files.push(IsoFile::File { name: path, data });
            }
        }
    }
}
