use proptest::prelude::*;

use super::{Cleaner, Position, is_jpeg};

fn rational(numerator: u32, denominator: u32) -> Vec<u8> {
    [numerator.to_le_bytes(), denominator.to_le_bytes()].concat()
}

fn entry(tag: u16, kind: u16, count: u32, value: [u8; 4]) -> Vec<u8> {
    [
        tag.to_le_bytes().as_slice(),
        &kind.to_le_bytes(),
        &count.to_le_bytes(),
        &value,
    ]
    .concat()
}

/// A little-endian TIFF block whose GPS IFD says 52°30′ N, 4°15′ W.
fn tiff() -> Vec<u8> {
    let mut block = b"II".to_vec();
    block.extend(42_u16.to_le_bytes());
    block.extend(8_u32.to_le_bytes());
    // IFD0 at 8: one entry, the GPS pointer to 26.
    block.extend(1_u16.to_le_bytes());
    block.extend(entry(0x8825, 4, 1, 26_u32.to_le_bytes()));
    block.extend(0_u32.to_le_bytes());
    // The GPS IFD at 26: four entries, the rationals at 80 and 104.
    block.extend(4_u16.to_le_bytes());
    block.extend(entry(1, 2, 2, *b"N\0\0\0"));
    block.extend(entry(2, 5, 3, 80_u32.to_le_bytes()));
    block.extend(entry(3, 2, 2, *b"W\0\0\0"));
    block.extend(entry(4, 5, 3, 104_u32.to_le_bytes()));
    block.extend(0_u32.to_le_bytes());
    for (degrees, minutes) in [(52, 30), (4, 15)] {
        block.extend(rational(degrees, 1));
        block.extend(rational(minutes, 1));
        block.extend(rational(0, 1));
    }
    block
}

fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
    let length = u16::try_from(payload.len() + 2).unwrap();
    [&[0xFF, marker], length.to_be_bytes().as_slice(), payload].concat()
}

const XMP: &[u8] = b"http://ns.adobe.com/xap/1.0/\0<exif:GPSLatitude>52,30N</exif:GPSLatitude>";
const IMAGE: &[u8] = &[0x12, 0x34, 0xFF, 0x00, 0x56, 0xFF, 0xD9];

fn photo() -> Vec<u8> {
    [
        vec![0xFF, 0xD8],
        segment(0xE0, b"JFIF\0\x01\x02"),
        segment(0xE1, &[b"Exif\0\0".as_slice(), &tiff()].concat()),
        segment(0xE1, XMP),
        segment(0xDB, &[0; 5]),
        vec![0xFF, 0xDA],
        IMAGE.to_vec(),
    ]
    .concat()
}

fn cleaned(bytes: &[u8], chunk: usize) -> (Vec<u8>, Option<Position>) {
    let mut cleaner = Cleaner::new();
    let mut out = Vec::new();
    for part in bytes.chunks(chunk) {
        out.extend(cleaner.feed(part).unwrap());
    }
    (out, cleaner.position())
}

#[test]
fn fil_003_the_sent_copy_has_no_gps_part_and_the_position_was_read_first() {
    let photo = photo();

    let (copy, position) = cleaned(&photo, photo.len());

    let position = position.unwrap();
    assert!((position.latitude - 52.5).abs() < 1e-9);
    assert!((position.longitude + 4.25).abs() < 1e-9);
    assert_eq!(cleaned(&copy, copy.len()).1, None);
    assert_eq!(
        copy.len(),
        photo.len() - XMP.len() - 4,
        "only the XMP segment goes"
    );
    assert!(copy.ends_with(IMAGE), "the image data is sent as it is");
    assert!(!copy.windows(11).any(|w| w == b"GPSLatitude"));
}

#[test]
fn fil_005_the_copy_is_the_same_in_any_size_of_parts() {
    let photo = photo();
    let whole = cleaned(&photo, photo.len());

    for chunk in [1, 2, 3, 7, 64] {
        assert_eq!(cleaned(&photo, chunk), whole, "{chunk}");
    }
}

#[test]
fn fil_003_a_file_that_is_not_a_jpeg_is_not_taken_for_one() {
    assert!(is_jpeg(&photo()));
    assert!(!is_jpeg(b"%PDF-1.7"));
    assert!(Cleaner::new().feed(b"%PDF-1.7").is_err());
}

proptest! {
    #[test]
    fn fil_003_the_walk_never_panics_on_any_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        let mut cleaner = Cleaner::new();
        let _ = cleaner.feed(&[&[0xFF, 0xD8][..], &bytes].concat());
    }

    #[test]
    fn fil_003_no_damage_to_the_exif_block_leaves_a_position_in_the_copy(
        changes in proptest::collection::vec((0_usize..128, any::<u8>()), 0..8),
        cut in 0_usize..140,
    ) {
        let mut block = tiff();
        for (at, byte) in changes {
            block[at] = byte;
        }
        block.truncate(cut.max(8));
        let photo = [
            vec![0xFF, 0xD8],
            segment(0xE1, &[b"Exif\0\0".as_slice(), &block].concat()),
            vec![0xFF, 0xDA],
        ]
        .concat();
        let (copy, _) = cleaned(&photo, photo.len());
        prop_assert_eq!(cleaned(&copy, copy.len()).1, None);
    }
}
