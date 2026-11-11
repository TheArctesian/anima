//! Every K8 example body reads and writes back unchanged (K8 Sections 1, 3 to 6).

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::*;
use crate::wire::{
    DEVICE_NAME_CHARS, EnrolRequest, Enrolled, Refusal, RenewRequest, Renewed, is_device_name,
};

/// Reads `fixture` as `T`, writes it back, and compares the two as JSON values.
fn round_trip<T: Serialize + DeserializeOwned>(fixture: &str) -> T {
    let body: T = serde_json::from_str(fixture).unwrap();
    let written = serde_json::to_value(&body).unwrap();
    assert_eq!(written, serde_json::from_str::<Value>(fixture).unwrap());
    body
}

#[test]
fn k8_3_the_enrol_request_round_trips() {
    let body: EnrolRequest = round_trip(include_str!("k8/enrol-request.json"));
    assert_eq!(body.code.expose(), "anima_enrol_AAECAwQFBgcICQoLDA0ODw");
}

#[test]
fn k8_3_enrolled_round_trips() {
    let body: Enrolled = round_trip(include_str!("k8/enrolled.json"));
    assert_eq!(body.device, device());
    assert_eq!(body.device_name, "pop-os");
    assert_eq!(body.expires_at, at("2027-11-08T10:00:00Z"));
}

#[test]
fn k8_4_the_batch_round_trips() {
    let body: Batch = round_trip(BATCH);
    assert_eq!(body.points.len(), 2);
    assert_eq!(body.points[1].max, None);
}

#[test]
fn k8_5_the_renew_request_and_renewed_round_trip() {
    let _: RenewRequest = round_trip(include_str!("k8/renew-request.json"));
    let body: Renewed = round_trip(include_str!("k8/renewed.json"));
    assert!(body.token.expose().starts_with("anima_agent_"));
}

#[test]
fn k8_6_a_refusal_round_trips_with_each_code_and_its_status() {
    let body: Refusal = round_trip(include_str!("k8/refusal.json"));
    assert_eq!(body.error, RefusalCode::OutOfRange);
    let codes = [
        (RefusalCode::Malformed, "MALFORMED", 400, true),
        (RefusalCode::TokenRefused, "TOKEN_REFUSED", 401, false),
        (
            RefusalCode::EnrolmentRefused,
            "ENROLMENT_REFUSED",
            401,
            false,
        ),
        (RefusalCode::WrongDevice, "WRONG_DEVICE", 403, false),
        (RefusalCode::TooLarge, "TOO_LARGE", 413, true),
        (RefusalCode::OutOfRange, "OUT_OF_RANGE", 422, true),
        (RefusalCode::Unavailable, "UNAVAILABLE", 503, false),
    ];
    for (code, text, status, drops) in codes {
        let body = serde_json::to_string(&Refusal::from(code)).unwrap();
        assert_eq!(body, format!("{{\"error\":\"{text}\"}}"));
        assert_eq!(
            (code.as_str(), code.status(), code.drops_batch()),
            (text, status, drops)
        );
    }
}

#[test]
fn k8_1_unknown_members_are_ignored() {
    let body = r#"{ "token": "anima_agent_yMnKy8zNzs_Q0dLT1NXW19jZ2tvc3d7f4OHi4-Tl5uc",
        "expiresAt": "2027-11-08T10:00:00Z", "newer": { "member": [1] } }"#;
    assert!(serde_json::from_str::<Renewed>(body).is_ok());
    assert!(serde_json::from_str::<RenewRequest>(r#"{ "later": true }"#).is_ok());
    let batch = BATCH.replacen("\"device\"", "\"agentVersion\": \"0.2.0\", \"device\"", 1);
    assert_eq!(serde_json::from_str::<Batch>(&batch).unwrap(), example());
}

#[test]
fn k8_1_times_are_written_in_utc_with_z_and_whole_seconds() {
    let mut body: Renewed = serde_json::from_str(include_str!("k8/renewed.json")).unwrap();
    body.expires_at = at("2027-11-08T11:00:00.987654+01:00");
    let written = serde_json::to_value(&body).unwrap();
    assert_eq!(written["expiresAt"], "2027-11-08T10:00:00Z");
}

#[test]
fn log_007_a_body_with_a_bad_secret_is_refused_without_echoing_it() {
    let bad = "anima_agent_ZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXp7fH1-f4CBgoN";
    let body = include_str!("k8/enrolled.json").replace(
        "anima_agent_ZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXp7fH1-f4CBgoM",
        bad,
    );
    let error = serde_json::from_str::<Enrolled>(&body)
        .err()
        .unwrap()
        .to_string();
    assert!(!error.contains("ZGVm"), "{error}");
    let code = r#"{ "code": "anima_enrol_AAECAwQFBgcICQoLDA0ODx" }"#;
    let error = serde_json::from_str::<EnrolRequest>(code)
        .err()
        .unwrap()
        .to_string();
    assert!(!error.contains("AAEC"), "{error}");
}

#[test]
fn k8_1_a_time_is_read_only_in_the_form_k8_writes() {
    let renewed = |time: &str| {
        let body = include_str!("k8/renewed.json").replace("2027-11-08T10:00:00Z", time);
        serde_json::from_str::<Renewed>(&body).map(|body| body.expires_at)
    };
    assert_eq!(
        renewed("2027-11-08T10:00:00Z").unwrap(),
        at("2027-11-08T10:00:00Z")
    );
    assert_eq!(
        renewed("2028-02-29T23:59:59Z").unwrap(),
        at("2028-02-29T23:59:59Z")
    );
    for other in [
        "2027-11-08T10:00:00+00:00",
        "2027-11-08T11:00:00+01:00",
        "2027-11-08T10:00:00.0Z",
        "2027-11-08T10:00:00.5Z",
        "2027-11-08t10:00:00z",
        "2027-11-08T10:00:00z",
        "2027-11-08 10:00:00Z",
        "2027-11-08T10:00:00",
        "2027-11-08T10:00:00Z ",
        "+2027-11-08T10:00:00Z",
        "20271108T100000Z",
        "2027-11-08T10:00:60Z",
        "2027-13-08T10:00:00Z",
        "2027-02-29T10:00:00Z",
        "2027-11-08T24:00:00Z",
        "",
    ] {
        let error = renewed(other).err().map(|error| error.to_string());
        assert!(error.is_some(), "{other:?}");
    }
    let body = BATCH.replace("2026-11-08T10:00:20Z", "2026-11-08T10:00:20+00:00");
    assert!(
        serde_json::from_str::<Batch>(&body).is_err(),
        "a point's at too"
    );
}

#[test]
fn met_012_a_device_name_is_1_to_64_characters_of_any_width() {
    // 64 three-byte characters: 192 bytes, a name the hub's device rule takes.
    assert!(is_device_name(&"漢".repeat(DEVICE_NAME_CHARS)));
    assert!(is_device_name(&"🖥".repeat(DEVICE_NAME_CHARS)));
    assert!(is_device_name("a"));
    assert!(!is_device_name(""));
    assert!(!is_device_name(&"a".repeat(DEVICE_NAME_CHARS + 1)));
    assert!(!is_device_name(&"漢".repeat(DEVICE_NAME_CHARS + 1)));
}

#[test]
fn met_012_a_device_name_has_no_control_character() {
    for name in ["pop\u{1b}[2Kos", "pop-os\n", "pop\u{7}", "\u{9b}31m"] {
        assert!(!is_device_name(name), "{name:?}");
    }
}
