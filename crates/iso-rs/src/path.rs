use std::io::Read;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PathTableEntryHeader {
    pub len: u8,
    pub extended_attr_record: u8,
    pub parent_directory_number: [u8; 4],
}

impl PathTableEntryHeader {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        *bytemuck::from_bytes(bytes)
    }
}

#[derive(Debug, Clone)]
pub struct PathTableEntry {
    pub header: PathTableEntryHeader,
    pub name: String,
}

impl PathTableEntry {
    pub fn parse<T: Read>(reader: &mut T) -> Result<Self, std::io::Error> {
        let mut buf = [0; size_of::<PathTableEntryHeader>()];
        reader.read_exact(&mut buf)?;
        let header = PathTableEntryHeader::from_bytes(&buf);
        let mut name = vec![0; header.len as usize];
        reader.read_exact(&mut name)?;

        Ok(Self {
            header,
            name: String::from_utf8(name).unwrap(),
        })
    }

    pub fn size(&self) -> usize {
        size_of::<PathTableEntryHeader>() + self.name.len()
    }
}
