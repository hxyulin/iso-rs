use crate::types::{IsoStrA, LittleEndian, U16, U32};

/// Types for El Torito boot catalogue
/// The boot catalogue consists of a series of boot catalogue entries:
/// First, the validation entry
/// Next, the initial/default entry
/// Section headers,
/// Section entries,
/// Section entry extensions

#[derive(Debug, Clone, Copy)]
pub enum BootCatalogueEntry {
    Validation(BootValidationEntry),
    Initial(BootInitialEntry),
    SectionHeader(BootSectionHeaderEntry),
    SectionEntry(BootSectionEntry),
    SectionEntryExtension(BootSectionEntryExtension),
    VolumeDescriptor(BootRecordVolumeDescriptor),
}

impl BootCatalogueEntry {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            BootCatalogueEntry::Validation(entry) => bytemuck::bytes_of(entry),
            BootCatalogueEntry::Initial(entry) => bytemuck::bytes_of(entry),
            BootCatalogueEntry::SectionHeader(entry) => bytemuck::bytes_of(entry),
            BootCatalogueEntry::SectionEntry(entry) => bytemuck::bytes_of(entry),
            BootCatalogueEntry::SectionEntryExtension(entry) => bytemuck::bytes_of(entry),
            BootCatalogueEntry::VolumeDescriptor(entry) => bytemuck::bytes_of(entry),
        }
    }

    pub fn size(&self) -> usize {
        match self {
            BootCatalogueEntry::Validation(_) => size_of::<BootValidationEntry>(),
            BootCatalogueEntry::Initial(_) => size_of::<BootInitialEntry>(),
            BootCatalogueEntry::SectionHeader(_) => size_of::<BootSectionHeaderEntry>(),
            BootCatalogueEntry::SectionEntry(_) => size_of::<BootSectionEntry>(),
            BootCatalogueEntry::SectionEntryExtension(_) => size_of::<BootSectionEntryExtension>(),
            BootCatalogueEntry::VolumeDescriptor(_) => size_of::<BootRecordVolumeDescriptor>(),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum PlatformId {
    /// This is for X8086, X86, and X86_64 architectures.
    X80X86 = 0x00,
    PowerPC = 0x01,
    Macintosh = 0x02,
    UEFI = 0xEF,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct BootValidationEntry {
    pub header_id: u8,
    pub platform_id: u8,
    pub reserved: [u8; 2],
    pub manufacturer: [u8; 24],
    pub checksum: U16<LittleEndian>,
    /// 0x55AA
    pub key: [u8; 2],
}

impl BootValidationEntry {
    pub fn calculate_checksum(&self) -> u16 {
        let bytes = bytemuck::bytes_of(self);
        let mut checksum = 0u16;
        for i in (0..32).step_by(2) {
            let value = u16::from_le_bytes([bytes[i], bytes[i + 1]]);
            checksum = checksum.wrapping_add(value);
        }
        u16::MAX - checksum + 1
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct BootInitialEntry {
    pub boot_indicator: u8,
    pub boot_media_type: u8,
    pub load_segment: U16<LittleEndian>,
    pub system_type: u8,
    pub reserved0: u8,
    pub sector_count: U16<LittleEndian>,
    pub load_rba: U32<LittleEndian>,
    pub reserved1: [u8; 20],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BootSectionHeaderEntry {
    /// 0x90 = Header, more headers follow
    /// 0x91 = Final header
    pub header_type: u8,
    pub platform_id: u8,
    pub section_count: U16<LittleEndian>,
    pub section_ident: [u8; 27],
}

unsafe impl bytemuck::Zeroable for BootSectionHeaderEntry {}
unsafe impl bytemuck::Pod for BootSectionHeaderEntry {}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BootSectionEntry {
    /// 0x88 = Bootable, 0x00 = Not bootable
    pub boot_indicator: u8,
    pub boot_media_type: u8,
    pub load_segment: U16<LittleEndian>,
    pub system_type: u8,
    pub reserved0: u8,
    pub sector_count: U16<LittleEndian>,
    pub load_rba: U32<LittleEndian>,
    pub selection_criteria: u8,
    pub vendor_unique: [u8; 19],
}

unsafe impl bytemuck::Zeroable for BootSectionEntry {}
unsafe impl bytemuck::Pod for BootSectionEntry {}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BootSectionEntryExtension {
    // Must be 0x44
    pub extension_indicator: u8,
    // Bit 5: 1 = more extensions follow, 0 = final extension
    pub flags: u8,
    pub vendor_unique: [u8; 30],
}

unsafe impl bytemuck::Zeroable for BootSectionEntryExtension {}
unsafe impl bytemuck::Pod for BootSectionEntryExtension {}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BootRecordVolumeDescriptor {
    /// Must be set to 0
    pub boot_record_indicator: u8,
    /// iso identifier, should be "CD001"
    pub iso_identifier: IsoStrA<5>,
    pub version: u8,
    pub boot_system_identifier: [u8; 32],
    pub unused0: [u8; 32],
    pub catalog_ptr: U32<LittleEndian>,
    pub unused1: [u8; 1973],
}

unsafe impl bytemuck::Zeroable for BootRecordVolumeDescriptor {}
unsafe impl bytemuck::Pod for BootRecordVolumeDescriptor {}
