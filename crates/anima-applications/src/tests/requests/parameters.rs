//! The forms of the `parameters` of each container type (K10 Sections 6.4.6 to 6.4.11).

use super::{apply, work};
use crate::{
    Refusal,
    request::read,
    tests::{APPLICATION, OF_ANIMA, item},
};

/// Each of `refused`, parameters of the type `kind`, is refused with `member_unknown`.
fn refuses(kind: &str, refused: &[&str]) {
    for parameters in refused {
        let read = work(kind, parameters);
        assert_eq!(read, Err(Refusal::MemberUnknown), "{kind} {parameters}");
    }
}

#[test]
fn k10_6_4_6_a_container_is_the_full_identifier_and_an_action_is_one_of_three() {
    let action = |container: &str, action: &str| {
        work(
            "container_action",
            &format!(r#"{{"container":"{container}","action":"{action}"}}"#),
        )
    };
    for sound in ["start", "stop", "restart"] {
        assert!(action(OF_ANIMA, sound).is_ok(), "{sound}");
    }
    for refused in ["kill", "pause", "remove", "rm", "Stop", ""] {
        assert_eq!(
            action(OF_ANIMA, refused),
            Err(Refusal::MemberUnknown),
            "{refused}"
        );
    }
    let upper = OF_ANIMA.to_uppercase();
    let long = format!("{OF_ANIMA}a");
    for refused in [
        "aaaaaaaaaaaa",
        "anima-app-1",
        "--all",
        upper.as_str(),
        long.as_str(),
        "",
    ] {
        assert_eq!(
            action(refused, "stop"),
            Err(Refusal::MemberUnknown),
            "{refused}"
        );
    }
    let number = work("container_action", r#"{"container":1,"action":"stop"}"#);
    assert_eq!(number, Err(Refusal::MemberUnknown));
}

#[test]
fn k10_6_4_6_the_members_of_an_apply_have_their_forms() {
    let sound = format!(
        r#""fileSha256":"{OF_ANIMA}","environmentSha256":"{OF_ANIMA}","pull":false,"keep":0"#
    );
    assert!(
        read(
            &item(
                "application_apply",
                &format!(r#"{{"application":"{APPLICATION}",{sound}}}"#)
            )
            .replacen(
                "\"compose_file\"",
                "\"compose_file\",\"environment_file\"",
                1
            )
        )
        .is_ok()
    );
    for (old, new) in [
        ("\"keep\":0", "\"keep\":2"),
        ("\"keep\":0", "\"keep\":true"),
        ("\"pull\":false", "\"pull\":0"),
        ("\"pull\":false,", ""),
        (
            "\"environmentSha256\":\"aaaa",
            "\"environmentSha256\":\"AAAA",
        ),
        ("\"fileSha256\":\"aaaa", "\"fileSha256\":\"aaa"),
        ("\"keep\":0", "\"keep\":0,\"profile\":\"full\""),
    ] {
        assert!(sound.contains(old), "{old}");
        assert_eq!(
            apply(&sound.replacen(old, new, 1)),
            Err(Refusal::MemberUnknown),
            "{new}"
        );
    }
    let members =
        format!(r#""fileSha256":"{OF_ANIMA}","environmentSha256":null,"pull":true,"keep":0"#);
    for application in [
        "0192F6B0-7C1E-7A3B-9D2F-4E5A6B7C8D9E",
        "0192f6b07c1e7a3b9d2f4e5a6b7c8d9e",
        "{0192f6b0-7c1e-7a3b-9d2f-4e5a6b7c8d9e}",
        "../other",
        "",
    ] {
        let parameters = format!(r#"{{"application":"{application}",{members}}}"#);
        assert_eq!(
            work("application_apply", &parameters),
            Err(Refusal::MemberUnknown),
            "{application}"
        );
    }
}

#[test]
fn k10_6_4_7_an_adopted_apply_has_no_down_and_pulls_only_with_up() {
    let long = format!(
        r#"{{"project":"{}","action":"up","pull":false}}"#,
        "a".repeat(64)
    );
    refuses(
        "adopted_apply",
        &[
            r#"{"project":"tracks","action":"down","pull":false}"#,
            r#"{"project":"tracks","action":"stop","pull":true}"#,
            r#"{"project":"tracks","action":"restart","pull":true}"#,
            r#"{"project":"Tracks","action":"up","pull":false}"#,
            r#"{"project":"-f","action":"up","pull":false}"#,
            r#"{"project":"a/b","action":"up","pull":false}"#,
            r#"{"project":"tracks","action":"up"}"#,
            &long,
        ],
    );
}

#[test]
fn k10_6_4_8_to_6_4_11_the_members_of_each_other_type_have_their_forms() {
    let no_images = format!(r#"{{"application":"{APPLICATION}","volumes":false}}"#);
    let text = format!(r#"{{"application":"{APPLICATION}","volumes":"no","images":false}}"#);
    refuses("application_remove", &[&no_images, &text]);
    let steps = format!(r#"{{"application":"{APPLICATION}","steps":2}}"#);
    refuses("application_back", &[&steps, "{}"]);
    // There is no `volumes` member of a prune.
    refuses(
        "docker_prune",
        &[
            r#"{"what":["volumes"]}"#,
            r#"{"what":["containers","containers"]}"#,
            r#"{"what":[]}"#,
            r#"{"what":"images"}"#,
        ],
    );
    // `tail` is 0 to 1,000.
    let follow = |tail: &str| format!(r#"{{"container":"{OF_ANIMA}","tail":{tail}}}"#);
    let none = format!(r#"{{"container":"{OF_ANIMA}"}}"#);
    refuses(
        "container_follow",
        &[&follow("1001"), &follow("-1"), &follow("\"all\""), &none],
    );
    assert!(work("container_follow", &follow("0")).is_ok());
    assert!(work("container_follow", &follow("1000")).is_ok());
}
