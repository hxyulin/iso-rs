use std::{
    io::{Read, Seek, Write},
    mem::offset_of,
};

use boot::{
    BootCatalogueEntry, BootInitialEntry, BootSectionEntry, BootSectionHeaderEntry,
    BootValidationEntry, PlatformId,
};
use types::{
    BigEndian, DecDateTime, FileInterchange, Filename, IsoStrA, IsoStrD, LittleEndian, U16,
    U16LsbMsb, U32, U32LsbMsb,
};

pub mod boot;
pub mod types;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeDescriptorType {
    BootRecord = 0x00,
    PrimaryVolumeDescriptor = 0x01,
    SupplementaryVolumeDescriptor = 0x02,
    VolumePartitionDescriptor = 0x03,
    VolumeSetTerminator = 0xFF,
}

impl VolumeDescriptorType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(Self::BootRecord),
            0x01 => Some(Self::PrimaryVolumeDescriptor),
            0x02 => Some(Self::SupplementaryVolumeDescriptor),
            0x03 => Some(Self::VolumePartitionDescriptor),
            0xFF => Some(Self::VolumeSetTerminator),
            _ => None,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VolumeDescriptorHeader {
    pub descriptor_type: u8,
    pub standard_identifier: IsoStrA<5>,
    pub version: u8,
}

