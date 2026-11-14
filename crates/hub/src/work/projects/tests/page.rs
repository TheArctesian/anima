//! The page cursor: any name and id come back as they went in; anything else is refused.

use proptest::prelude::*;
use uuid::Uuid;

use crate::{
    ids::ProjectId,
    work::projects::page::{After, decode_cursor, encode_cursor, page_size},
};

proptest! {
    #[test]
    fn api_006_a_cursor_gives_back_its_place(name in ".{0,80}", id in any::<u128>()) {
        let after = After { name, id: ProjectId::from(Uuid::from_u128(id)) };
        prop_assert_eq!(decode_cursor(&encode_cursor(&after)).unwrap(), after);
    }

    #[test]
    fn api_006_any_other_text_is_an_after_error_not_a_panic(text in ".{0,80}") {
        if let Err(error) = decode_cursor(&text) {
            prop_assert_eq!(error.field, "after");
        }
    }
}

#[test]
fn api_006_a_forged_cursor_is_refused() {
    for text in [
        "",
        "p1.",
        "p1.zz.x",
        "p1.6.0190",
        "n1.61",
        "p1.ff.not-a-uuid",
    ] {
        assert_eq!(decode_cursor(text).unwrap_err().field, "after", "{text}");
    }
}

#[test]
fn api_006_a_page_holds_1_to_200() {
    assert_eq!(page_size(1).unwrap(), 1);
    assert_eq!(page_size(200).unwrap(), 200);
    assert_eq!(page_size(0).unwrap_err().field, "first");
    assert_eq!(page_size(201).unwrap_err().field, "first");
}
