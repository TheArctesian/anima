//! The wait of an old enrolment for the socket of its move (R-317): on Unix the
//! [`Watch`](super::old_origin::Watch) of the move; on Windows, where nothing moves, a wait
//! that never ends.

#[cfg(unix)]
pub type LinkWatch = super::old_origin::Watch;
#[cfg(windows)]
pub type LinkWatch = ();

/// Ends when the old enrolment of this daemon can move to the socket of the link.
#[cfg(unix)]
pub async fn passed(link: &LinkWatch) {
    link.passed().await;
}

/// Never ends: a daemon on Windows has no old enrolment to move.
#[cfg(windows)]
pub async fn passed(_: &LinkWatch) {
    std::future::pending::<()>().await;
}
