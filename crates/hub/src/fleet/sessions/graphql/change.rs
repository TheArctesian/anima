//! `SessionChange` as the GraphQL type K1 describes, and the stream of them from `Live`'s
//! feed (RA-034).

use async_graphql::{
    Object,
    futures_util::stream::{self, Stream},
};
use tokio::sync::broadcast::{Receiver, error::RecvError};

use crate::fleet::sessions::{Session, SessionChange};

/// A session's start or end, published once committed. A session ended before its start was
/// published (by `endSession` right after the commit, or by a shutdown racing it) shows as an
/// end with no start.
#[Object]
impl SessionChange {
    /// The session as the change left it: live after a start, ended after an end.
    async fn session(&self) -> &Session {
        &self.session
    }
}

/// The changes on `feed`. A subscriber that fell behind skips the changes it missed; the
/// stream ends when the feed is closed.
pub(super) fn session_changes(feed: Receiver<SessionChange>) -> impl Stream<Item = SessionChange> {
    stream::unfold(feed, |mut feed| async move {
        loop {
            match feed.recv().await {
                Ok(change) => return Some((change, feed)),
                Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => return None,
            }
        }
    })
}
