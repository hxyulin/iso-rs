use core::marker::PhantomData;
use std::time::SystemTime;

use chrono::Datelike;

pub trait Charset: Copy {
    fn is_valid(chars: &[u8]) -> bool;
}

/// The `a-characters` character set.
/// This supports `a-z`, `A-Z`, `0-9` and `!"%$'()*+,-./:;<=>?`.
#[derive(Copy, Clone)]
pub struct CharsetA;
#[derive(Copy, Clone)]
pub struct CharsetD;

impl Charset for CharsetA {
    fn is_valid(chars: &[u8]) -> bool {
        const VALID_SYMBOLS: &[u8] = b"!\"%$'()*+,-./:;<=>?";
        chars
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || VALID_SYMBOLS.contains(c))
    }
}

impl Charset for CharsetD {
    fn is_valid(chars: &[u8]) -> bool {
        const SPECIAL_CHARS: &[u8] = b"0123456789_";
        chars
            .iter()
            .all(|c| c.is_ascii_uppercase() || SPECIAL_CHARS.contains(c))
    }
}

/// A space padded string with a fixed length.
#[derive(Clone, Copy, PartialEq)]
pub struct IsoStr<C: Charset, const N: usize> {
    chars: [u8; N],
    _marker: PhantomData<C>,
}

unsafe impl<C: Charset, const N: usize> bytemuck::Zeroable for IsoStr<C, N> {}
unsafe impl<C: Charset + 'static, const N: usize> bytemuck::Pod for IsoStr<C, N> {}

impl<C: Charset, const N: usize> IsoStr<C, N> {
    pub fn empty() -> Self {
        Self {
            chars: [b' '; N],
            _marker: core::marker::PhantomData,
        }
    }

    pub fn max_len() -> usize {
        N
    }

    pub fn len(&self) -> usize {
        self.chars.iter().position(|&c| c == b' ').unwrap_or(N)
    }

    // TODO: Error type
    pub fn from_str(s: &str) -> Result<Self, ()> {
        let mut chars = [b' '; N];
        if s.len() > N {
            return Err(());
        }

        if !C::is_valid(s.as_bytes()) {
            return Err(());
        }

        for (i, c) in s.bytes().enumerate() {
            chars[i] = c;
        }
        Ok(Self {
            chars,
            _marker: core::marker::PhantomData,
        })
    }

    pub fn to_str(&self) -> &str {
        // SAFETY: The string is constructed from valid ASCII characters.
        unsafe { core::str::from_utf8_unchecked(&self.chars) }
    }
}

impl<C: Charset, const N: usize> core::fmt::Display for IsoStr<C, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl<C: Charset, const N: usize> core::fmt::Debug for IsoStr<C, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

pub type IsoStrA<const N: usize> = IsoStr<CharsetA, N>;
pub type IsoStrD<const N: usize> = IsoStr<CharsetD, N>;

pub trait FileInterchange {
    type Padding: Copy + Default;
}

/// A level 1 `microsoft` filename,
/// which comes from the FAT 8.3 standard.
pub struct InterchangeL1 {
    basename: IsoStrD<8>,
    extension: IsoStrD<3>,
}

impl FileInterchange for InterchangeL1 {
    // If it is even, then we need to add a padding byte, because of the version byte.
    type Padding = u8;
}

/// A filename, which can be either a level 1 or level 2 filename.
/// And a padding byte if the filename is odd
pub struct Filename<F: FileInterchange> {
    file: F,
    version: u8,
}

pub type FilenameL1 = Filename<InterchangeL1>;

// Endian types copied from https://github.com/hxyulin/hadris

pub enum EndianType {
    AnyEndian,
    LittleEndian,
    BigEndian,
}

pub trait Endianness: Copy {
    fn get() -> EndianType;

    fn get_u16(bytes: [u8; 2]) -> u16;
    fn set_u16(value: u16, bytes: &mut [u8; 2]);
    fn get_u32(bytes: [u8; 4]) -> u32;
    fn set_u32(value: u32, bytes: &mut [u8; 4]);
    fn get_u64(bytes: [u8; 8]) -> u64;
    fn set_u64(value: u64, bytes: &mut [u8; 8]);
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub struct NativeEndian;
#[repr(transparent)]
#[derive(Debug, Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub struct LittleEndian;
#[repr(transparent)]
#[derive(Debug, Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub struct BigEndian;

impl Endianness for NativeEndian {
    #[inline]
    fn get() -> EndianType {
        EndianType::AnyEndian
    }

    #[inline]
    fn get_u16(bytes: [u8; 2]) -> u16 {
        u16::from_ne_bytes(bytes)
    }

