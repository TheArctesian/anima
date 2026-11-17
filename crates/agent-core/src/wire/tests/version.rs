//! The agent version of a batch (K8 Section 4, R-122): optional, so an older agent's batch is
//! taken as it was, and held to the version rule when it is present.

use super::*;

fn with_version(version: &str) -> impl FnOnce(&mut Batch) {
    let version = version.to_owned();
    move |batch: &mut Batch| batch.agent_version = Some(version)
}

#[test]
fn r_122_a_batch_without_a_version_is_taken_with_none() {
    assert!(
        !BATCH.contains("agentVersion"),
        "the K8 example is an older agent's"
    );
    assert_eq!(checked(|_| {}).unwrap().agent_version, None);
}

#[test]
fn r_122_the_version_of_a_batch_reaches_the_hub_as_sent() {
    for version in ["0.1.0", "12.0.3", "1.2.3-rc.1+build.5"] {
        let accepted = checked(with_version(version)).unwrap();
        assert_eq!(accepted.agent_version.as_deref(), Some(version));
    }
    let longest = format!("1{}", ".0".repeat(31) + "0");
    assert_eq!(longest.len(), 64);
    assert_accepted("64 bytes", with_version(&longest));
}

#[test]
fn r_122_a_version_that_breaks_the_rule_is_malformed() {
    let too_long = format!("1{}", "0".repeat(64));
    let cases = [
        ("empty", ""),
        ("no leading digit", "v0.1.0"),
        ("a space", "0.1.0 beta"),
        ("a control character", "0.1.0\n"),
        ("a slash", "0.1.0/x"),
        ("not ASCII", "0.1.0é"),
        ("65 bytes", too_long.as_str()),
    ];
    for (case, version) in cases {
        assert_malformed(case, with_version(version));
    }
}

#[test]
fn r_122_the_agent_signs_its_batch_with_its_version_in_camel_case() {
    let batch = Batch::new(device(), [], Announcements::default()).from_agent("0.3.1");
    let written = serde_json::to_value(&batch).unwrap();
    assert_eq!(written["agentVersion"], "0.3.1");
    let unsigned = serde_json::to_value(Batch::new(device(), [], Announcements::default())).unwrap();
    assert!(
        unsigned.get("agentVersion").is_none(),
        "no member when unsigned"
    );
}
