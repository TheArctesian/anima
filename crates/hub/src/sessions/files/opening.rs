//! `openFiles` (K1, K4 Section 9): a files ticket for a live SSH session of the same operator
//! and client address, whose terminal tunnel is open.

use sqlx::PgPool;

use super::tickets::FilesTicket;
use crate::{
    ids::SessionId,
    sessions::{
        Actor, Error, Live, Missing, Protocol, Ticket, store,
        tickets::{LIFETIME, TOKEN_BYTES, TicketToken},
    },
};

/// Issues a files ticket for `session`, redeemable once on `/files` from `by.client` within
/// 30 s of `by.at`. The session must be an SSH session `by.operator` started from that
/// client, not ended, with its terminal tunnel open; otherwise it is `NotFound`, so the
/// answer does not tell another operator's session from a missing one.
///
/// # Errors
/// `NotFound` as above; `Unavailable` when the hub is shutting down or too many files
/// tickets are pending; `Database` or `Random` when a dependency fails.
pub(super) async fn open_files(
    db: &PgPool,
    live: &Live,
    by: &Actor,
    session: SessionId,
) -> Result<Ticket, Error> {
    let Actor {
        operator,
        client,
        at: now,
    } = by;
    let (client, now) = (*client, *now);
    if live.is_shutting_down() {
        return Err(Error::Unavailable);
    }
    let missing = Error::NotFound(Missing::Session(session));
    let Some(row) = store::get(db, session).await? else {
        return Err(missing);
    };
    let device = match row.device {
        Some(device)
            if row.is_live()
                && row.protocol == Protocol::Ssh
                && row.operator == operator.id
                && row.client == client.0
                && live.has_tunnel(session) =>
        {
            device
        }
        _ => return Err(missing),
    };
    let token = random_token()?;
    let ticket = FilesTicket {
        operator: operator.clone(),
        client,
        session,
        device,
        expires_at: now + LIFETIME,
    };
    live.files()
        .issue(token.hash(), ticket, now)
        .map_err(|_| Error::Unavailable)?;
    Ok(Ticket::files(session, token, now + LIFETIME))
}

/// Random bytes from the operating system.
fn random_token() -> Result<TicketToken, Error> {
    let mut bytes = [0; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(Error::Random)?;
    Ok(TicketToken::from_random(bytes))
}
