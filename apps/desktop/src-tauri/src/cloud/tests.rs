use reqwest::StatusCode;

use super::{
    CloudError, api, device::cloud_device, memory_session, name::device_name, phone, workos,
};
use crate::this_machine::OperatingSystem;

#[test]
fn hmd_043_a_device_name_is_lowercase_letters_digits_and_dashes_from_a_letter() {
    assert_eq!(
        device_name("Stephen's MacBook Pro"),
        "stephen-s-macbook-pro"
    );
    assert_eq!(device_name("  studio.local  "), "studio-local");
    assert_eq!(device_name("2024 Mini"), "computer-2024-mini");
    assert_eq!(device_name("Ünïcode—Box"), "n-code-box");
    assert_eq!(device_name(""), "computer");
    assert_eq!(device_name("---"), "computer");
    let long = device_name(&"a".repeat(80));
    assert_eq!(long.len(), 40);
    assert!(!device_name("a234567890123456789012345678901234567-9z").ends_with('-'));
}

#[test]
fn hmd_044_the_cloud_device_has_the_link_name_the_computer_name_and_the_os() {
    let device = cloud_device("Stephen's MacBook Pro", OperatingSystem::Macos);
    assert_eq!(
        serde_json::to_value(&device).ok(),
        Some(serde_json::json!({
            "name": "stephen-s-macbook-pro",
            "computer": "Stephen's MacBook Pro",
            "os": "MACOS",
        }))
    );
    let unnamed = serde_json::to_value(cloud_device("", OperatingSystem::Linux)).ok();
    assert_eq!(
        unnamed,
        Some(serde_json::json!({ "name": "computer", "computer": "", "os": "LINUX" }))
    );
}

#[test]
fn hmd_032_the_device_answer_gives_the_code_and_the_page_and_keeps_the_device_code() {
    let body = br#"{"device_code":"secret-device","user_code":"BCDF-GHJK",
        "verification_uri":"https://x/device","verification_uri_complete":"https://x/device?user_code=BCDF-GHJK",
        "expires_in":300,"interval":5}"#;
    let code = workos::device_code(StatusCode::OK, body).ok().unwrap();
    assert_eq!(code.user_code, "BCDF-GHJK");
    assert_eq!(code.page, "https://x/device?user_code=BCDF-GHJK");
    assert_eq!(code.interval, 5);
    assert_eq!(code.secret.as_str(), "secret-device");
}

#[test]
fn hmd_023_each_poll_answer_of_the_reference_has_its_meaning() {
    let poll = |status: u16, body: &str| {
        workos::polled(StatusCode::from_u16(status).unwrap(), body.as_bytes())
    };
    assert!(matches!(
        poll(400, r#"{"error":"authorization_pending"}"#),
        Ok(workos::Poll::Waiting)
    ));
    assert!(matches!(
        poll(400, r#"{"error":"slow_down"}"#),
        Ok(workos::Poll::SlowDown)
    ));
    assert_eq!(
        poll(400, r#"{"error":"access_denied"}"#).err(),
        Some(CloudError::Cancelled)
    );
    assert_eq!(
        poll(400, r#"{"error":"expired_token"}"#).err(),
        Some(CloudError::Expired)
    );
    assert_eq!(
        poll(400, r#"{"error":"invalid_grant"}"#).err(),
        Some(CloudError::Unauthenticated)
    );
    assert_eq!(
        poll(400, r#"{"error":"invalid_client"}"#).err(),
        Some(CloudError::Refused)
    );
    assert_eq!(poll(503, "").err(), Some(CloudError::Offline));
    let signed_in = poll(200, r#"{"access_token":"a","refresh_token":"r","user":{}}"#);
    assert!(
        matches!(signed_in, Ok(workos::Poll::SignedIn(tokens)) if tokens.refresh.as_str() == "r")
    );
}

#[test]
fn hmd_043_the_cloud_errors_of_the_contract() {
    let refusal = |status: u16, body: &str| {
        api::refusal(StatusCode::from_u16(status).unwrap(), body.as_bytes())
    };
    assert_eq!(
        refusal(401, r#"{"error":{"code":"unauthenticated"}}"#),
        CloudError::Unauthenticated
    );
    assert_eq!(
        refusal(409, r#"{"error":{"code":"not_ready"}}"#),
        CloudError::NotReady
    );
    assert_eq!(
        refusal(409, r#"{"error":{"code":"other"}}"#),
        CloudError::Refused
    );
    assert_eq!(refusal(403, ""), CloudError::Refused);
    assert_eq!(
        refusal(402, r#"{"error":{"code":"payment_failed"}}"#),
        CloudError::PaymentFailed
    );
    assert_eq!(
        refusal(400, r#"{"error":{"code":"bad_request"}}"#),
        CloudError::Refused
    );
    assert_eq!(
        refusal(503, r#"{"error":{"code":"not_configured"}}"#),
        CloudError::Offline
    );
    assert_eq!(refusal(502, ""), CloudError::Offline);
}

#[test]
fn log_007_an_error_serialises_as_its_code_only() {
    assert_eq!(
        serde_json::to_string(&CloudError::NotReady).unwrap(),
        r#"{"code":"not-ready"}"#
    );
}

/// A folder of this test only, removed when the test ends.
fn phone_folder(test: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("anima-phone-{test}-"))
        .tempdir()
        .unwrap()
}

#[test]
fn m3_4_a_phone_makes_its_device_name_one_time_and_keeps_it() {
    let folder = phone_folder("kept");
    let first = phone::device_name(Some(folder.path()), "iphone", || Some([0xab, 0x01, 0xff]));
    assert_eq!(first, "iphone-ab01ff");
    assert_eq!(device_name(&first), first, "the cloud takes the name");
    let second = phone::device_name(Some(folder.path()), "iphone", || Some([1, 2, 3]));
    assert_eq!(second, first, "a second sign-in sends the same name");
}

#[test]
fn m3_4_a_kept_name_that_the_cloud_does_not_take_is_made_again() {
    let folder = phone_folder("damaged");
    for damaged in ["", "Not A Name", "9lives", "a/../b"] {
        std::fs::write(folder.path().join("cloud-device"), damaged).unwrap();
        let name = phone::device_name(Some(folder.path()), "android", || Some([0, 0, 7]));
        assert_eq!(name, "android-000007", "{damaged:?}");
    }
}

#[test]
fn m3_4_a_phone_with_no_folder_or_no_random_bytes_still_has_a_name() {
    assert_eq!(
        phone::device_name(None, "android", || Some([9, 9, 9])),
        "android-090909"
    );
    let folder = phone_folder("no-random");
    assert_eq!(
        phone::device_name(Some(folder.path()), "iphone", || None),
        "iphone"
    );
    let missing = folder.path().join("not-there");
    assert_eq!(
        phone::device_name(Some(&missing), "iphone", || Some([1, 2, 3])),
        "iphone-010203"
    );
}

#[test]
fn m3_4_the_memory_session_keeps_one_refresh_token_for_each_client() {
    let (one, other) = ("client_test_memory_one", "client_test_memory_other");
    assert!(memory_session::read(one).unwrap().is_none());
    memory_session::write(one, "first").unwrap();
    memory_session::write(one, "rotated").unwrap();
    assert_eq!(
        memory_session::read(one).unwrap().unwrap().as_str(),
        "rotated"
    );
    assert!(memory_session::read(other).unwrap().is_none());
    memory_session::clear(one).unwrap();
    memory_session::clear(one).unwrap();
    assert!(memory_session::read(one).unwrap().is_none());
}
