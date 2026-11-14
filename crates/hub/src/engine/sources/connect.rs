//! Connecting a GitHub App installation, the one path that stores a source key: the key is
//! checked, proved with a mint against the source, then stored with the account, audited and
//! published (SRC-003, SRC-008, PRJ-018, K-E1 Section 10a). A bad key or an unreachable source
//! stores nothing. `connectGithubApp` and the connect flow both come here.

use chrono::{DateTime, Utc};
use ring::rand::SystemRandom;
use source_exchange::{Declaration, Minted};
use sqlx::PgConnection;
use uuid::Uuid;
use zeroize::Zeroizing;

use super::{
    Error, Scheduling,
    acquisition::{MintFailure, app_key::AppKey, mint, stored_key, transport::Transport},
    model::Trigger,
    run,
    store::{self, Connecting},
    trail::connected,
};
use crate::{
    engine::extensions,
    fleet::{
        credentials::{Vault, store_source_secret},
        events::{self, Trail},
    },
    ids::SourceAccountId,
};

/// The source an App key connects.
const GITHUB: &str = "github";

/// What to connect: the operator's input, or what the connect flow converted. The key is
/// wiped on drop.
pub struct Connection {
    pub label: String,
    pub app_id: String,
    pub installation_id: String,
    pub private_key: Zeroizing<String>,
}

/// The stored account, whether it is new, and whether a consumer of its source is on (then its
/// first run has started).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Established {
    pub id: SourceAccountId,
    pub created: bool,
    pub consumer_on: bool,
}

/// The longest label, in characters.
const MAX_LABEL: usize = 64;

/// The label without control or invisible characters, trimmed, 1 to 64 characters.
pub(super) fn label(text: &str) -> Option<String> {
    let cleaned = crate::text::clean(text, MAX_LABEL * 4);
    let trimmed = cleaned.trim();
    let count = trimmed.chars().count();
    (1..=MAX_LABEL).contains(&count).then(|| trimmed.to_owned())
}

/// A checked connection: its key ready to sign and its label clean.
struct Checked {
    label: String,
    key: AppKey,
}

/// Checks the input before anything is sent: `Label` for a bad label; `Key` for a bad App ID,
/// installation ID or key.
fn check(input: &Connection) -> Result<Checked, Error> {
    let label = label(&input.label).ok_or(Error::Label)?;
    let key = AppKey::read(
        &input.app_id,
        &input.installation_id,
        input.private_key.as_bytes(),
    )?;
    Ok(Checked { label, key })
}

/// Mints a token with `key` at the source: the proof that the source takes the key.
/// `KeyRefused` when the source refuses it; `Unreachable` when it cannot be reached, answers
/// with a server error or limits requests.
async fn prove(
    transport: &Transport,
    declaration: &Declaration,
    key: &AppKey,
    now: DateTime<Utc>,
) -> Result<Minted, Error> {
    mint(transport, declaration, key, (now, &SystemRandom::new()))
        .await
        .map_err(|failure| match failure {
            MintFailure::Refused => Error::KeyRefused,
            MintFailure::RateLimited(_)
            | MintFailure::Unreachable
            | MintFailure::Unusable
            | MintFailure::Unsigned => Error::Unreachable,
            MintFailure::NotAppKey => Error::UnknownSource,
        })
}

/// Stores the account and its sealed key in `conn`; the same App and installation again keeps
/// the account and replaces its key. Returns the account's id and whether it is new. A failed
/// write is `Database` or `Credentials`; the caller rolls back.
async fn store(
    conn: &mut PgConnection,
    vault: &Vault,
    (declaration, checked, input): (&Declaration, &Checked, &Connection),
    now: DateTime<Utc>,
) -> Result<(SourceAccountId, bool), Error> {
    let instance_key = checked.key.instance_key();
    let connecting = Connecting {
        id: SourceAccountId::from(Uuid::now_v7()),
        source: declaration.key,
        label: &checked.label,
        instance_key: &instance_key,
        now,
    };
    let (id, created) = store::connect(conn, &connecting).await?;
    let secret = stored_key::to_secret(
        &input.app_id,
        checked.key.installation_id(),
        &input.private_key,
    );
    let credential = store_source_secret(conn, vault, id, secret).await?;
    store::set_credential(conn, id, credential).await?;
    Ok((id, created))
}

/// Checks, proves and stores `input`, writes the audit entry and the event of `trail` in the
/// same transaction, publishes the change, and starts the first run when a consumer of the
/// source is on. The proving token is kept only for that run; otherwise it is dropped and the
/// first run mints its own (PRJ-018).
///
/// # Errors
/// `Label` or `Key` for bad input, before anything is sent; `KeyRefused` or `Unreachable` from
/// the proof; `UnknownSource` without a GitHub declaration; `Database`, `Credentials` or
/// `Extensions` when the hub's own store fails. Nothing is stored unless it returns `Ok`.
pub async fn establish(
    s: &Scheduling,
    trail: Trail<'_>,
    input: Connection,
) -> Result<Established, Error> {
    let checked = check(&input)?;
    let declaration = *s.registry.declaration(GITHUB).ok_or(Error::UnknownSource)?;
    let ticket = s.transport.tokens().ticket();
    let minted = prove(&s.transport, &declaration, &checked.key, s.clock.now()).await?;
    let mut tx = s.pool.begin().await?;
    let (id, created) = store(
        &mut tx,
        &s.vault,
        (&declaration, &checked, &input),
        trail.now,
    )
    .await?;
    let instance_key = checked.key.instance_key();
    let named = (
        declaration.key,
        checked.label.as_str(),
        instance_key.as_str(),
    );
    trail
        .write(&mut tx, connected(id, named, created))
        .await
        .map_err(|events::Error::Database(error)| Error::Database(error))?;
    tx.commit().await?;
    drop(input);
    s.feed.publish(id);
    let on = extensions::on_keys(&s.pool).await?;
    let consumer_on = s.registry.consumer_on(declaration.key, &on);
    if consumer_on {
        drop(s.transport.tokens().keep(id, minted, ticket));
        first_run(s.clone(), id);
    } else {
        drop(minted);
    }
    Ok(Established {
        id,
        created,
        consumer_on,
    })
}

/// Starts the account's first run in its own task; the outcome reaches the account row and
/// `sourceAccountChanged`.
fn first_run(scheduling: Scheduling, id: SourceAccountId) {
    tokio::spawn(async move {
        if let Err(error) = run::run(&scheduling, id, Trigger::Connect).await {
            // Fixed words only (LOG-007).
            tracing::warn!(%error, "source sync could not be written");
        }
    });
}