    #[inline]
    fn set_u16(value: u16, bytes: &mut [u8; 2]) {
        bytes.copy_from_slice(&value.to_ne_bytes());
    }

    #[inline]
    fn get_u32(bytes: [u8; 4]) -> u32 {
        u32::from_ne_bytes(bytes)
    }

    #[inline]
    fn set_u32(value: u32, bytes: &mut [u8; 4]) {
        bytes.copy_from_slice(&value.to_ne_bytes());
    }

    #[inline]
    fn get_u64(bytes: [u8; 8]) -> u64 {
        u64::from_ne_bytes(bytes)
    }

    #[inline]
    fn set_u64(value: u64, bytes: &mut [u8; 8]) {
        bytes.copy_from_slice(&value.to_ne_bytes());
    }
}

impl Endianness for LittleEndian {
    #[inline]
    fn get() -> EndianType {
        EndianType::LittleEndian
    }

    #[inline]
    fn get_u16(bytes: [u8; 2]) -> u16 {
        u16::from_le_bytes(bytes)
    }

    #[inline]
    fn set_u16(value: u16, bytes: &mut [u8; 2]) {
        bytes.copy_from_slice(&value.to_le_bytes());
    }

    #[inline]
    fn get_u32(bytes: [u8; 4]) -> u32 {
        u32::from_le_bytes(bytes)
    }

    #[inline]
    fn set_u32(value: u32, bytes: &mut [u8; 4]) {
        bytes.copy_from_slice(&value.to_le_bytes());
    }

    #[inline]
    fn get_u64(bytes: [u8; 8]) -> u64 {
        u64::from_le_bytes(bytes)
    }

    #[inline]
    fn set_u64(value: u64, bytes: &mut [u8; 8]) {
        bytes.copy_from_slice(&value.to_le_bytes());
    }
}
impl Endianness for BigEndian {
    #[inline]
    fn get() -> EndianType {
        EndianType::BigEndian
    }

    #[inline]
    fn get_u16(bytes: [u8; 2]) -> u16 {
        u16::from_be_bytes(bytes)
    }

    #[inline]
    fn set_u16(value: u16, bytes: &mut [u8; 2]) {
        bytes.copy_from_slice(&value.to_be_bytes());
    }

    #[inline]
    fn get_u32(bytes: [u8; 4]) -> u32 {
        u32::from_be_bytes(bytes)
    }

    #[inline]
    fn set_u32(value: u32, bytes: &mut [u8; 4]) {
        bytes.copy_from_slice(&value.to_be_bytes());
    }

    #[inline]
    fn get_u64(bytes: [u8; 8]) -> u64 {
        u64::from_be_bytes(bytes)
    }

    #[inline]
    fn set_u64(value: u64, bytes: &mut [u8; 8]) {
        bytes.copy_from_slice(&value.to_be_bytes());
    }
}

pub trait Endian {
    type Output: bytemuck::Pod + bytemuck::Zeroable;
    type LsbType: bytemuck::Pod + bytemuck::Zeroable + Endian<Output = Self::Output>;
    type MsbType: bytemuck::Pod + bytemuck::Zeroable + Endian<Output = Self::Output>;

    fn new(value: Self::Output) -> Self;
    fn get(&self) -> Self::Output;
    fn set(&mut self, value: Self::Output);
}

#[repr(transparent)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct U16<E: Endianness> {
    bytes: [u8; 2],
    _marker: PhantomData<E>,
}

impl<E: Endianness> Endian for U16<E> {
    type Output = u16;
    type LsbType = U16<LittleEndian>;
    type MsbType = U16<BigEndian>;

    fn new(value: u16) -> Self {
        let mut bytes = [0; 2];
        E::set_u16(value, &mut bytes);
        Self {
            bytes,
            _marker: PhantomData,
        }
    }

    fn get(&self) -> u16 {
        E::get_u16(self.bytes)
    }

    fn set(&mut self, value: u16) {
        E::set_u16(value, &mut self.bytes);
    }
}

impl<E: Endianness> core::fmt::Debug for U16<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("U16").field(&self.get()).finish()
    }
}

impl<E: Endianness> core::fmt::LowerHex for U16<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let value = self.get();
        write!(f, "0x{:04x}", value)
    }
}

impl<E: Endianness> core::fmt::UpperHex for U16<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let value = self.get();
        write!(f, "0x{:04X}", value)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct U32<E: Endianness> {
    bytes: [u8; 4],
    _marker: PhantomData<E>,
}

impl<E: Endianness> Endian for U32<E> {
    type Output = u32;
    type LsbType = U32<LittleEndian>;
    type MsbType = U32<BigEndian>;

    fn new(value: u32) -> Self {
        let mut bytes = [0; 4];
        E::set_u32(value, &mut bytes);
        Self {
            bytes,
            _marker: PhantomData,
        }
    }

