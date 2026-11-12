//! What the web view asks for, checked before anything runs (MET-019, MET-020): an enrolment
//! code in the form the hub issues, and the hub's `https` origin. Pure.

use std::fmt;

use url::Url;
use zeroize::Zeroizing;

/// K8 Section 3: the prefix, then the unpadded base64url of 16 bytes.
const CODE_PREFIX: &str = "anima_enrol_";
const CODE_TAIL_LENGTH: usize = 22;

/// An enrolment code. It goes only to the agent's standard input; it has no `Display`, its
/// `Debug` hides it, and its bytes are wiped when it is dropped (LOG-007).
pub struct EnrolmentCode(Zeroizing<String>);

impl EnrolmentCode {
    /// The code in `text`, when it is `anima_enrol_` and 22 base64url characters.
    pub fn parse(text: &str) -> Option<Self> {
        let tail = text.strip_prefix(CODE_PREFIX)?;
        let base64url = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-';
        (tail.len() == CODE_TAIL_LENGTH && tail.bytes().all(base64url))
            .then(|| Self(Zeroizing::new(text.to_owned())))
    }

    /// The line the agent reads at `enrol -`: the code and a newline, wiped when dropped.
    pub fn input_line(&self) -> Zeroizing<Vec<u8>> {
        let mut line = Zeroizing::new(Vec::with_capacity(self.0.len() + 1));
        line.extend_from_slice(self.0.as_bytes());
        line.push(b'\n');
        line
    }
}

impl fmt::Debug for EnrolmentCode {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str("EnrolmentCode(hidden)")
    }
}

/// The hub's origin as the agent must reach it: `https`, a host, port 443 (MET-020), and nothing
/// after the host. Held in the URL standard's serialisation, so the agent gets one spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubOrigin(String);

impl HubOrigin {
    /// The origin `text` spells. It must be `https://` and an authority with no user, path,
    /// query, fragment or white space; a port other than 443 is refused, as the agent refuses it.
    pub fn parse(text: &str) -> Option<Self> {
        let authority = text.strip_prefix("https://")?;
        let plain = |character: char| {
            !character.is_whitespace() && !character.is_control() && !"/?#@\\".contains(character)
        };
        if authority.is_empty() || !authority.chars().all(plain) {
            return None;
        }
        let url = Url::parse(text).ok()?;
        let bare = url.scheme() == "https"
            && url.host().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.port().is_none();
        bare.then(|| Self(url.origin().ascii_serialization()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
