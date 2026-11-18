use reqwest::StatusCode;

use super::{CloudError, api, device::cloud_device, name::device_name, workos};
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
