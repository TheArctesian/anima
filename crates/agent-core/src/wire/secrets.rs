//! The agent's two secrets: the enrolment code and the agent token (K8 Section 2, MET-012,
//! MET-013).
//!
//! Each is held as its text, wiped on drop, with the SHA-256 of its **bytes** (the 16 or 32
//! random bytes, not the text with its prefix). Only the canonical spelling is accepted, so a
//! value has one text and one hash. Neither type implements `Debug`, `Display`, `Clone`,
//! `Serialize` or `Deserialize` (LOG-007): the text leaves only through `expose`, for the one
//! body or header that carries it, and through the fields of the K8 bodies in this module's
//! parent.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize as _, Deserializer, Serializer, de::Error as _};
use sha2::{Digest as _, Sha256};
use zeroize::{Zeroize as _, Zeroizing};

const CODE_PREFIX: &str = "anima_enrol_";
const TOKEN_PREFIX: &str = "anima_agent_";

/// Random bytes in an enrolment code: 128 bits (MET-012).
pub const CODE_BYTES: usize = 16;

/// Random bytes in an agent token: 256 bits (MET-013, P3-004).
pub const TOKEN_BYTES: usize = 32;

/// A one-use enrolment code, `anima_enrol_` and 22 base64url characters (K8 Sections 2, 3).
pub struct EnrolmentCode(Secret);

/// An agent's bearer token, `anima_agent_` and 43 base64url characters (K8 Sections 2, 4, 5).
pub struct AgentToken(Secret);

/// The SHA-256 of an enrolment code's 16 bytes: the only form of a code the hub keeps.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CodeHash([u8; 32]);

/// The SHA-256 of an agent token's 32 bytes: the only form of a token the hub keeps.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TokenHash([u8; 32]);

impl EnrolmentCode {
    /// A new code made of `bytes` from the OS random source (R-092 D-1). Its copy of the bytes
    /// is wiped; pass the array straight from the source and keep no other.
    #[must_use]
    pub fn from_random(bytes: [u8; CODE_BYTES]) -> Self {
        Self(Secret::from_random(CODE_PREFIX, bytes))
    }

    /// The code `presented` spells, or `None` unless it is `anima_enrol_` followed by the
    /// canonical unpadded base64url of exactly 16 bytes.
    #[must_use]
    pub fn parse(presented: &str) -> Option<Self> {
        Secret::parse::<CODE_BYTES>(CODE_PREFIX, presented).map(Self)
    }

    /// The text, for the one place that carries it: shown once to the operator, or sent in the
    /// enrol request. Never log it (LOG-007).
    #[must_use]
    pub fn expose(&self) -> &str {
        self.0.expose()
    }

    #[must_use]
    pub const fn hash(&self) -> CodeHash {
        CodeHash(self.0.hash)
    }
}

impl AgentToken {
    /// A new token made of `bytes` from the OS random source (R-092 D-1). Its copy of the bytes
    /// is wiped; pass the array straight from the source and keep no other.
    #[must_use]
    pub fn from_random(bytes: [u8; TOKEN_BYTES]) -> Self {
        Self(Secret::from_random(TOKEN_PREFIX, bytes))
    }

    /// The token `presented` spells, or `None` unless it is `anima_agent_` followed by the
    /// canonical unpadded base64url of exactly 32 bytes.
    #[must_use]
    pub fn parse(presented: &str) -> Option<Self> {
        Secret::parse::<TOKEN_BYTES>(TOKEN_PREFIX, presented).map(Self)
    }

    /// The text, for the one place that carries it: the `Authorization: Bearer` header, the
    /// agent's token store, or the `Enrolled` and `Renewed` bodies. Never log it (LOG-007).
    #[must_use]
    pub fn expose(&self) -> &str {
        self.0.expose()
    }

    #[must_use]
    pub const fn hash(&self) -> TokenHash {
        TokenHash(self.0.hash)
    }
}

impl CodeHash {
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl TokenHash {
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Prefixed canonical base64url text, wiped on drop, and the SHA-256 of its bytes.
struct Secret {
    text: Zeroizing<String>,
    hash: [u8; 32],
}

/// Room for the base64 text of the longest secret (32 bytes give 43 characters).
const TEXT_ROOM: usize = 64;

impl Secret {
    fn from_random<const N: usize>(prefix: &str, mut bytes: [u8; N]) -> Self {
        const { assert!(N <= TEXT_ROOM / 4 * 3) };
        // Encoded into a buffer wiped on drop, not through the engine's own unwiped one.
        let mut encoded = Zeroizing::new([0_u8; TEXT_ROOM]);
        let written = URL_SAFE_NO_PAD
            .encode_slice(bytes.as_slice(), encoded.as_mut_slice())
            .expect("the bytes fit the room");
        let mut text = Zeroizing::new(String::with_capacity(prefix.len() + written));
        text.push_str(prefix);
        text.extend(encoded[..written].iter().copied().map(char::from));
        let hash = Sha256::digest(bytes.as_slice()).into();
        bytes.zeroize();
        Self { text, hash }
    }

    fn parse<const N: usize>(prefix: &str, presented: &str) -> Option<Self> {
        let encoded = presented.strip_prefix(prefix)?;
        if Some(encoded.len()) != base64::encoded_len(N, false) {
            return None;
        }
        // Room for the engine's estimate, so the buffer never moves and is wiped on every path.
        let mut bytes = Zeroizing::new(Vec::with_capacity(N + 3));
        // The engine refuses padding and a last character with bits beyond the bytes.
        URL_SAFE_NO_PAD.decode_vec(encoded, &mut bytes).ok()?;
        if bytes.len() != N {
            return None;
        }
        Some(Self {
            text: Zeroizing::new(presented.to_owned()),
            hash: Sha256::digest(bytes.as_slice()).into(),
        })
    }

    fn expose(&self) -> &str {
        &self.text
    }
}

/// Writes a code into a body: the enrol request.
pub(super) fn write_code<S: Serializer>(code: &EnrolmentCode, out: S) -> Result<S::Ok, S::Error> {
    out.serialize_str(code.expose())
}

/// Writes a token into a body: `Enrolled` or `Renewed`.
pub(super) fn write_token<S: Serializer>(token: &AgentToken, out: S) -> Result<S::Ok, S::Error> {
    out.serialize_str(token.expose())
}

/// Reads a code from a body. The error never holds the text.
pub(super) fn read_code<'de, D: Deserializer<'de>>(input: D) -> Result<EnrolmentCode, D::Error> {
    let text = Zeroizing::new(String::deserialize(input)?);
    EnrolmentCode::parse(&text).ok_or_else(|| D::Error::custom("not an enrolment code"))
}

/// Reads a token from a body. The error never holds the text.
pub(super) fn read_token<'de, D: Deserializer<'de>>(input: D) -> Result<AgentToken, D::Error> {
    let text = Zeroizing::new(String::deserialize(input)?);
    AgentToken::parse(&text).ok_or_else(|| D::Error::custom("not an agent token"))
}
