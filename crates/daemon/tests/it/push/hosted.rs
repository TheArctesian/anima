//! HM-T35: the daemon of a hosted hub, which reaches its hub on a socket, deletes a token that
//! the hub refuses and ends, so that `anima-local-hub` enrols it again at its next start.

use std::future::pending;
use std::time::Duration;

use anima_daemon::config::Config;
use anima_daemon::config::home::Home;
use anima_daemon::config::origin::Origin;
use anima_daemon::runtime::{self, RunError};
use anima_daemon::token_store::{Credential, TokenStore};
use daemon_core::wire::RefusalCode;

use crate::fake_hub::{FakeHub, Reply};
use crate::kit::{at, device, token, year};

/// Longer than one window of points and one push (10 seconds each), with room for a slow machine.
const LIMIT: Duration = Duration::from_secs(90);

/// A daemon folder enrolled at the hub behind `socket`, and a data folder of a hub to measure.
fn enrolled_beside(socket: &std::path::Path, dir: &std::path::Path) -> (Home, std::path::PathBuf) {
    let home = Home::at(dir.join("daemon"));
    let hub = Origin::parse_kept(&format!("unix:{}", socket.display())).expect("an origin");
    let config = Config {
        hub: hub.clone(),
        device: device(),
        device_name: "hub".to_owned(),
    };
    home.write_config(&config).expect("config");
    let credential = Credential {
        token: token(1),
        expires_at: at(0) + year(),
        hub,
    };
    TokenStore::open(&home)
        .save_new(&credential)
        .expect("token");
    let data = dir.join("data");
    std::fs::create_dir_all(&data).expect("a data folder");
    std::fs::write(data.join("database"), [0_u8; 512]).expect("a file of the hub");
    (home, data)
}

#[tokio::test]
async fn hmd_046_a_hosted_daemon_deletes_a_refused_token_and_ends() {
    for code in [RefusalCode::TokenRefused, RefusalCode::WrongDevice] {
        let dir = tempfile::tempdir().expect("a folder");
        let socket = dir.path().join("own.sock");
        let hub = FakeHub::behind_socket(&socket);
        hub.queue_push([Reply::refuse(code)]);
        let (home, data) = enrolled_beside(&socket, dir.path());
        let ended = tokio::time::timeout(LIMIT, runtime::run(&home, Some(data), pending())).await;
        assert!(
            matches!(ended, Ok(Err(RunError::TokenRefused))),
            "{code:?}: the run ends on the refusal"
        );
        assert!(
            TokenStore::open(&home).load().expect("a store").is_none(),
            "{code:?}: the refused token is gone, so the next start enrols"
        );
        assert_eq!(
            hub.seen("push").len(),
            1,
            "{code:?}: nothing after the refusal"
        );
    }
}
