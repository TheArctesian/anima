//! The three values that tie a browser to the attempt: the ticket, the binding cookie and the
//! state sent to GitHub (K-E1 Section 7a, PRJ-041). Pure: the random source is a parameter.
//!
//! The ticket and the cookie are 32 random bytes each; the two states are derived from the
//! cookie, one for each of GitHub's pages. Each is base64url text. The hub keeps only the
//! SHA-256 hash of each and
//! compares a presented value's hash in constant time. No value has `Debug`; the text wipes
//! itself on drop (LOG-007).

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::{hmac, rand::SecureRandom};
use sha2::{Digest as _, Sha256};
use subtle::ConstantTimeEq as _;
use zeroize::Zeroizing;

/// How many random bytes each value holds.
const BYTES: usize = 32;
/// The longest text a presented value may have: 32 bytes are 43 base64url characters. Longer
/// text is never hashed.
const MAX_PRESENTED: usize = 64;

/// The random source gave nothing. Fixed words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the random source failed")]
pub struct RandomFailed;

/// The SHA-256 hash of one value: what the attempt keeps.
#[derive(Clone, Copy)]
pub struct Hash([u8; 32]);

impl Hash {
    /// The hash of `text`.
    fn of(text: &str) -> Self {
        Self(Sha256::digest(text.as_bytes()).into())
    }

    /// Whether `presented` is the value of this hash; compared in constant time. Text longer
    /// than a value can be is never a match.
    #[must_use]
    pub fn matches(&self, presented: &str) -> bool {
        presented.len() <= MAX_PRESENTED && bool::from(Self::of(presented).0.ct_eq(&self.0))
    }
}

/// A new value: its text, which leaves the hub once, and its hash, which stays.
pub struct Minted {
    text: Zeroizing<String>,
    hash: Hash,
}

impl Minted {
    /// 32 bytes from `random`, as base64url text without padding.
    ///
    /// # Errors
    /// [`RandomFailed`] when the random source fails.
    pub fn new(random: &dyn SecureRandom) -> Result<Self, RandomFailed> {
        let mut bytes = Zeroizing::new([0_u8; BYTES]);
        random.fill(&mut *bytes).map_err(|_| RandomFailed)?;
        let text = Zeroizing::new(URL_SAFE_NO_PAD.encode(*bytes));
        let hash = Hash::of(&text);
        Ok(Self { text, hash })
    }

    /// The text: for the one URL, cookie or page it travels in, never for a log.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The state the bound browser with `cookie` carries through GitHub for `step`: HMAC-SHA-256
    /// of a fixed label, keyed with the cookie. One browser gets the same state for a step on
    /// every ask (a second form or a replayed return replaces nothing); the two steps get
    /// unrelated states, and neither reveals the cookie. The hub still keeps only hashes.
    #[must_use]
    pub fn state(cookie: &str, step: Step) -> Self {
        let label: &[u8] = match step {
            Step::Create => b"anima connect github: state S1",
            Step::Install => b"anima connect github: state S2",
        };
        let tag = hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, cookie.as_bytes()), label);
        let text = Zeroizing::new(URL_SAFE_NO_PAD.encode(tag.as_ref()));
        let hash = Hash::of(&text);
        Self { text, hash }
    }

    #[must_use]
    pub const fn hash(&self) -> Hash {
        self.hash
    }
}

/// Which of GitHub's two pages a state is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// "Create GitHub App": S1.
    Create,
    /// "Install": S2.
    Install,
}

/// Seven random bytes for the App's name.
///
/// # Errors
/// [`RandomFailed`] when the random source fails.
pub fn name_entropy(random: &dyn SecureRandom) -> Result<[u8; 7], RandomFailed> {
    let mut bytes = [0_u8; 7];
    random.fill(&mut bytes).map_err(|_| RandomFailed)?;
    Ok(bytes)
}
