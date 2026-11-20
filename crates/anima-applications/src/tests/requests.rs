//! One container item as the daemon gives it (K10 Sections 4, 5 and 6.4.6 to 6.4.11).

use super::{APPLICATION, OF_ANIMA, item};
use crate::{
    Action, AdoptedAction, Pruned, Refusal, Work,
    request::{LARGEST_REQUEST, read},
};

fn work(kind: &str, parameters: &str) -> Result<Work, Refusal> {
    read(&item(kind, parameters)).map(|request| request.work)
}

fn apply(members: &str) -> Result<Work, Refusal> {
    work(
        "application_apply",
        &format!(r#"{{"application":"{APPLICATION}",{members}}}"#),
    )
}

#[test]
fn k10_6_4_each_container_type_is_read_with_the_members_of_the_contract() {
    let action = work(
        "container_action",
        &format!(r#"{{"container":"{OF_ANIMA}","action":"restart"}}"#),
    );
    assert_eq!(
        action,
        Ok(Work::Container {
            container: OF_ANIMA.to_owned(),
            action: Action::Restart
        })
    );
    let Ok(Work::Apply(read)) = apply(&format!(
        r#""fileSha256":"{OF_ANIMA}","environmentSha256":null,"pull":true,"keep":1"#
    )) else {
        panic!("an apply is read");
    };
    assert!(read.pull && read.keep && read.environment_sha256.is_none());
    assert_eq!(
        work(
            "adopted_apply",
            r#"{"project":"tracks","action":"up","pull":true}"#
        ),
        Ok(Work::Adopted {
            project: "tracks".to_owned(),
            action: AdoptedAction::Up,
            pull: true
        })
    );
    let removal = format!(r#"{{"application":"{APPLICATION}","volumes":false,"images":true}}"#);
    assert!(matches!(work("application_remove", &removal), Ok(Work::Remove(read)) if read.images));
    let back = format!(r#"{{"application":"{APPLICATION}"}}"#);
    assert!(matches!(
        work("application_back", &back),
        Ok(Work::Back { .. })
    ));
    assert_eq!(
        work(
            "docker_prune",
            r#"{"what":["containers","images","networks"]}"#
        ),
        Ok(Work::Prune {
            what: vec![Pruned::Containers, Pruned::Images, Pruned::Networks]
        })
    );
    let follow = format!(r#"{{"container":"{OF_ANIMA}","tail":200}}"#);
    assert!(matches!(
        work("container_follow", &follow),
        Ok(Work::Follow { tail: 200, .. })
    ));
}

#[test]
fn k10_5_a_type_that_is_not_a_container_type_and_a_kind_other_than_host_are_refused() {
    for kind in ["build", "runner", "shell", "container_exec", ""] {
        assert_eq!(work(kind, "{}"), Err(Refusal::TypeUnknown), "{kind}");
    }
    let stop = item(
        "container_action",
        &format!(r#"{{"container":"{OF_ANIMA}","action":"stop"}}"#),
    );
    for kind in ["container", "native", "Host"] {
        let text = stop.replacen("\"kind\":\"host\"", &format!("\"kind\":\"{kind}\""), 1);
        assert_eq!(read(&text), Err(Refusal::KindNotPermitted), "{kind}");
    }
}

#[test]
fn k10_5_a_member_that_the_contract_does_not_name_is_refused_at_each_level() {
    let stop = item(
        "container_action",
        &format!(r#"{{"container":"{OF_ANIMA}","action":"stop"}}"#),
    );
    assert!(read(&stop).is_ok());
    for (old, new) in [
        (
            "\"attempt\":1,",
            "\"attempt\":1,\"command\":\"docker system prune\",",
        ),
        // An item of a container type has no `repository`, and its `project` is null.
        (
            "\"attempt\":1,",
            "\"attempt\":1,\"repository\":{\"forge\":\"github\",\"owner\":\"a\",\"name\":\"b\"},",
        ),
        (
            "\"project\":null",
            "\"project\":\"0192f000-0000-7000-8000-000000000009\"",
        ),
        (
            "\"until\":\"2026-11-20T10:01:30Z\"",
            "\"until\":\"2026-11-20T10:01:30Z\",\"more\":1",
        ),
        ("\"seconds\":120,", "\"seconds\":120,\"cpus\":0,"),
        (
            "\"action\":\"stop\"",
            "\"action\":\"stop\",\"signal\":\"KILL\"",
        ),
        (
            "\"action\":\"stop\"",
            "\"action\":\"stop\",\"action\":\"start\"",
        ),
        // The hub makes no second attempt of a container item.
        ("\"attempt\":1,", "\"attempt\":2,"),
        ("\"attempt\":1,", ""),
        ("\"seconds\":120,", "\"seconds\":0,"),
        // The maximum of `limits.seconds` of `container_action` is 120.
        ("\"seconds\":120,", "\"seconds\":121,"),
        ("\"seconds\":120,", "\"seconds\":\"120\","),
        ("\"idleSeconds\":0,", ""),
        (
            "\"id\":\"0192f6aa-0000-7000-8000-000000000002\"",
            "\"id\":\"../../etc\"",
        ),
        (
            "\"until\":\"2026-11-20T10:01:30Z\"",
            "\"until\":\"$(reboot)\"",
        ),
    ] {
        assert!(stop.contains(old), "{old}");
        let text = stop.replacen(old, new, 1);
        assert_eq!(read(&text), Err(Refusal::MemberUnknown), "{new}");
    }
    assert_eq!(read("[]"), Err(Refusal::MemberUnknown));
    assert_eq!(read(&format!("{stop}{stop}")), Err(Refusal::MemberUnknown));
    let long = stop.replacen(
        "\"stop\"",
        &format!("\"{}\"", "s".repeat(LARGEST_REQUEST)),
        1,
    );
    assert_eq!(read(&long), Err(Refusal::MemberUnknown));
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
fn k10_8_secrets_names_only_the_purposes_of_the_type() {
    let stop = item(
        "container_action",
        &format!(r#"{{"container":"{OF_ANIMA}","action":"stop"}}"#),
    );
    for secrets in ["\"compose_file\"", "\"jit_config\"", "\"repository_token\""] {
        let text = stop.replacen("\"secrets\":[]", &format!("\"secrets\":[{secrets}]"), 1);
        assert_eq!(read(&text), Err(Refusal::SecretUnknown), "{secrets}");
    }
    let members =
        format!(r#""fileSha256":"{OF_ANIMA}","environmentSha256":null,"pull":true,"keep":0"#);
    let install = item(
        "application_apply",
        &format!(r#"{{"application":"{APPLICATION}",{members}}}"#),
    );
    assert!(read(&install).is_ok());
    let unknown = install.replacen("\"compose_file\"", "\"compose_file\",\"project_secret\"", 1);
    assert_eq!(read(&unknown), Err(Refusal::SecretUnknown));
    // An environment file that the parameters do not name, and no Compose file.
    let more = install.replacen(
        "\"compose_file\"",
        "\"compose_file\",\"environment_file\"",
        1,
    );
    assert_eq!(read(&more), Err(Refusal::MemberUnknown));
    let none = install.replacen("\"compose_file\"", "", 1);
    assert_eq!(read(&none), Err(Refusal::MemberUnknown));
}

/// Each of `refused`, parameters of the type `kind`, is refused with `member_unknown`.
fn refuses(kind: &str, refused: &[&str]) {
    for parameters in refused {
        let read = work(kind, parameters);
        assert_eq!(read, Err(Refusal::MemberUnknown), "{kind} {parameters}");
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

#[test]
fn k10_5_no_refusal_holds_text_of_a_request() {
    let secret = "hunter2-value-of-the-request";
    let text = item(
        "container_action",
        &format!(r#"{{"container":"{secret}","action":"stop"}}"#),
    );
    let refusal = read(&text).expect_err("refused");
    assert_eq!(refusal.code(), "member_unknown");
    assert!(!refusal.to_string().contains(secret));
    assert!(!format!("{refusal:?}").contains(secret));
}
