//! Text an operator writes is bounded and free of NUL and control characters (PRJ-028,
//! PRJ-035, decisions item 11).

use proptest::prelude::*;

use super::long;
use crate::{
    InputName, InputValue, Inputs, Label, Labels, PartError, StatusDescription, Text, Title,
};

#[test]
fn prj_028_text_is_bounded_and_free_of_nul_and_control_characters() {
    assert!(Title::parse("Fix the build").is_ok());
    assert!(Title::parse(&long('t', 256)).is_ok());
    for text in ["", &long('t', 257), "a\nb", "a\0b", "a\u{1b}[31m"] {
        assert_eq!(Title::parse(text), Err(PartError::Title), "{text:?}");
    }
    assert!(Text::parse("Line one\n\n- [ ] a\ttask\r\n").is_ok());
    assert!(Text::parse(&long('é', 65_536)).is_ok());
    for text in ["", &long('x', 65_537), "a\0b", "a\u{1b}[31m", "a\u{7f}"] {
        assert_eq!(Text::parse(text), Err(PartError::Text), "{text:?}");
    }
    assert!(StatusDescription::parse("").is_ok());
    assert!(StatusDescription::parse(&long('d', 140)).is_ok());
    for text in [&long('d', 141), "a\nb", "a\0"] {
        assert_eq!(
            StatusDescription::parse(text),
            Err(PartError::StatusDescription),
            "{text:?}"
        );
    }
}

#[test]
fn prj_028_labels_are_bounded_in_length_and_number() {
    let label = Label::parse("good first issue").unwrap();
    for text in ["", &long('l', 51), "a\nb", "a\0"] {
        assert_eq!(Label::parse(text), Err(PartError::Label), "{text:?}");
    }
    assert!(Labels::new(vec![label.clone(); 20]).is_ok());
    assert_eq!(Labels::new(vec![label; 21]), Err(PartError::Label));
}

#[test]
fn prj_035_dispatch_inputs_have_the_limits_of_decision_eleven() {
    for text in ["env", "deploy_target-1", &long('n', 100)] {
        assert!(InputName::parse(text).is_ok(), "{text:?}");
    }
    for text in ["", "a b", "a.b", "a/b", "a\0", &long('n', 101)] {
        assert_eq!(
            InputName::parse(text),
            Err(PartError::InputName),
            "{text:?}"
        );
    }
    assert!(InputValue::parse("").is_ok());
    assert!(InputValue::parse("two\nlines").is_ok());
    for text in [&long('v', 1025), "a\0b", "a\u{1b}"] {
        assert_eq!(
            InputValue::parse(text),
            Err(PartError::InputValue),
            "{text:?}"
        );
    }
    let input = |n: usize| {
        (
            InputName::parse(&format!("{n:0>100}")).unwrap(),
            InputValue::parse(&long('v', 1024)).unwrap(),
        )
    };
    let full = Inputs::new((0..25).map(input).collect()).unwrap();
    let characters: usize = full
        .as_slice()
        .iter()
        .map(|(n, v)| n.as_str().len() + v.as_str().len())
        .sum();
    assert!(
        characters <= 65_535,
        "the largest inputs fit GitHub's total"
    );
    assert_eq!(
        Inputs::new((0..26).map(input).collect()),
        Err(PartError::Inputs)
    );
    assert_eq!(
        Inputs::new(vec![input(1), input(1)]),
        Err(PartError::Inputs)
    );
}

proptest! {
    #[test]
    fn prj_028_accepted_text_holds_no_nul(text in any::<String>()) {
        for accepted in [
            Text::parse(&text).map(|t| t.as_str().to_owned()),
            Title::parse(&text).map(|t| t.as_str().to_owned()),
            InputValue::parse(&text).map(|t| t.as_str().to_owned()),
        ].into_iter().flatten() {
            prop_assert!(!accepted.contains('\0'));
        }
    }
}
