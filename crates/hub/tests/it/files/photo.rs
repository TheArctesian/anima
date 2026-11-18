//! A small JPEG whose EXIF data says 52°30′ N, 4°15′ W, built in the test.

fn entry(tag: u16, kind: u16, count: u32, value: [u8; 4]) -> Vec<u8> {
    [
        tag.to_le_bytes().as_slice(),
        &kind.to_le_bytes(),
        &count.to_le_bytes(),
        &value,
    ]
    .concat()
}

/// The rationals of the position: what must not reach a download.
pub fn gps_rationals() -> Vec<u8> {
    let mut rationals = Vec::new();
    for (degrees, minutes) in [(52_u32, 30_u32), (4, 15)] {
        for (numerator, denominator) in [(degrees, 1_u32), (minutes, 1), (0, 1)] {
            rationals.extend(numerator.to_le_bytes());
            rationals.extend(denominator.to_le_bytes());
        }
    }
    rationals
}

fn tiff() -> Vec<u8> {
    let mut block = b"II".to_vec();
    block.extend(42_u16.to_le_bytes());
    block.extend(8_u32.to_le_bytes());
    block.extend(1_u16.to_le_bytes());
    block.extend(entry(0x8825, 4, 1, 26_u32.to_le_bytes()));
    block.extend(0_u32.to_le_bytes());
    block.extend(4_u16.to_le_bytes());
    block.extend(entry(1, 2, 2, *b"N\0\0\0"));
    block.extend(entry(2, 5, 3, 80_u32.to_le_bytes()));
    block.extend(entry(3, 2, 2, *b"W\0\0\0"));
    block.extend(entry(4, 5, 3, 104_u32.to_le_bytes()));
    block.extend(0_u32.to_le_bytes());
    block.extend(gps_rationals());
    block
}

fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
    let length = u16::try_from(payload.len() + 2).unwrap();
    [&[0xFF, marker], length.to_be_bytes().as_slice(), payload].concat()
}

/// The photo: EXIF with GPS, then image data large enough for two parts of a piece.
pub fn photo() -> Vec<u8> {
    let image: Vec<u8> = (0..70_000_u32)
        .map(|i| u8::try_from(i % 200).unwrap())
        .collect();
    [
        vec![0xFF, 0xD8],
        segment(0xE1, &[b"Exif\0\0".as_slice(), &tiff()].concat()),
        segment(0xDB, &[0; 5]),
        vec![0xFF, 0xDA],
        image,
        vec![0xFF, 0xD9],
    ]
    .concat()
}
