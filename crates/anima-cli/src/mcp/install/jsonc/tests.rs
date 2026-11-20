use super::{JsoncError, member, remove, set};

const ENTRY: &str =
    "{\n  \"command\": \"/usr/local/bin/anima\",\n  \"args\": [\n    \"mcp\"\n  ]\n}";
const PATH: &[&str] = &["mcpServers"];

/// Setting, then removing, gives the file back byte for byte.
fn round_trip(before: &str) -> String {
    let after = set(before, PATH, "anima", ENTRY).expect("set");
    assert_eq!(
        member(&after, PATH, "anima").expect("read"),
        Some(&*super::indented(
            ENTRY,
            super::indent_of(&after, after.rfind("\"anima\"").expect("member"))
        ))
    );
    assert_eq!(
        remove(&after, PATH, "anima").expect("remove").as_deref(),
        Some(before)
    );
    after
}

#[test]
fn mcp_030_a_member_is_added_after_the_last_one_and_every_other_byte_stays() {
    let before = "// my servers\n{\n\t\"theme\": \"dark\", /* keep */\n\t\"mcpServers\": {\n\t\t\"github\": { \"command\": \"gh-mcp\" } // mine\n\t}\n}\n";
    let after = round_trip(before);
    assert!(after.starts_with("// my servers\n{\n\t\"theme\": \"dark\", /* keep */\n\t\"mcpServers\": {\n\t\t\"github\": { \"command\": \"gh-mcp\" },\n\t\t\"anima\": {\n\t\t  \"command\""));
    assert!(after.ends_with(" // mine\n\t}\n}\n"));
}

#[test]
fn mcp_030_a_file_that_ends_its_members_with_a_comma_keeps_that_form() {
    let before = "{\n  \"mcpServers\": {\n    \"a\": {},\n    \"b\": {},\n  },\n}\n";
    let after = round_trip(before);
    assert!(after.contains("\"b\": {},\n    \"anima\": {"));
    assert!(after.contains("]\n    },\n  },\n}\n"));
}

#[test]
fn mcp_030_an_empty_object_takes_the_member_and_gives_it_back() {
    assert_eq!(
        set("{\n  \"mcpServers\": {}\n}\n", PATH, "anima", "1").expect("set"),
        "{\n  \"mcpServers\": {\n    \"anima\": 1\n  }\n}\n"
    );
    round_trip("{\n  \"mcpServers\": {}\n}\n");
}

#[test]
fn mcp_030_a_place_that_is_not_there_is_made_and_an_empty_file_is_an_object() {
    assert_eq!(
        set("{\n  \"theme\": \"dark\"\n}\n", PATH, "anima", "1").expect("set"),
        "{\n  \"theme\": \"dark\",\n  \"mcpServers\": {\n    \"anima\": 1\n  }\n}\n"
    );
    assert_eq!(
        set("", &["mcp", "servers"], "anima", "1").expect("set"),
        "{\n  \"mcp\": {\n    \"servers\": {\n      \"anima\": 1\n    }\n  }\n}\n"
    );
    assert_eq!(remove("", PATH, "anima"), Ok(None));
    assert_eq!(member("  \n", PATH, "anima"), Ok(None));
}

#[test]
fn mcp_030_an_entry_that_is_there_is_replaced_in_its_place() {
    let before = "{\"mcpServers\":{\"anima\":{\"command\":\"old\"},\"z\":1}}";
    assert_eq!(
        set(before, PATH, "anima", "2").expect("set"),
        "{\"mcpServers\":{\"anima\":2,\"z\":1}}"
    );
    assert_eq!(
        remove(before, PATH, "anima").expect("remove").as_deref(),
        Some("{\"mcpServers\":{\"z\":1}}")
    );
    assert_eq!(
        remove("{\"mcpServers\":{\"z\":1}}", PATH, "anima"),
        Ok(None)
    );
}

#[test]
fn mcp_030_a_key_inside_a_string_or_a_comment_is_not_the_member() {
    let before = "{\n  // \"mcpServers\": {\"anima\": 0}\n  \"note\": \"\\\"mcpServers\\\": {\",\n  \"mcpServers\": {\n    \"x\": \"}\"\n  }\n}";
    assert_eq!(member(before, PATH, "anima"), Ok(None));
    round_trip(before);
}

#[test]
fn mcp_034_a_file_that_is_not_an_object_is_refused_and_not_changed() {
    assert_eq!(set("[1]", PATH, "anima", "1"), Err(JsoncError::NotAnObject));
    assert_eq!(
        set("{\"mcpServers\": []}", PATH, "anima", "1"),
        Err(JsoncError::PlaceNotAnObject)
    );
    assert_eq!(
        set("{\"a\": 1} trailing", PATH, "anima", "1"),
        Err(JsoncError::Malformed)
    );
    assert_eq!(
        set("{\"a\": ", PATH, "anima", "1"),
        Err(JsoncError::Malformed)
    );
    assert_eq!(
        set("{ /* open", PATH, "anima", "1"),
        Err(JsoncError::Malformed)
    );
    let deep = format!("{{\"a\":{}1{}}}", "[".repeat(200), "]".repeat(200));
    assert_eq!(set(&deep, PATH, "anima", "1"), Err(JsoncError::TooDeep));
}
