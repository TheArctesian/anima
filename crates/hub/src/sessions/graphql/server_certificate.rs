//! `ServerCertificate` as the GraphQL type K1 describes (SEC-007). No field returns it until
//! `acceptServerCertificate` and the connection test serve it.

use async_graphql::Object;
use chrono::{DateTime, Utc};

use crate::sessions::ServerCertificate;

#[Object]
impl ServerCertificate {
    /// SHA-256 of the leaf certificate's DER, as 32 lower-case hex pairs joined by colons.
    async fn fingerprint_sha256(&self) -> &str {
        &self.fingerprint_sha256
    }

    /// The certificate's subject, as the device chose it.
    async fn subject(&self) -> Option<&str> {
        self.subject.as_deref()
    }

    async fn not_after(&self) -> Option<DateTime<Utc>> {
        self.not_after
    }

    async fn first_seen_at(&self) -> DateTime<Utc> {
        self.first_seen_at
    }
}
