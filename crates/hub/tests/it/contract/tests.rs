use super::*;

#[tokio::test]
async fn con_002_the_hub_schema_and_k1_declare_the_same_types_with_the_same_members() {
    let k1 = k1();
    let hub = hub(&k1);
    let differences = differences(&hub, &k1);
    assert!(
        differences.is_empty(),
        "CON-002 differences:\n{}",
        differences.join("\n")
    );
    // R-026(4): the liveness probe is `GET /health`, not a field.
    assert!(!hub["Query"].members.contains_key("health"));
    assert!(
        hub["Query"].members.contains_key("me") && hub["Mutation"].members.contains_key("login")
    );
}

#[tokio::test]
async fn con_002_a_missing_member_a_changed_signature_and_a_stray_type_are_each_reported() {
    let k1 = k1();
    let mut hub = hub(&k1);
    hub.get_mut("Device").unwrap().members.remove("name");
    hub.get_mut("Site")
        .unwrap()
        .members
        .insert("extra".into(), "(): Int".into());
    hub.get_mut("Query")
        .unwrap()
        .members
        .insert("me".into(), "(): Int".into());
    hub.insert(
        "Stray".into(),
        Declaration {
            header: "enum".into(),
            ..Declaration::default()
        },
    );
    hub.remove("Mutation");
    let header = |hub: &mut Schema, name: &str, header: &str| {
        hub.get_mut(name).unwrap().header = header.into();
    };
    header(&mut hub, "SshCredentialInput", "input");
    header(&mut hub, "CheckConfig", "union = IcmpConfig | TcpConfig");

    let found = differences(&hub, &k1);
    for expected in [
        "Device.name: in K1, not in the hub",
        "Site.extra: in the hub, not in K1",
        "Stray: declared by the hub, not in K1",
        "Mutation: declared by K1, not in the hub",
    ] {
        assert!(
            found.iter().any(|d| d == expected),
            "{expected} in {found:?}"
        );
    }
    for (name, theirs) in [
        ("SshCredentialInput", "input @oneOf"),
        (
            "CheckConfig",
            "union = DnsConfig | GroupConfig | HttpConfig | IcmpConfig | PushConfig | TcpConfig | TlsConfig",
        ),
    ] {
        let prefix = format!("{name}: hub `");
        let suffix = format!("K1 `{theirs}`");
        assert!(
            found
                .iter()
                .any(|d| d.starts_with(&prefix) && d.ends_with(&suffix)),
            "{name} header in {found:?}"
        );
    }
    assert!(
        found
            .iter()
            .any(|d| d.starts_with("Query.me: hub `(): Int`")),
        "{found:?}"
    );
}

// The `@pending` rule (D-1, P2-002) on small synthetic schemas.

fn found(hub: &str, k1: &str) -> Vec<String> {
    differences(&schema(hub), &schema(k1))
}

/// Asserts that the hub and K1 differ nowhere.
fn agree(hub: &str, k1: &str) {
    let found = found(hub, k1);
    assert!(found.is_empty(), "{found:?}");
}

const PENDING_DIRECTIVE: &str = "directive @pending on OBJECT | FIELD_DEFINITION | ARGUMENT_DEFINITION | ENUM | ENUM_VALUE | UNION | INPUT_OBJECT | INPUT_FIELD_DEFINITION\n";

fn k1_with(body: &str) -> String {
    format!("{PENDING_DIRECTIVE}{body}")
}

#[test]
fn con_002_the_pending_directive_definition_is_declared_by_k1_only() {
    assert_eq!(
        found("type A { x: Int }", &k1_with("type A { x: Int }")),
        Vec::<String>::new()
    );
}

#[test]
fn con_002_a_pending_type_absent_from_the_hub_is_not_a_difference() {
    let k1 = k1_with("type A { x: Int } type B @pending { y: Int }");
    agree("type A { x: Int }", &k1);
}

#[test]
fn con_002_a_pending_type_the_hub_declares_is_reported() {
    let k1 = k1_with("type A { x: Int } type B @pending { y: Int }");
    assert_eq!(
        found("type A { x: Int } type B { y: Int }", &k1),
        ["B: declared by the hub, still @pending in K1"]
    );
}

