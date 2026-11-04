//! Hub faults reach the process log by kind only, never with SQL or a token (K7, LOG-007,
//! API-010).

use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};

use anima_hub::events::process_log;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;

use crate::hub::{Hub, PASSWORD, USERNAME, start};

/// Collects everything the process log writes.
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .expect("capture lock")
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Capture {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().expect("capture lock").clone()).unwrap()
    }
}

/// A hub whose database cannot be reached: each query fails with a pool or I/O error.
fn unreachable_hub() -> Hub {
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(300))
        .connect_lazy("postgres://nobody@127.0.0.1:1/none")
        .unwrap();
    Hub::new(&pool, start())
}

#[tokio::test]
async fn api_010_an_unreachable_database_is_logged_by_kind_at_both_edges() {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = process_log::subscriber("error", move || writer.clone()).unwrap();
    let _default = tracing::subscriber::set_default(subscriber);
    let hub = unreachable_hub();

    let login = hub.login(USERNAME, PASSWORD).await;
    assert_eq!(
        login["errors"][0]["extensions"]["code"], "UNAVAILABLE",
        "{login}"
    );
    let resolver_lines = capture
        .text()
        .matches(r#""fault":"database_unreachable""#)
        .count();
    assert_eq!(
        resolver_lines,
        1,
        "through the resolver: {}",
        capture.text()
    );

    let token = URL_SAFE_NO_PAD.encode([9_u8; 32]);
    let me = hub
        .post(Some(&token), "{ me { username } }", json!({}))
        .await;
    assert_eq!(me["errors"][0]["extensions"]["code"], "UNAVAILABLE", "{me}");
    let text = capture.text();
    assert_eq!(
        text.matches(r#""fault":"database_unreachable""#).count(),
        2,
        "and through identify: {text}"
    );
    for leak in [
        token.as_str(),
        PASSWORD,
        "select",
        "insert",
        "access.",
        "127.0.0.1",
    ] {
        assert!(!text.contains(leak), "{leak:?} in {text}");
    }
}