    fn get(&self) -> u32 {
        E::get_u32(self.bytes)
    }

    fn set(&mut self, value: u32) {
        E::set_u32(value, &mut self.bytes);
    }
}

impl<E: Endianness> core::fmt::Debug for U32<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("U32").field(&self.get()).finish()
    }
}

impl<E: Endianness> core::fmt::LowerHex for U32<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let value = self.get();
        write!(f, "0x{:08x}", value)
    }
}

impl<E: Endianness> core::fmt::UpperHex for U32<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let value = self.get();
        write!(f, "0x{:08X}", value)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct U64<E: Endianness> {
    bytes: [u8; 8],
    _marker: PhantomData<E>,
}

impl<E: Endianness> Endian for U64<E> {
    type Output = u64;
    type LsbType = U64<LittleEndian>;
    type MsbType = U64<BigEndian>;

    fn new(value: u64) -> Self {
        let mut bytes = [0; 8];
        E::set_u64(value, &mut bytes);
        Self {
            bytes,
            _marker: PhantomData,
        }
    }

    fn get(&self) -> u64 {
        E::get_u64(self.bytes)
    }

    fn set(&mut self, value: u64) {
        E::set_u64(value, &mut self.bytes);
    }
}

impl<E: Endianness> core::fmt::Debug for U64<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("U64").field(&self.get()).finish()
    }
}

impl<E: Endianness> core::fmt::LowerHex for U64<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let value = self.get();
        write!(f, "0x{:016x}", value)
    }
}

impl<E: Endianness> core::fmt::UpperHex for U64<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let value = self.get();
        write!(f, "0x{:016X}", value)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LsbMsb<T: Endian> {
    lsb: T::LsbType,
    msb: T::MsbType,
}

unsafe impl<T: Endian> bytemuck::Zeroable for LsbMsb<T> {}
unsafe impl<T: Endian + Copy + 'static> bytemuck::Pod for LsbMsb<T> {}

impl<T: Endian> LsbMsb<T> {
    pub fn new(value: T::Output) -> Self {
        Self {
            lsb: Endian::new(value),
            msb: Endian::new(value),
        }
    }

    pub fn read(&self) -> T::Output {
        #[cfg(target_endian = "little")]
        {
            self.lsb.get()
        }
        #[cfg(target_endian = "big")]
        {
            self.msb.get()
        }
    }

    pub fn write(&mut self, value: T::Output) {
        self.lsb.set(value);
        self.msb.set(value);
    }
}

pub type U16LsbMsb = LsbMsb<U16<LittleEndian>>;
pub type U32LsbMsb = LsbMsb<U32<LittleEndian>>;
pub type U64LsbMsb = LsbMsb<U64<LittleEndian>>;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DecDateTime {
    pub year: IsoStrD<4>,
    pub month: IsoStrD<2>,
    pub day: IsoStrD<2>,
    pub hour: IsoStrD<2>,
    pub minute: IsoStrD<2>,
    pub second: IsoStrD<2>,
    pub hundredths: IsoStrD<2>,
    pub timezone: u8,
}

impl DecDateTime {
    pub fn now() -> Self {
        use chrono::{DateTime, Datelike, Timelike, Utc};
        let now: DateTime<Utc> = SystemTime::now().into();
        Self {
            year: IsoStrD::from_str(&now.year().to_string()).unwrap(),
            month: IsoStrD::from_str(&now.month().to_string()).unwrap(),
            day: IsoStrD::from_str(&now.day().to_string()).unwrap(),
            hour: IsoStrD::from_str(&now.hour().to_string()).unwrap(),
            minute: IsoStrD::from_str(&now.minute().to_string()).unwrap(),
            second: IsoStrD::from_str(&now.second().to_string()).unwrap(),
            hundredths: IsoStrD::from_str(&(now.nanosecond() / 10_000_000).to_string()).unwrap(),
            timezone: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u16() {
        let mut value = U16::<NativeEndian>::new(0x1234);
        assert_eq!(value.get(), 0x1234);
        value.set(0x5678);
        assert_eq!(value.get(), 0x5678);
    }

    #[test]
    fn test_u32() {
        let mut value = U32::<NativeEndian>::new(0x12345678);
        assert_eq!(value.get(), 0x12345678);
        value.set(0x9abcdef0);
        assert_eq!(value.get(), 0x9abcdef0);
    }

    #[test]
    fn test_u64() {
        let mut value = U64::<NativeEndian>::new(0x123456789abcdef0);
        assert_eq!(value.get(), 0x123456789abcdef0);
        value.set(0x0123456789abcdef);
        assert_eq!(value.get(), 0x0123456789abcdef);
    }
}
