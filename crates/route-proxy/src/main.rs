//! `anima-route-proxy`: the reverse proxy of the public names of applications, a process apart
//! from the hub (task H21). It wires the settings, the hub socket and the listeners; it holds no
//! rule. With `ANIMA_ROUTE_PROXY_LISTEN` unset it says that it is off and ends.

use std::{process::ExitCode, sync::Arc, time::Duration};

use anima_route_proxy::{
    hub::HubSocket,
    serve::{Proxy, serve_plain, serve_tls},
    settings,
};
use tokio::net::TcpListener;

/// How often the proxy reads the table again and reports its counts.
const REFRESH: Duration = Duration::from_secs(60);

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt().json().init();
    let env = std::env::vars()
        .filter(|(name, _)| name.starts_with("ANIMA_"))
        .collect();
    let settings = match settings::from_env(&env) {
        Ok(Some(settings)) => settings,
        Ok(None) => {
            eprintln!("anima-route-proxy is off: ANIMA_ROUTE_PROXY_LISTEN is not set");
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("anima-route-proxy: {error}");
            return ExitCode::from(2);
        }
    };
    let Ok(listener) = TcpListener::bind(settings.listen).await else {
        eprintln!(
            "anima-route-proxy: port {} is in use by a different program, or not permitted",
            settings.listen.port()
        );
        return ExitCode::FAILURE;
    };
    let mut plain = None;
    if let Some(address) = settings.listen_plain {
        let Ok(listener) = TcpListener::bind(address).await else {
            eprintln!(
                "anima-route-proxy: port {} is in use by a different program, or not permitted",
                address.port()
            );
            return ExitCode::FAILURE;
        };
        plain = Some(listener);
    }
    let hub = HubSocket(settings.hub_socket.clone());
    let Ok(proxy) = Proxy::new(hub, &settings) else {
        return ExitCode::FAILURE;
    };
    let proxy = Arc::new(proxy);
    if let Some(plain) = plain {
        tokio::spawn(serve_plain(Arc::clone(&proxy), plain));
    }
    tokio::spawn(serve_tls(Arc::clone(&proxy), listener));
    loop {
        if let Err(error) = proxy.refresh().await {
            tracing::warn!(error = %error, "the table of the hub was not read");
        }
        proxy.report().await;
        tokio::time::sleep(REFRESH).await;
    }
}
