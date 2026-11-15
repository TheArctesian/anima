//! `anima-gateway listen <address:port> [--other-gateway <host:port>]` runs the gateway.
//! `anima-gateway session` is one worker; the listener starts it. Wiring only.

use std::process::ExitCode;

use anima_gateway::listening::{self, MOST_SESSIONS, OTHER_GATEWAY, WORKER, Workers};
use anima_gateway::session::{self, Settings};
use tokio::net::TcpListener;

const USAGE: &str = "usage: anima-gateway listen <address:port> [--other-gateway <host:port>]";

/// `rest` as the options of a command: nothing, or `--other-gateway` and its value.
fn other_gateway(rest: &[String]) -> std::io::Result<Option<String>> {
    match rest {
        [] => Ok(None),
        [option, value] if option == OTHER_GATEWAY => Ok(Some(value.clone())),
        _ => Err(std::io::Error::other(USAGE)),
    }
}

async fn listen(address: &str, other_gateway: Option<String>) -> std::io::Result<()> {
    let listener = TcpListener::bind(address).await?;
    let workers = Workers {
        program: std::env::current_exe()?,
        other_gateway,
        most_sessions: MOST_SESSIONS,
    };
    eprintln!("anima-gateway: listening on {}", listener.local_addr()?);
    listening::listen(listener, workers).await
}

async fn work(other_gateway: Option<String>) -> std::io::Result<()> {
    let hub = listening::hub_connection()?;
    let settings = Settings {
        other_gateway,
        ..Settings::default()
    };
    let connection_id = format!("${}", uuid::Uuid::now_v7());
    let ended = session::serve(hub, &connection_id, &settings).await;
    eprintln!("anima-gateway: session ended: {ended}");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Ok(runtime) = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    else {
        eprintln!("anima-gateway: the runtime did not start");
        return ExitCode::FAILURE;
    };
    let ran = match args.split_first() {
        Some((command, [address, rest @ ..])) if command == "listen" => {
            other_gateway(rest).and_then(|other| runtime.block_on(listen(address, other)))
        }
        Some((command, rest)) if command == WORKER => {
            other_gateway(rest).and_then(|other| runtime.block_on(work(other)))
        }
        _ => Err(std::io::Error::other(USAGE)),
    };
    match ran {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("anima-gateway: {error}");
            ExitCode::FAILURE
        }
    }
}
