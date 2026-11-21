//! The `CAA` check before the hub asks for a certificate (Section 12.5): the hub reads the
//! records of the name and of each domain above it, and does not ask when the first set it finds
//! does not permit the authority (RFC 8659). It says which record stops it.

use super::{
    names::RouteName,
    proof::{Caa, LookupFailed, Resolver},
};

/// Why the hub does not ask for a certificate of a name.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CaaRefusal {
    /// The `issue` records of `domain` name other authorities only.
    #[error("the CAA records of {domain} do not permit {authority}")]
    NotPermitted { domain: String, authority: String },
    /// DNS did not answer: an authority does not issue then either.
    #[error("the CAA records cannot be read")]
    Unread,
}

/// Reads the `CAA` records from the name up and checks the first set that exists.
///
/// # Errors
/// [`CaaRefusal`] when the records stop `authority` or cannot be read.
pub async fn check(
    resolver: &dyn Resolver,
    name: &RouteName,
    authority: &str,
) -> Result<(), CaaRefusal> {
    for domain in name.with_parents() {
        let records = resolver
            .caa(domain)
            .await
            .map_err(|LookupFailed| CaaRefusal::Unread)?;
        if !records.is_empty() {
            return permits(domain, &records, authority);
        }
    }
    Ok(())
}

/// Whether the one relevant set `records` of `domain` permits `authority` to issue for a name
/// that is not a wildcard. A set with no `issue` record permits each authority.
///
/// # Errors
/// [`CaaRefusal::NotPermitted`] when no `issue` record names `authority`.
pub fn permits(domain: &str, records: &[Caa], authority: &str) -> Result<(), CaaRefusal> {
    let mut issue = records
        .iter()
        .filter(|record| record.tag.eq_ignore_ascii_case("issue"))
        .peekable();
    if issue.peek().is_none() {
        return Ok(());
    }
    let named = issue.any(|record| {
        let issuer = record.value.split(';').next().unwrap_or_default().trim();
        issuer.eq_ignore_ascii_case(authority)
    });
    if named {
        Ok(())
    } else {
        Err(CaaRefusal::NotPermitted {
            domain: domain.to_owned(),
            authority: authority.to_owned(),
        })
    }
}
