//! The pure parts of the files domain: the stream form of a piece (FIL-002), the checks of
//! names and media types, and the checks of extracted fields (VIS-004).

use base64::{Engine as _, engine::general_purpose::STANDARD};
use proptest::prelude::*;

use super::{
    model::{media_type, name},
    proposals::{Field, FieldType, checked_fields},
    stream::{Binding, PART, RECORD, StreamError, open_all, seal_all},
};
use crate::engine::data_keys::{DataKey, DataKeys};

const HASH: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
const OTHER_HASH: &str = "60303ae22b998861bce3b28f33eec1be758a213c86c93c076dbe9f558c11c752";

fn keys() -> (tempfile::TempDir, DataKey, DataKey) {
    let folder = tempfile::tempdir().unwrap();
    for (name, byte) in [("engine", 1_u8), ("wardrobe", 2)] {
        std::fs::write(
            folder.path().join(format!("{name}.key")),
            STANDARD.encode([byte; 32]),
        )
        .unwrap();
    }
    let keys = DataKeys::open(Some(folder.path()), b"none").unwrap();
    let (engine, wardrobe) = (keys.key("engine").unwrap(), keys.key("wardrobe").unwrap());
    (folder, engine, wardrobe)
}

fn plain(len: usize) -> Vec<u8> {
    (0..len).map(|i| u8::try_from(i % 251).unwrap()).collect()
}

#[test]
fn fil_002_a_piece_opens_with_its_key_and_hash_in_every_size() {
    let (_folder, key, _) = keys();
    let binding = Binding {
        key: &key,
        piece: HASH,
    };

    for len in [0, 1, PART - 1, PART, PART + 1, 3 * PART] {
        let piece = seal_all(binding, &plain(len));
        assert_eq!(open_all(binding, &piece).unwrap(), plain(len), "{len}");
    }
}

#[test]
fn fil_002_a_piece_does_not_open_with_a_different_key_or_hash() {
    let (_folder, engine, wardrobe) = keys();
    let piece = seal_all(
        Binding {
            key: &engine,
            piece: HASH,
        },
        &plain(PART + 5),
    );

    let other_key = open_all(
        Binding {
            key: &wardrobe,
            piece: HASH,
        },
        &piece,
    );
    let other_hash = open_all(
        Binding {
            key: &engine,
            piece: OTHER_HASH,
        },
        &piece,
    );

    assert_eq!(other_key, Err(StreamError::WrongKey));
    assert_eq!(other_hash, Err(StreamError::Damaged));
}

#[test]
fn fil_002_a_cut_piece_does_not_open_also_at_a_part_boundary() {
    let (_folder, key, _) = keys();
    let binding = Binding {
        key: &key,
        piece: HASH,
    };
    let piece = seal_all(binding, &plain(2 * PART + 10));
    let header = binding.header().len();

    for cut in [
        header,
        header + RECORD,
        header + 2 * RECORD,
        piece.len() - 1,
    ] {
        assert!(open_all(binding, &piece[..cut]).is_err(), "{cut}");
    }
}

#[test]
fn fil_002_parts_that_change_place_do_not_open() {
    let (_folder, key, _) = keys();
    let binding = Binding {
        key: &key,
        piece: HASH,
    };
    let piece = seal_all(binding, &plain(3 * PART));
    let header = binding.header().len();
    let part = |n: usize| &piece[header + n * RECORD..header + (n + 1) * RECORD];

    let swapped = [&piece[..header], part(1), part(0), part(2)].concat();

    assert_eq!(open_all(binding, &swapped), Err(StreamError::Damaged));
}

#[test]
fn fil_002_a_part_of_another_piece_does_not_open_in_this_one() {
    let (_folder, key, _) = keys();
    let this = Binding {
        key: &key,
        piece: HASH,
    };
    let other = Binding {
        key: &key,
        piece: OTHER_HASH,
    };
    let mine = seal_all(this, &plain(2 * PART));
    let theirs = seal_all(other, &plain(2 * PART));
    let header = this.header().len();

    let mixed = [&mine[..header + RECORD], &theirs[header + RECORD..]].concat();

    assert_eq!(open_all(this, &mixed), Err(StreamError::Damaged));
}

proptest! {
    #[test]
    fn fil_002_no_change_of_one_byte_opens(at in 0_usize..400, byte in 1_u8..=255) {
        let (_folder, key, _) = keys();
        let binding = Binding { key: &key, piece: HASH };
        let mut piece = seal_all(binding, &plain(300));
        let at = at % piece.len();
        piece[at] ^= byte;
        prop_assert!(open_all(binding, &piece).is_err());
    }

    #[test]
    fn fil_002_any_bytes_are_refused_without_a_panic(bytes in proptest::collection::vec(any::<u8>(), 0..200)) {
        let (_folder, key, _) = keys();
        let binding = Binding { key: &key, piece: HASH };
        prop_assert!(open_all(binding, &bytes).is_err());
    }
}

#[test]
fn fil_001_names_and_media_types_are_checked() {
    assert_eq!(name("  photo.jpg "), Some("photo.jpg".to_owned()));
    assert_eq!(name(""), None);
    assert_eq!(name("a\nb"), None);
    assert_eq!(name(&"x".repeat(201)), None);
    assert_eq!(
        media_type(Some("Image/JPEG; q=1")),
        Some("image/jpeg".to_owned())
    );
    assert_eq!(
        media_type(None),
        Some("application/octet-stream".to_owned())
    );
    assert_eq!(media_type(Some("text/html\r\nX: y")), None);
    assert_eq!(media_type(Some("nothing")), None);
}

fn field(name: &str, kind: FieldType, value: &str, confidence: Option<f64>) -> Field {
    Field {
        name: name.to_owned(),
        kind,
        value: value.to_owned(),
        confidence,
    }
}

#[test]
fn vis_004_fields_are_typed_and_a_confidence_is_0_to_1_or_none() {
    let good = [
        field("brand", FieldType::Text, "Acme", Some(0.9)),
        field("size", FieldType::Number, "42.5", None),
        field("bought", FieldType::Date, "2026-11-18", Some(1.0)),
        field("worn", FieldType::Boolean, "false", Some(0.0)),
        field("place", FieldType::Reference, "place:1", None),
    ];
    assert!(checked_fields(&good).is_ok());

    let bad = [
        field("Brand", FieldType::Text, "x", None),
        field("size", FieldType::Number, "NaN", None),
        field("bought", FieldType::Date, "18/11/2026", None),
        field("worn", FieldType::Boolean, "yes", None),
        field("place", FieldType::Reference, "nowhere", None),
        field("score", FieldType::Number, "1", Some(1.5)),
        field("size", FieldType::Number, "1", None),
    ];
    let refused: Vec<String> = checked_fields(&bad)
        .unwrap_err()
        .into_iter()
        .map(|bad| bad.field)
        .collect();
    assert_eq!(
        refused,
        [
            "fields.0.name",
            "fields.1.value",
            "fields.2.value",
            "fields.3.value",
            "fields.4.value",
            "fields.5.confidence",
            "fields.6.name",
        ]
    );
}
