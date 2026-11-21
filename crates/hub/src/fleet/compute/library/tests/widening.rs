//! How the publisher of a base model joins the approved list, by the setting and the actor
//! (LIBM-003, R-NEW-modellibrary).

use crate::fleet::compute::library::publisher::{PublisherApproval, Widening, widening};

fn approved() -> Vec<String> {
    vec!["unsloth".to_owned(), "mlx-community".to_owned()]
}

#[test]
fn a_person_adds_the_base_model_publisher_in_the_base_model_mode() {
    let found = widening(
        PublisherApproval::BaseModel,
        true,
        Some("Qwen"),
        &approved(),
    );
    assert_eq!(found, Widening::Add("Qwen".to_owned()));
}

#[test]
fn an_api_key_or_an_agent_never_widens_the_list() {
    let found = widening(
        PublisherApproval::BaseModel,
        false,
        Some("Qwen"),
        &approved(),
    );
    assert_eq!(found, Widening::Withheld("Qwen".to_owned()));
}

#[test]
fn the_by_hand_mode_adds_nothing_for_anyone() {
    for person in [true, false] {
        let found = widening(PublisherApproval::ByHand, person, Some("Qwen"), &approved());
        assert_eq!(found, Widening::Nothing);
    }
}

#[test]
fn a_listed_or_missing_publisher_changes_nothing() {
    let mode = PublisherApproval::BaseModel;
    assert_eq!(
        widening(mode, false, Some("UNSLOTH"), &approved()),
        Widening::Nothing
    );
    assert_eq!(widening(mode, true, None, &approved()), Widening::Nothing);
}
