//! K8 Section 6: a refusal's body is exactly `{"error":"<CODE>"}` with the code's status, and
//! its event names the route, the address and the code, nothing from the request.

use std::net::{IpAddr, Ipv4Addr};

use agent_core::wire::RefusalCode;
use axum::http::{StatusCode, header};
use http_body_util::BodyExt as _;

use crate::{
    events::ClientAddr,
    metrics::ingest::refusals::{Refused, Route, answer, sentence, unrecorded_sentence},
};

#[tokio::test]
async fn k8_every_refusal_body_is_exactly_its_code() {
    let codes = [
        (RefusalCode::Malformed, StatusCode::BAD_REQUEST),
        (RefusalCode::TokenRefused, StatusCode::UNAUTHORIZED),
        (RefusalCode::EnrolmentRefused, StatusCode::UNAUTHORIZED),
        (RefusalCode::WrongDevice, StatusCode::FORBIDDEN),
        (RefusalCode::TooLarge, StatusCode::PAYLOAD_TOO_LARGE),
        (RefusalCode::OutOfRange, StatusCode::UNPROCESSABLE_ENTITY),
        (RefusalCode::Unavailable, StatusCode::SERVICE_UNAVAILABLE),
    ];
    for (code, status) in codes {
        let response = answer(code);
        assert_eq!(response.status(), status);
        let headers = response.headers();
        assert_eq!(headers[header::CONTENT_TYPE], "application/json");
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let expected = format!(r#"{{"error":"{}"}}"#, code.as_str());
        assert_eq!(body, expected.as_bytes());
    }
}

#[test]
fn k8_a_refusal_event_names_the_route_the_address_and_the_code() {
    let client = ClientAddr(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 9)));
    let refused = |route, code| Refused {
        route,
        code,
        client,
        device: None,
    };
    assert_eq!(
        sentence(&refused(Route::Enrol, RefusalCode::EnrolmentRefused)),
        "Refused an agent enrolment from 100.64.0.9: ENROLMENT_REFUSED."
    );
    assert_eq!(
        sentence(&refused(Route::Renew, RefusalCode::TokenRefused)),
        "Refused an agent token renewal from 100.64.0.9: TOKEN_REFUSED."
    );
    assert_eq!(
        unrecorded_sentence(3),
        "Left out 3 more agent refusals from other addresses in the last minute."
    );
}
