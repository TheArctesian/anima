//! One container item as the daemon gives it (K10 Sections 4, 5 and 8); the forms of the
//! members of each type are in `parameters`.

mod parameters;

use super::{APPLICATION, OF_ANIMA, item};
use crate::{
    Action, AdoptedAction, Pruned, Refusal, Work,
    request::{LARGEST_REQUEST, read},
};

pub(super) fn work(kind: &str, parameters: &str) -> Result<Work, Refusal> {
    read(&item(kind, parameters)).map(|request| request.work)
}

pub(super) fn apply(members: &str) -> Result<Work, Refusal> {
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