#[test]
fn con_002_pending_enums_inputs_and_unions_follow_the_same_rule() {
    let k1 = k1_with(
        "enum E @pending { A } input I @pending { a: Int } union U @pending = X type X { a: Int }",
    );
    agree("type X { a: Int }", &k1);
    let hub = "enum E { A } input I { a: Int } union U = X type X { a: Int }";
    assert_eq!(
        found(hub, &k1),
        [
            "E: declared by the hub, still @pending in K1",
            "I: declared by the hub, still @pending in K1",
            "U: declared by the hub, still @pending in K1",
        ]
    );
}

#[test]
fn con_002_a_pending_field_absent_from_the_hub_is_not_a_difference() {
    let k1 = k1_with("type A { x: Int  y: Int @pending }");
    agree("type A { x: Int }", &k1);
}

#[test]
fn con_002_a_pending_field_the_hub_has_is_reported() {
    let k1 = k1_with("type A { x: Int  y: Int @pending }");
    assert_eq!(
        found("type A { x: Int  y: Int }", &k1),
        ["A.y: in the hub, still @pending in K1"]
    );
}

#[test]
fn con_002_a_pending_input_field_follows_the_field_rule() {
    let k1 = k1_with("input I { a: Int  b: Int @pending }");
    agree("input I { a: Int }", &k1);
    assert_eq!(
        found("input I { a: Int  b: Int }", &k1),
        ["I.b: in the hub, still @pending in K1"]
    );
}

#[test]
fn con_002_a_pending_enum_value_follows_the_field_rule() {
    let k1 = k1_with("enum E { SSH  RDP @pending }");
    agree("enum E { SSH }", &k1);
    assert_eq!(
        found("enum E { SSH  RDP }", &k1),
        ["E.RDP: in the hub, still @pending in K1"]
    );
}

#[test]
fn con_002_a_pending_union_member_the_hub_lacks_is_left_out_of_the_union_header() {
    let k1 = k1_with("union C = I | D  type I { a: Int }  type D @pending { b: Int }");
    agree("union C = I  type I { a: Int }", &k1);
}

#[test]
fn con_002_a_pending_union_member_the_hub_declares_must_be_in_the_hub_union() {
    let k1 = k1_with("union C = I | D  type I { a: Int }  type D @pending { b: Int }");
    let found = found("union C = I  type I { a: Int }  type D { b: Int }", &k1);
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(found[0], "D: declared by the hub, still @pending in K1");
    assert!(found[1].starts_with("C: hub `union = I`, K1 `union = D | I`"));
}

#[test]
fn con_002_a_pending_argument_absent_from_the_hub_is_left_out_of_the_signature() {
    let k1 = k1_with("type Q { f(a: Int, b: Int @pending): Int }");
    agree("type Q { f(a: Int): Int }", &k1);
}

#[test]
fn con_002_a_pending_argument_the_hub_has_is_reported() {
    let k1 = k1_with("type Q { f(a: Int, b: Int @pending): Int }");
    assert_eq!(
        found("type Q { f(a: Int, b: Int): Int }", &k1),
        ["Q.f(b): in the hub, still @pending in K1"]
    );
}

#[test]
fn con_002_a_change_beside_a_pending_argument_is_still_reported() {
    let k1 = k1_with("type Q { f(a: Int, b: Int @pending): Int }");
    let found = found("type Q { f(a: String): Int }", &k1);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("Q.f: hub `(a: String): Int`, K1 `(a: Int): Int`"));
}

#[test]
fn con_002_a_member_that_is_not_pending_and_absent_from_the_hub_is_still_reported() {
    let k1 = k1_with("type A { x: Int  y: Int }");
    assert_eq!(
        found("type A { x: Int }", &k1),
        ["A.y: in K1, not in the hub"]
    );
    assert_eq!(
        found(
            "type A { x: Int }",
            &k1_with("type A { x: Int } type B { y: Int }")
        ),
        ["B: declared by K1, not in the hub"]
    );
}