pub struct BootRecord {
    pub header: VolumeDescriptorHeader,
    pub boot_system_identifier: IsoStrA<32>,
    pub boot_identifier: IsoStrA<32>,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct PrimaryVolumeDescriptor {
    pub header: VolumeDescriptorHeader,
    pub unused0: u8,
    pub system_identifier: IsoStrA<32>,
    pub volume_identifier: IsoStrD<32>,
    pub unused1: [u8; 8],
    pub volume_space_size: U32LsbMsb,
    pub unused2: [u8; 32],
    pub volume_set_size: U16LsbMsb,
    pub volume_sequence_number: U16LsbMsb,
    pub logical_block_size: U16LsbMsb,
    pub path_table_size: U32LsbMsb,
    pub type_l_path_table: U32<LittleEndian>,
    pub opt_type_l_path_table: U32<LittleEndian>,
    pub type_m_path_table: U32<BigEndian>,
    pub opt_type_m_path_table: U32<BigEndian>,
    pub dir_record: [u8; 34],
    pub volume_set_identifier: IsoStrD<128>,
    pub publisher_identifier: IsoStrA<128>,
    pub preparer_identifier: IsoStrA<128>,
    pub application_identifier: IsoStrA<128>,
    pub copyright_file_identifier: IsoStrD<37>,
    pub abstract_file_identifier: IsoStrD<37>,
    pub bibliographic_file_identifier: IsoStrD<37>,
    pub creation_date: DecDateTime,
    pub modification_date: DecDateTime,
    pub expiration_date: DecDateTime,
    pub effective_date: DecDateTime,
    pub file_structure_version: u8,
    pub unused3: u8,
    pub app_data: [u8; 512],
    pub reserved: [u8; 653],
}

impl PrimaryVolumeDescriptor {
    pub fn new(sectors: u32) -> Self {
        use types::Endian;
        Self {
            header: VolumeDescriptorHeader {
                descriptor_type: VolumeDescriptorType::PrimaryVolumeDescriptor as u8,
                standard_identifier: IsoStrA::from_str("CD001").unwrap(),
                version: 1,
            },
            unused0: 0,
            system_identifier: IsoStrA::empty(),
            volume_identifier: IsoStrD::from_str("ISOIMAGE").unwrap(),
            unused1: [0; 8],
            volume_space_size: U32LsbMsb::new(sectors),
            unused2: [0; 32],
            volume_set_size: U16LsbMsb::new(1),
            volume_sequence_number: U16LsbMsb::new(1),
            logical_block_size: U16LsbMsb::new(2048),
            path_table_size: U32LsbMsb::new(0),
            type_l_path_table: U32::<LittleEndian>::new(0),
            opt_type_l_path_table: U32::<LittleEndian>::new(0),
            type_m_path_table: U32::<BigEndian>::new(0),
            opt_type_m_path_table: U32::<BigEndian>::new(0),
            dir_record: [0; 34],
            volume_set_identifier: IsoStrD::empty(),
            publisher_identifier: IsoStrA::empty(),
            preparer_identifier: IsoStrA::empty(),
            application_identifier: IsoStrA::from_str("ISO-RS").unwrap(),
            copyright_file_identifier: IsoStrD::empty(),
            abstract_file_identifier: IsoStrD::empty(),
            bibliographic_file_identifier: IsoStrD::empty(),
            creation_date: DecDateTime::now(),
            modification_date: DecDateTime::now(),
            expiration_date: DecDateTime::now(),
            effective_date: DecDateTime::now(),
            file_structure_version: 1,
            unused3: 0,
            app_data: [0; 512],
            reserved: [0; 653],
        }
    }
}

unsafe impl bytemuck::Zeroable for PrimaryVolumeDescriptor {}
unsafe impl bytemuck::Pod for PrimaryVolumeDescriptor {}

pub struct VolumeDescriptorSetTerminator(pub VolumeDescriptorHeader);

pub struct PathTableEntry<F: FileInterchange> {
    pub len: u8,
    pub extended_attr_record: u8,
    pub parent_directory_number: [u8; 4],
    pub name: Filename<F>,
    pub padding: F::Padding,
}

/// The header of a directory record, because the identifier is variable length,
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DirectoryRecordHeader {
    pub len: u8,
    pub extended_attr_record: u8,
    /// The LBA of the record
    pub extent: U32LsbMsb,
    /// The length of the data in bytes
    pub data_len: U32LsbMsb,
    pub date_time: DirDateTime,
    pub flags: u8,
    pub file_unit_size: u8,
    pub interleave_gap_size: u8,
    pub volume_sequence_number: U16LsbMsb,
    pub file_identifier_len: u8,
}

struct DirectoryRecord {
    pub header: DirectoryRecordHeader,
    pub name: Vec<u8>,
}

impl DirectoryRecord {
    pub fn size(&self) -> usize {
        size_of::<DirectoryRecordHeader>() + self.name.len()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(bytemuck::bytes_of(&self.header));
        bytes.extend_from_slice(&self.name);
        bytes
    }
}

/// The root directory entry
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RootDirectoryEntry {
    pub header: DirectoryRecordHeader,
    /// There is no name on the root directory, so this is always empty
    pub padding: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DirDateTime {
    /// Number of years since 1900
    year: u8,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    offset: u8,
}

impl Default for DirDateTime {
    fn default() -> Self {
        Self {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            offset: 0,
        }
    }
}

bitflags::bitflags! {
    pub struct FileFlags: u8 {
        const HIDDEN = 0b0000_0001;
        const DIRECTORY = 0b0000_0010;
        const ASSOCIATED_FILE = 0b0000_0100;
        const EXTENDED_ATTRIBUTES = 0b0000_1000;
        const EXTENDED_PERMISSIONS = 0b0001_0000;
        const NOT_FINAL = 0b1000_0000;
    }
}

pub struct IsoFile {
    pub name: String,
    pub data: Vec<u8>,
}

pub struct FormatOptions {
    el_torito: Option<ElToritoOptions>,
    files: Vec<IsoFile>,
}

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

pub struct IsoImage<'a, T: ReadWriteSeek> {
    pub data: &'a mut T,
    pub size: usize,
}

impl<'a, T: ReadWriteSeek> IsoImage<'a, T> {
    pub fn format_new(data: &'a mut T, ops: FormatOptions) -> Result<Self, std::io::Error> {
        let size = data.seek(std::io::SeekFrom::End(0)).unwrap() as usize;
        assert!(
            size % 2048 == 0,
            "Size must be a multiple of 2048, got {}",
            size
        );

        let mut image = Self { data, size };

        // We start at 20 to avoid the reserved area
        image
            .data
            .seek(std::io::SeekFrom::Start(20 * 2048))
            .unwrap();

        let mut pvd = PrimaryVolumeDescriptor::new((size / 2048) as u32);

        let mut boot_catalogue: Vec<BootCatalogueEntry> = Vec::new();

        if let Some(el_torito) = ops.el_torito {
            use types::Endian;

            let boot_code = if let Some(boot_image) = &el_torito.boot_image {
                // FIXME: El-Torito is complaiining about the boot catalogue if we have a boot
                // image, so there is some logic bug here
                let sector = image.current_sector();
                image.data.write_all(&boot_image)?;
                Some((sector, to_sectors_ceil(boot_image.len()) as u32))
            } else {
                None
            };

            image.align()?;

            let brvd = boot::BootRecordVolumeDescriptor {
                boot_record_indicator: 0,
                iso_identifier: IsoStrA::from_str("CD001").unwrap(),
                version: 1,
                boot_system_identifier: *b"EL TORITO SPECIFICATION\0\0\0\0\0\0\0\0\0",
                unused0: [0; 32],
                // Catalog is after the boot code
                catalog_ptr: U32::<LittleEndian>::new(image.current_sector() as u32),
                unused1: [0; 1973],
            };
            boot_catalogue.push(BootCatalogueEntry::VolumeDescriptor(brvd));

            let mut validation_entry = BootValidationEntry {
                header_id: 0x01,
                platform_id: boot::PlatformId::X80X86 as u8,
                reserved: [0; 2],
                manufacturer: [0; 24],
                checksum: U16::<LittleEndian>::new(0),
                key: [0x55, 0xAA],
            };
            validation_entry
                .checksum
                .set(validation_entry.calculate_checksum());
            boot_catalogue.push(BootCatalogueEntry::Validation(validation_entry));
            // Default entry is UEFI
            let default_entry = BootInitialEntry {
                boot_indicator: 0x88,
                boot_media_type: 0x00,
                load_segment: U16::<LittleEndian>::new(0),
                system_type: 0x00,
                reserved0: 0x00,
                sector_count: U16::<LittleEndian>::new(el_torito.load_size),
                load_rba: U32::<LittleEndian>::new(0),
                reserved1: [0; 20],
            };
            boot_catalogue.push(BootCatalogueEntry::Initial(default_entry));
            if let Some((lba, size)) = boot_code {
                // Section for BIOS Boot
                let section_header = BootSectionHeaderEntry {
                    header_type: 0x91,
                    platform_id: PlatformId::UEFI as u8,
                    section_count: U16::<LittleEndian>::new(1),
                    section_ident: [0u8; 27],
                };
                boot_catalogue.push(BootCatalogueEntry::SectionHeader(section_header));
                let section = BootSectionEntry {
                    boot_indicator: 0x88,
                    boot_media_type: 0x00,
                    load_segment: U16::<LittleEndian>::new(0),
                    system_type: 0x00,
                    reserved0: 0x00,
                    sector_count: U16::<LittleEndian>::new(size as u16),
                    load_rba: U32::<LittleEndian>::new(lba as u32),
                    selection_criteria: 0x00,
                    vendor_unique: [0; 19],
                };
                boot_catalogue.push(BootCatalogueEntry::SectionEntry(section));
                for entry in boot_catalogue.iter() {
                    image.data.write_all(entry.as_bytes())?;
                }
            }
        }

        image.align()?;
        // TODO: Support nested directories
        let mut file_entries: Vec<DirectoryRecord> = Vec::new();
        let total_size = file_entries.iter().map(|r| r.size()).sum::<usize>();
        for file in ops.files {
            // TODO: Create the files and append to the file_entries
            // and then Write the file
            // At the same time we populate the path table
        }

        // Now we do a second pass to write the file entries

        image.align()?;

        let root_dir_entry = RootDirectoryEntry {
            header: DirectoryRecordHeader {
                len: size_of::<RootDirectoryEntry>() as u8 + 1,
                extended_attr_record: 0,
                extent: U32LsbMsb::new(image.current_sector() as u32),
                data_len: U32LsbMsb::new(total_size as u32),
                date_time: DirDateTime::default(),
                flags: 0,
                file_unit_size: 0,
                interleave_gap_size: 0,
                volume_sequence_number: U16LsbMsb::new(0),
                file_identifier_len: 0,
            },
            padding: 0,
        };
        pvd.dir_record
            .copy_from_slice(bytemuck::bytes_of(&root_dir_entry));

        // Now we can write the directory records
        for entry in file_entries {
            image.data.write_all(&entry.to_bytes())?;
        }

        // And we write the end record
        let end_record = DirectoryRecordHeader {
            len: 0,
            extended_attr_record: 0,
            extent: U32LsbMsb::new(0),
            data_len: U32LsbMsb::new(0),
            date_time: DirDateTime::default(),
            flags: 0,
            file_unit_size: 0,
            interleave_gap_size: 0,
            volume_sequence_number: U16LsbMsb::new(0),
            file_identifier_len: 0,
        };
        image.data.write_all(bytemuck::bytes_of(&end_record))?;

        // Now we can write everything at the end
        image.data.seek(std::io::SeekFrom::Start(16 * 2048))?;
        image.data.write_all(bytemuck::bytes_of(&pvd))?;

        Ok(image)
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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_pvd_new() {
        let pvd = PrimaryVolumeDescriptor::new(1024);
        println!("{:?}", &pvd);
    }

    #[test]
    fn test_new_image() {
        let options = FormatOptions {
            el_torito: Some(ElToritoOptions {
                load_size: 4,
                boot_image: None,
            }),
            files: Vec::new(),
        };
        let mut data = Cursor::new(vec![0; 1024 * 2048]);
        let image = IsoImage::format_new(&mut data, options);
        drop(image);
        data.seek(std::io::SeekFrom::Start(0)).unwrap();
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open("test.iso")
            .unwrap();
        let mut buffer = [0u8; 32768];
        loop {
            let read = data.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read]).unwrap();
        }
    }
}
