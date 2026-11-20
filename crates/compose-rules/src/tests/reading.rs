//! The reader of YAML refuses each form that a second reader could read differently.

use super::{Rule, SOUND, checked, refusal, top};
use crate::{LARGEST_FILE, Node, Settings, check};

#[test]
fn host_028_a_sound_file_passes_and_keeps_the_order_of_its_services() {
    let document = checked(SOUND).unwrap().document;
    let services = document.get("services").and_then(Node::entries).unwrap();
    let names: Vec<&str> = services.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["app", "db"]);
    // A volume with no member is how a file names a volume.
    assert_eq!(
        document.get("volumes").unwrap().get("data"),
        Some(&Node::Null)
    );
}

#[test]
fn host_028_an_alias_an_anchor_a_tag_and_the_merge_key_are_refused() {
    for file in [
        "services:\n  app: &a\n    image: a:1\n  db: *a\n",
        "x: &anchor 1\nservices:\n  app:\n    image: a:1\n",
        "services:\n  app:\n    image: !!str a:1\n",
        "services:\n  app:\n    <<: {image: a:1}\n",
        "services: !!map\n  app:\n    image: a:1\n",
    ] {
        assert_eq!(refusal(file), top(Rule::YamlNotPermitted), "{file}");
    }
}

#[test]
fn host_028_a_second_document_a_key_two_times_and_a_key_that_is_no_text_are_refused() {
    for file in [
        "services:\n  app:\n    image: a:1\n---\nservices: {}\n",
        "services:\n  app:\n    image: a:1\n    image: b:1\n",
        "services:\n  app:\n    image: a:1\nservices:\n  app:\n    image: b:1\n",
        "services:\n  app:\n    [image]: a:1\n",
        "services:\n  app:\n    {a: b}: a:1\n",
        "not: [closed\n",
        "",
    ] {
        assert_eq!(refusal(file), top(Rule::YamlNotPermitted), "{file:?}");
    }
}

#[test]
fn host_028_a_file_that_is_too_deep_or_too_large_is_refused() {
    let deep = format!(
        "services:\n  app:\n    command: {}x{}\n",
        "[".repeat(40),
        "]".repeat(40)
    );
    assert_eq!(refusal(&deep), top(Rule::YamlNotPermitted));
    let large = format!(
        "services:\n  app:\n    image: a:1\n#{}\n",
        "x".repeat(LARGEST_FILE)
    );
    assert_eq!(refusal(&large), top(Rule::TooLarge));
    let environment = "A=".to_owned() + &"x".repeat(crate::LARGEST_ENVIRONMENT);
    assert_eq!(
        check(SOUND, &environment, &Settings::default()),
        Err(top(Rule::TooLarge))
    );
}

#[test]
fn log_007_a_refusal_holds_a_code_and_a_position_and_no_text_of_the_file() {
    let file = "services:\n  app:\n    image: a:1\n    privileged: hunter2-secret-value\n";
    let refused = refusal(file);
    for text in [refused.to_string(), format!("{refused:?}")] {
        assert!(
            !text.contains("hunter2") && !text.contains("privileged"),
            "{text}"
        );
    }
    assert_eq!(Rule::KeyNotPermitted.code(), "key_not_permitted");
}
