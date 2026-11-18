//! The GPS part of the EXIF data of a photo (FIL-003). Pure, and it trusts no length or offset
//! of the data: each read is checked against the bounds of the block.

/// The tag of IFD0 that points at the GPS IFD.
const GPS_POINTER: u16 = 0x8825;
const LATITUDE_REF: u16 = 1;
const LATITUDE: u16 = 2;
const LONGITUDE_REF: u16 = 3;
const LONGITUDE: u16 = 4;
const ENTRY: usize = 12;

/// A position in degrees, as the photo gives it. It is metadata of the file, not evidence of
/// where the operator was.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Clone, Copy)]
struct Tiff {
    big_endian: bool,
}

impl Tiff {
    fn u16(self, block: &[u8], at: usize) -> Option<u16> {
        let bytes: [u8; 2] = block.get(at..at.checked_add(2)?)?.try_into().ok()?;
        Some(if self.big_endian {
            u16::from_be_bytes(bytes)
        } else {
            u16::from_le_bytes(bytes)
        })
    }

    fn u32(self, block: &[u8], at: usize) -> Option<u32> {
        let bytes: [u8; 4] = block.get(at..at.checked_add(4)?)?.try_into().ok()?;
        Some(if self.big_endian {
            u32::from_be_bytes(bytes)
        } else {
            u32::from_le_bytes(bytes)
        })
    }

    fn offset(self, block: &[u8], at: usize) -> Option<usize> {
        usize::try_from(self.u32(block, at)?).ok()
    }
}

/// The byte order of a TIFF block and the offset of its IFD0.
fn header(block: &[u8]) -> Option<(Tiff, usize)> {
    let tiff = match block.get(..2)? {
        b"II" => Tiff { big_endian: false },
        b"MM" => Tiff { big_endian: true },
        _ => return None,
    };
    (tiff.u16(block, 2)? == 42).then_some(())?;
    Some((tiff, tiff.offset(block, 4)?))
}

/// The entries of the IFD at `at`: the offset of each 12-byte entry.
fn entries(tiff: Tiff, block: &[u8], at: usize) -> Option<impl Iterator<Item = usize>> {
    let count = usize::from(tiff.u16(block, at)?);
    let first = at.checked_add(2)?;
    let end = first.checked_add(count.checked_mul(ENTRY)?)?;
    (end <= block.len()).then(|| (0..count).map(move |index| first + index * ENTRY))
}

/// The offset of the GPS IFD that IFD0 points at.
fn gps_ifd(tiff: Tiff, block: &[u8], ifd0: usize) -> Option<usize> {
    entries(tiff, block, ifd0)?
        .find(|&entry| tiff.u16(block, entry) == Some(GPS_POINTER))
        .and_then(|entry| tiff.offset(block, entry + 8))
}

/// The size of an entry's value in bytes, from its type and count.
fn value_size(tiff: Tiff, block: &[u8], entry: usize) -> Option<usize> {
    let unit: usize = match tiff.u16(block, entry + 2)? {
        1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 => 4,
        5 | 10 | 12 => 8,
        _ => return None,
    };
    unit.checked_mul(usize::try_from(tiff.u32(block, entry + 4)?).ok()?)
}

/// Where the value of an entry is: in the entry when it fits in 4 bytes, else at its offset.
fn value_at(tiff: Tiff, block: &[u8], entry: usize) -> Option<(usize, usize)> {
    let size = value_size(tiff, block, entry)?;
    let at = if size <= 4 {
        entry + 8
    } else {
        tiff.offset(block, entry + 8)?
    };
    (at.checked_add(size)? <= block.len()).then_some((at, size))
}

/// Degrees from three unsigned rationals (degrees, minutes, seconds).
fn degrees(tiff: Tiff, block: &[u8], entry: usize) -> Option<f64> {
    let (at, size) = value_at(tiff, block, entry)?;
    (tiff.u16(block, entry + 2)? == 5 && size == 24).then_some(())?;
    let mut total = 0.0;
    for (index, scale) in [1.0, 60.0, 3600.0].into_iter().enumerate() {
        let numerator = f64::from(tiff.u32(block, at + index * 8)?);
        let denominator = f64::from(tiff.u32(block, at + index * 8 + 4)?);
        if denominator == 0.0 {
            return None;
        }
        total += numerator / denominator / scale;
    }
    Some(total)
}

/// The first byte of an ASCII reference such as `N` or `W`.
fn reference(tiff: Tiff, block: &[u8], entry: usize) -> Option<u8> {
    let (at, size) = value_at(tiff, block, entry)?;
    (size >= 1).then(|| block[at])
}

/// The position in the GPS IFD at `at`, when it has one in range.
fn position(tiff: Tiff, block: &[u8], at: usize) -> Option<Position> {
    let mut parts = (None, None, None, None);
    for entry in entries(tiff, block, at)? {
        match tiff.u16(block, entry)? {
            LATITUDE_REF => parts.0 = reference(tiff, block, entry),
            LATITUDE => parts.1 = degrees(tiff, block, entry),
            LONGITUDE_REF => parts.2 = reference(tiff, block, entry),
            LONGITUDE => parts.3 = degrees(tiff, block, entry),
            _ => {}
        }
    }
    let (Some(north), Some(latitude), Some(east), Some(longitude)) = parts else {
        return None;
    };
    let latitude = if north == b'S' { -latitude } else { latitude };
    let longitude = if east == b'W' { -longitude } else { longitude };
    ((-90.0..=90.0).contains(&latitude) && (-180.0..=180.0).contains(&longitude)).then_some(
        Position {
            latitude,
            longitude,
        },
    )
}

/// Reads the position of a TIFF block (the EXIF data after `Exif\0\0`) and then removes the GPS
/// part: each value of the GPS IFD and each of its entries become zero bytes, and its count
/// becomes 0, so the block keeps its length and every other offset in it. A block whose GPS IFD
/// cannot be found holds no GPS part that a reader could find.
pub fn remove_gps(block: &mut [u8]) -> Option<Position> {
    let (tiff, ifd0) = header(block)?;
    let at = gps_ifd(tiff, block, ifd0)?;
    let found = position(tiff, block, at);
    let Some(entries) = entries(tiff, block, at).map(Iterator::collect::<Vec<usize>>) else {
        // A GPS IFD cut by the end of the block: a lenient reader could still find its first
        // entries, so everything from it to the end goes.
        if let Some(rest) = block.get_mut(at..) {
            rest.fill(0);
        }
        return None;
    };
    for &entry in &entries {
        if let Some((value, size)) = value_at(tiff, block, entry).filter(|(_, size)| *size > 4) {
            block[value..value + size].fill(0);
        }
        block[entry..entry + ENTRY].fill(0);
    }
    block[at..at + 2].fill(0);
    found
}
