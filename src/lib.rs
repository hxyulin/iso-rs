use types::{
    BigEndian, DecDateTime, FileInterchange, Filename, IsoStrA, IsoStrD, LittleEndian, U16LsbMsb,
    U32, U32LsbMsb,
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
            system_identifier: IsoStrA::from_str("Example-OS").unwrap(),
            volume_identifier: IsoStrD::from_str("EXAMPLEOS").unwrap(),
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
#[derive(Debug, Clone, Copy)]
pub struct DirectoryRecordHeader {
    pub len: u8,
    pub extended_attr_record: u8,
    pub extent: U32LsbMsb,
    pub data_len: U32LsbMsb,
    pub date_time: DecDateTime,
    pub flags: u8,
    pub file_unit_size: u8,
    pub interleave_gap_size: u8,
    pub volume_sequence_number: U16LsbMsb,
    pub file_identifier_len: u8,
}

#[derive(Debug, Clone, Copy)]
struct DirDateTime {
    /// Number of years since 1900
    year: u8,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    offset: u8,
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

pub struct FormatOptions {
    el_torito: Option<ElToritoOptions>,
}

pub struct ElToritoOptions {
    // Emulating is not supported
    load_size: u32,
}

pub struct IsoImage {
    // TODO: We make a this Read Write trait instead of in memory Vec
    pub data: Vec<u8>,
    pub cur_sector: usize,
}

impl IsoImage {
    pub fn format_new(size: usize, ops: FormatOptions) -> Self {
        assert!(size % 2048 == 0, "Size must be a multiple of 2048");
        assert!(size >= 16 * 2048, "Size must be at least 16 sectors");
        let mut image = Self {
            data: vec![0; size],
            cur_sector: 17,
        };

        *image.pvd() = PrimaryVolumeDescriptor::new((size / 2048) as u32);
        if let Some(el_torito) = ops.el_torito {
            use types::Endian;
            let brvd = boot::BootRecordVolumeDescriptor {
                boot_record_indicator: 0,
                iso_identifier: IsoStrA::from_str("CD001").unwrap(),
                version: 1,
                boot_system_identifier: *b"EL TORITO SPECIFICATION\0\0\0\0\0\0\0\0\0",
                unused0: [0; 32],
                catalog_ptr: U32::<LittleEndian>::new(0),
                unused1: [0; 1972],
            };
            // TODO: We need to write boot catalogue, and then come back and write the volume descriptor becaue we need the pointer (relative to the start of the image) to the boot catalogue.
            unimplemented!()
        }

        image
    }

    pub fn pvd(&mut self) -> &mut PrimaryVolumeDescriptor {
        let offset = 16 * 2048;
        bytemuck::from_bytes_mut(&mut self.data[offset..offset + 2048])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pvd_new() {
        let pvd = PrimaryVolumeDescriptor::new(1024);
        println!("{:?}", &pvd);
    }

    #[test]
    fn test_new_image() {
        let image = IsoImage::format_new(1024 * 2048);
        std::fs::write("test.iso", &image.data).unwrap();
    }
}
