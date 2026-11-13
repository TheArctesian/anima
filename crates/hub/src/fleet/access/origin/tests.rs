//! The public origin's `Origin` header text and its WebSocket origin (D-3).

use super::PublicOrigin;

fn origin(text: &str) -> PublicOrigin {
    PublicOrigin::new(text.to_owned())
}

#[test]
fn d_3_the_websocket_origin_follows_the_scheme() {
    assert_eq!(
        origin("https://hub.anima.ing").websocket(),
        "wss://hub.anima.ing"
    );
    let local = origin("http://127.0.0.1:8443");
    assert_eq!(local.websocket(), "ws://127.0.0.1:8443");
    assert_eq!(local.as_str(), "http://127.0.0.1:8443");
    assert_eq!(origin("tauri://localhost").websocket(), "");
}

#[test]
fn d_3_only_http_text_with_a_host_is_an_origin_header() {
    let public = origin("https://hub.anima.ing");
    assert_eq!(public.origin_header(), Some("https://hub.anima.ing"));
    let local = origin("http://127.0.0.1:8443");
    assert_eq!(local.origin_header(), Some("http://127.0.0.1:8443"));
    for text in [
        "null",
        "",
        "https://",
        "http://",
        "tauri://localhost",
        "file://",
    ] {
        assert_eq!(origin(text).origin_header(), None, "{text:?}");
    }
}
