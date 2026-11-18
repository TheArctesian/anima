//! The open claims of a hosted hub (hosted-mode.md HMD-037, R-129). The cloud gets one claim
//! with each device invite it asks for; the device that joined with that invite trades the
//! claim for its own token. The hub keeps only the hash of a claim, never the claim (LOG-007),
//! and a claim works once, until its invite expires.

use std::{
    collections::HashMap,
    sync::{Mutex, PoisonError},
};

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

/// Random bytes in a claim: 256 bits, so no throttle is needed to stop a guess.
pub(crate) const CLAIM_BYTES: usize = 32;

/// The hash of a claim, as the book keeps it.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ClaimHash([u8; 32]);

impl ClaimHash {
    pub(crate) fn of(claim: &str) -> Self {
        Self(Sha256::digest(claim.as_bytes()).into())
    }
}

/// What a claim admits: the link device with this name, until the invite expires.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OpenClaim {
    pub device_name: String,
    pub expires: DateTime<Utc>,
}

/// The open claims, by hash. A hub restart drops them: the cloud then asks again.
#[derive(Default)]
pub struct Claims(Mutex<HashMap<ClaimHash, OpenClaim>>);

impl Claims {
    /// Keeps a claim. The expired ones leave the book at the same time, and so does an open
    /// claim of the same device: one device has one open claim, the newest.
    pub(crate) fn open(&self, hash: ClaimHash, claim: OpenClaim, now: DateTime<Utc>) {
        let mut book = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        book.retain(|_, open| now < open.expires && open.device_name != claim.device_name);
        book.insert(hash, claim);
    }

    /// Ends the claim and returns what it admitted, or `None` for a claim that is used,
    /// expired or wrong. Only the first call for a claim succeeds.
    pub(crate) fn use_claim(&self, hash: ClaimHash, now: DateTime<Utc>) -> Option<OpenClaim> {
        let mut book = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        book.remove(&hash).filter(|open| now < open.expires)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;

    use super::*;

    fn at(minutes: i64) -> DateTime<Utc> {
        DateTime::UNIX_EPOCH + TimeDelta::minutes(minutes)
    }

    fn claim(name: &str, expires: i64) -> OpenClaim {
        OpenClaim {
            device_name: name.to_owned(),
            expires: at(expires),
        }
    }

    #[test]
    fn r_129_a_claim_works_once_and_only_until_its_invite_expires() {
        let claims = Claims::default();
        claims.open(ClaimHash::of("first"), claim("Studio", 15), at(0));
        assert_eq!(claims.use_claim(ClaimHash::of("wrong"), at(1)), None);
        assert_eq!(
            claims.use_claim(ClaimHash::of("first"), at(1)),
            Some(claim("Studio", 15))
        );
        assert_eq!(claims.use_claim(ClaimHash::of("first"), at(2)), None);
        claims.open(ClaimHash::of("late"), claim("Laptop", 15), at(0));
        assert_eq!(claims.use_claim(ClaimHash::of("late"), at(15)), None);
    }

    #[test]
    fn a_new_claim_of_a_device_ends_its_open_claim() {
        let claims = Claims::default();
        claims.open(ClaimHash::of("first"), claim("Studio", 15), at(0));
        claims.open(ClaimHash::of("other"), claim("Laptop", 15), at(0));
        claims.open(ClaimHash::of("second"), claim("Studio", 20), at(5));
        assert_eq!(claims.use_claim(ClaimHash::of("first"), at(6)), None);
        assert_eq!(
            claims.use_claim(ClaimHash::of("other"), at(6)),
            Some(claim("Laptop", 15))
        );
        assert_eq!(
            claims.use_claim(ClaimHash::of("second"), at(6)),
            Some(claim("Studio", 20))
        );
    }

    #[test]
    fn expired_claims_leave_the_book_when_a_new_one_opens() {
        let claims = Claims::default();
        claims.open(ClaimHash::of("old"), claim("Studio", 15), at(0));
        claims.open(ClaimHash::of("new"), claim("Laptop", 40), at(20));
        let book = claims.0.lock().unwrap();
        assert_eq!(book.len(), 1);
    }
}
