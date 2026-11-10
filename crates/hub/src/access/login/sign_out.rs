//! Signing out: a `logout`, and the browser token a new sign-in replaces (SEC-002, D-3, LOG-005).

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};

use super::{Replaced, audit_entry};
use crate::{
    access::{ClientAddr, Error, Identity, store},
    events::{self, AuditAction},
};

/// Deletes the identity's token and audits the sign-out.
pub(in crate::access) async fn logout(
    db: &PgPool,
    identity: &Identity,
    client: ClientAddr,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let mut tx = db.begin().await?;
    if !store::delete_token(&mut tx, identity.token()).await? {
        return Err(Error::Unauthenticated);
    }
    let entry = audit_entry(identity.operator().clone(), AuditAction::Logout, client);
    events::audit(&mut tx, now, entry).await?;
    tx.commit().await?;
    Ok(())
}

/// Deletes the replaced browser token and audits it as a sign-out of its operator, as
/// `logout` does. A token already gone (a `logout` that committed first) is left alone.
pub(super) async fn retire(
    conn: &mut PgConnection,
    replaced: Replaced,
    client: ClientAddr,
    at: DateTime<Utc>,
) -> Result<(), Error> {
    if store::delete_token(conn, replaced.token).await? {
        let entry = audit_entry(replaced.operator, AuditAction::Logout, client);
        events::audit(conn, at, entry).await?;
    }
    Ok(())
}
