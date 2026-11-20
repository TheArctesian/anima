//! Graceful shutdown and the second-signal emergency exit.
/// The first stop signal. A second one ends the process at once, without waiting for the
/// sessions or the drain.
pub(crate) async fn first_stop_signal() {
    stop_signal().await;
    tracing::info!("hub stopping");
    tokio::spawn(async {
        stop_signal().await;
        tracing::warn!("second stop signal; exiting now");
        std::process::exit(1);
    });
}

/// Resolves on Ctrl-C or SIGTERM. A signal that cannot be watched is logged and never
/// resolves, so the other one still stops the hub.
#[cfg(unix)]
async fn stop_signal() {
    let interrupt = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::warn!(%error, "cannot watch Ctrl-C");
            std::future::pending::<()>().await;
        }
    };
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut terminate) => {
                terminate.recv().await;
            }
            Err(error) => {
                tracing::warn!(%error, "cannot watch SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    tokio::select! {
        () = interrupt => {}
        () = terminate => {}
    }
}

/// The managed Windows parent can request graceful shutdown through its private pipe.
#[cfg(windows)]
async fn stop_signal() {
    match anima_windows::shutdown::Requested::new() {
        Ok(mut requested) => requested.wait().await,
        Err(error) => {
            tracing::warn!(%error, "cannot watch shutdown requests");
            std::future::pending::<()>().await;
        }
    }
}
