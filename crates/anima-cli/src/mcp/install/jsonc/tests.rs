use super::{JsoncError, member, remove, set};

const ENTRY: &str =
    "{\n  \"command\": \"/usr/local/bin/anima\",\n  \"args\": [\n    \"mcp\"\n  ]\n}";
const PATH: &[&str] = &["mcpServers"];

/// The entry as it is written at the indent of the member `anima` in `after`.
fn reindented(after: &str) -> String {
    let at = after.rfind("\"anima\"").expect("member");
    let line = after[..at].rfind('\n').map_or(0, |newline| newline + 1);
    ENTRY.replace('\n', &format!("\n{}", &after[line..at]))
}

/// Setting, then removing, gives the file back byte for byte.
fn round_trip(before: &str) -> String {
    let after = set(before, PATH, "anima", ENTRY).expect("set");
    assert_eq!(
        member(&after, PATH, "anima").expect("read"),
        Some(&*reindented(&after))
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

#[test]
fn mcp_030_removing_a_member_keeps_the_comments_and_the_members_beside_it() {
    let joined = "{\n  \"mcpServers\": { // mine\n    \"anima\": {\"command\":\"/a\"}, \"other\": {\"command\":\"/keep\"}\n  }\n}";
    let after = remove(joined, PATH, "anima")
        .expect("remove")
        .expect("there");
    assert_eq!(
        member(&after, PATH, "other"),
        Ok(Some("{\"command\":\"/keep\"}"))
    );
    assert!(after.contains("{ // mine\n"));
    let noted = "{\"mcpServers\":{\n  \"a\": 1, // about a\n  /* about anima */ \"anima\": 2\n}}";
    let after = remove(noted, PATH, "anima")
        .expect("remove")
        .expect("there");
    assert!(
        after.contains("// about a")
            && after.contains("/* about anima */")
            && member(&after, PATH, "a") == Ok(Some("1"))
    );
    let held = "{\"mcpServers\": { // put servers here\n}}";
    let with = set(held, PATH, "anima", "1").expect("set");
    assert!(with.contains("// put servers here") && member(&with, PATH, "anima") == Ok(Some("1")));
    assert!(
        remove(&with, PATH, "anima")
            .expect("remove")
            .expect("there")
            .contains("// put servers here")
    );
}

#[test]
fn mcp_030_a_file_with_windows_line_ends_keeps_them() {
    let before = "{\r\n  \"mcpServers\": {\r\n    \"a\": 1\r\n  }\r\n}\r\n";
    let after = set(before, PATH, "anima", ENTRY).expect("set");
    assert!(!after.replace("\r\n", "").contains('\n'), "{after:?}");
    assert_eq!(
        remove(&after, PATH, "anima").expect("remove").as_deref(),
        Some(before)
    );
}

#[test]
fn mcp_034_a_key_that_is_there_twice_and_a_missing_comma_are_refused() {
    assert_eq!(
        set(
            "{\"mcpServers\": {\"anima\": 1}, \"mcpServers\": {\"x\": 2}}",
            PATH,
            "anima",
            "1"
        ),
        Err(JsoncError::Malformed)
    );
    assert_eq!(
        member(
            "{\"mcpServers\": {\"anima\": 1, \"anima\": 2}}",
            PATH,
            "anima"
        ),
        Err(JsoncError::Malformed)
    );
    assert_eq!(
        remove("{\"mcpServers\": {\"anima\":1 \"b\":2}}", PATH, "anima"),
        Err(JsoncError::Malformed)
    );
}
