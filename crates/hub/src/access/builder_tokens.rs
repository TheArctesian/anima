//! Builder tokens: access tokens of the `builder` kind and scope, issued to a computer the operator
//! enrols to build releases and revoked with it (BLD-021 to BLD-025, D-1).
//!
//! Only the token's SHA-256 is stored, as for every token (BLD-023, D-7). The caller passes its
//! connection, so the builder record and its audit entry commit with the token.

use chrono::{DateTime, Utc};
use sqlx::PgConnection;

use super::{
    Error, login, store,
    tokens::{BUILDER_LIFETIME, Scope, TOKEN_BYTES, Token, TokenKind},
};
use crate::ids::{OperatorId, TokenId};

/// A new builder token, shown once. No `Debug`, `Clone` or `Display`: it holds the token
/// (LOG-007). Wiped from memory on drop.
pub struct IssuedToken {
    pub id: TokenId,
    pub expires_at: DateTime<Utc>,
    token: Token,
}

impl IssuedToken {
    /// The token text to give the enrolling client once (BLD-021). Never log it.
    #[must_use]
    pub fn secret(&self) -> &str {
        self.token.expose()
    }
}

/// Issues a builder token (kind and scope `builder`) for `operator` that holds for 180 days from `now` (BLD-022).
///
/// # Errors
///
/// `Error::Random` if the random source fails; `Error::Database` if the token cannot be stored.
pub async fn issue_builder_token(
    conn: &mut PgConnection,
    operator: OperatorId,
    now: DateTime<Utc>,
) -> Result<IssuedToken, Error> {
    let token = Token::from_random(login::random::<TOKEN_BYTES>()?);
    let expires_at = now + BUILDER_LIFETIME;
    let new_token = store::NewToken {
        operator,
        hash: token.hash(),
        kind: TokenKind::Builder,
        scope: Scope::Builder,
        expires_at,
    };
    let id = store::insert_token(conn, &new_token, now).await?;
    Ok(IssuedToken {
        id: TokenId::from(id),
        expires_at,
        token,
    })
}

/// Deletes a builder token, so it stops working at once (BLD-025). Returns false if there is
/// no builder token with this id; another token is never deleted.
///
/// # Errors
///
/// `Error::Database` if the token cannot be deleted.
pub async fn revoke_builder_token(conn: &mut PgConnection, id: TokenId) -> Result<bool, Error> {
    Ok(store::delete_builder_token(conn, id.into()).await?)
}
