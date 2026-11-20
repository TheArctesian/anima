//! `anima event watch --json` on the real hub's subscription (K1 `eventAdded`): the event of a
//! sign-in made while it runs arrives as one JSON line, and SIGINT stops it.

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;
use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;
use sqlx::PgPool;
use tokio::io::{AsyncBufReadExt as _, BufReader};

use super::words;
use crate::kit::real_hub::{RealHub, USERNAME};

/// How long the whole watch may take.
const DEADLINE: Duration = Duration::from_secs(30);

/// How long one sign-in's event is waited for before another sign-in: the watch may not have
/// subscribed yet when the first one happens, and nothing outside the hub shows when it has.
const EACH_TRY: Duration = Duration::from_millis(250);

#[sqlx::test(migrations = "../hub/migrations")]
async fn cmd_020_event_watch_prints_whole_json_lines_until_it_stops(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let reader = hub.login_token().await;
    let mut command = hub
        .anima()
        .with("ANIMA_TOKEN", &reader)
        .command(env!("CARGO_BIN_EXE_anima"));
    let mut watch = command
        .args(words("event watch --json"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("the watch starts");
    let mut lines = BufReader::new(watch.stdout.take().expect("stdout")).lines();

    let event = tokio::time::timeout(DEADLINE, async {
        loop {
            hub.login_token().await;
            if let Ok(line) = tokio::time::timeout(EACH_TRY, lines.next_line()).await {
                return line.expect("readable").expect("a line before the end");
            }
        }
    })
    .await
    .expect("an event while the watch runs");
    let event: Value = serde_json::from_str(&event).expect("one JSON object per line");
    assert_eq!(event["source"], "ACCESS", "{event}");
    assert!(
        event["message"]
            .as_str()
            .unwrap_or_default()
            .contains(USERNAME),
        "{event}"
    );

    #[cfg(unix)]
    {
        let pid = watch.id().expect("running").to_string();
        let sent = tokio::process::Command::new("/bin/kill")
            .args(["-INT", &pid])
            .status()
            .await
            .expect("kill runs");
        assert!(sent.success());
        let status = tokio::time::timeout(DEADLINE, watch.wait())
            .await
            .expect("the watch stops on SIGINT")
            .expect("an exit status");
        let interrupted = status.signal() == Some(2) || matches!(status.code(), Some(0 | 130));
        assert!(interrupted, "{status:?}");
    }
    #[cfg(windows)]
    {
        watch.kill().await.expect("the watch stops");
    }
    while let Ok(Some(line)) = lines.next_line().await {
        let whole: Result<Value, _> = serde_json::from_str(&line);
        assert!(whole.is_ok(), "a cut line: {line}");
    }
}

#[cfg(windows)]
#[sqlx::test(migrations = "../hub/migrations")]
async fn cmd_020_windows_console_ctrl_c_stops_a_running_watch(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let reader = hub.login_token().await;
    let anima = hub.anima().with("ANIMA_TOKEN", &reader);
    let mut terminal = crate::kit::terminal::Terminal::start(&anima, &["event", "watch", "--json"]);
    tokio::time::timeout(DEADLINE, async {
        loop {
            hub.login_token().await;
            if tokio::time::timeout(EACH_TRY, terminal.expect("ACCESS"))
                .await
                .is_ok()
            {
                break;
            }
        }
    })
    .await
    .expect("the watch has subscribed and received an event");
    terminal.press("\x03").await;
    let (code, screen) = terminal.finish().await;
    // STATUS_CONTROL_C_EXIT is the Windows counterpart of termination by SIGINT.
    assert!(matches!(code, 0 | 130 | -1_073_741_510), "{code}: {screen}");
    assert!(
        !screen.contains(&reader),
        "the token must never reach the console"
    );
}
