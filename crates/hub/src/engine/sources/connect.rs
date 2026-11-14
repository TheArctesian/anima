//! Connecting a GitHub App installation: the key is checked, proved with a mint against the
//! source, then stored with the account in the caller's transaction (SRC-003, SRC-008,
//! PRJ-018). A bad key or an unreachable source stores nothing.

use chrono::{DateTime, Utc};
use ring::rand::SystemRandom;
use source_exchange::{Declaration, Minted};
use sqlx::PgConnection;
use uuid::Uuid;
use zeroize::Zeroizing;

use super::{
    Error,
    acquisition::{MintFailure, app_key::AppKey, mint, stored_key, transport::Transport},
    store::{self, Connecting},
};
use crate::{
    fleet::credentials::{Vault, store_source_secret},
    ids::SourceAccountId,
};

/// The operator's input, the key wiped on drop.
pub struct Connection {
    pub label: String,
    pub app_id: String,
    pub installation_id: String,
    pub private_key: Zeroizing<String>,
}

/// The longest label, in characters.
const MAX_LABEL: usize = 64;

/// The label without control or invisible characters, trimmed, 1 to 64 characters.
fn label(text: &str) -> Option<String> {
    let cleaned = crate::text::clean(text, MAX_LABEL * 4);
    let trimmed = cleaned.trim();
    let count = trimmed.chars().count();
    (1..=MAX_LABEL).contains(&count).then(|| trimmed.to_owned())
}

/// A checked connection: its key ready to sign and its label clean.
pub struct Checked {
    pub label: String,
    pub key: AppKey,
}

/// Checks the input before anything is sent.
///
/// # Errors
/// `Label` for a bad label; `Key` for a bad App ID, installation ID or key.
pub fn check(input: &Connection) -> Result<Checked, Error> {
    let label = label(&input.label).ok_or(Error::Label)?;
    let key = AppKey::read(
        &input.app_id,
        &input.installation_id,
        input.private_key.as_bytes(),
    )?;
    Ok(Checked { label, key })
}

/// Mints a token with `key` at the source: the proof that the source takes the key.
///
/// # Errors
/// `KeyRefused` when the source refuses it; `Unreachable` when it cannot be reached, answers
/// with a server error or limits requests.
pub async fn prove(
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
/// the account and replaces its key. Returns the account's id and whether it is new.
///
/// # Errors
/// `Database` or `Credentials` when a write fails; the caller rolls back.
pub async fn store(
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
