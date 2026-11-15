//! `anima-gateway listen <address:port> [--other-gateway <host:port>]` runs the gateway.
//! `anima-gateway session` is one worker; the listener starts it. Wiring only.

use std::process::ExitCode;

use anima_gateway::listening::{self, OTHER_GATEWAY, WORKER, Workers};
use anima_gateway::session::{self, Settings};
use tokio::net::TcpListener;

const USAGE: &str = "usage: anima-gateway listen <address:port> [--other-gateway <host:port>]";

/// The value of `--other-gateway` in `args`, if it is there.
fn other_gateway(args: &[String]) -> Option<String> {
    let at = args.iter().position(|arg| arg == OTHER_GATEWAY)?;
    args.get(at + 1).cloned()
}

async fn listen(address: &str, other_gateway: Option<String>) -> std::io::Result<()> {
    let listener = TcpListener::bind(address).await?;
    let workers = Workers {
        program: std::env::current_exe()?,
        other_gateway,
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
    let other = other_gateway(&args);
    let ran = match args.first().map(String::as_str) {
        Some("listen") => match args.get(1) {
            Some(address) => runtime.block_on(listen(address, other)),
            None => Err(std::io::Error::other(USAGE)),
        },
        Some(WORKER) => runtime.block_on(work(other)),
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
