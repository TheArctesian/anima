//! The bearer token: a login token or an API key (CMD-010, CMD-011). It never prints: no
//! `Display`, a `Debug` that hides it, and its memory is wiped when it drops (CMD-014, LOG-007).

use std::fmt;

use zeroize::Zeroizing;

/// An API key's secret starts with this (K7, INT-003).
const API_KEY_PREFIX: &str = "anima_";

/// The longest token the command line accepts; the hub's are under 100 characters.
const MAX_LENGTH: usize = 4096;

/// A hub token. Only [`Token::expose`] reads it, for the `Authorization` header and the key
/// store.
pub struct Token(Zeroizing<String>);

/// Which kind of token the hub gave (INT-003).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TokenKind {
    /// From `anima auth login`; permits every operation.
    Login,
    /// `anima_…`; permits what its scope and INT-003 allow.
    ApiKey,
}

impl Token {
    /// `text` without surrounding white space, as a token; `None` when it is empty, too long,
    /// or holds a character that cannot go in an HTTP header.
    #[must_use]
    pub fn new(text: Zeroizing<String>) -> Option<Self> {
        let trimmed = text.trim();
        let fits = !trimmed.is_empty()
            && trimmed.len() <= MAX_LENGTH
            && trimmed.bytes().all(|byte| byte.is_ascii_graphic());
        if !fits {
            return None;
        }
        if trimmed.len() == text.len() {
            return Some(Self(text));
        }
        Some(Self(Zeroizing::new(trimmed.to_owned())))
    }

    /// The secret itself. Callers put it only in the `Authorization` header or the key store.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn kind(&self) -> TokenKind {
        if self.0.starts_with(API_KEY_PREFIX) {
            TokenKind::ApiKey
        } else {
            TokenKind::Login
        }
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Token({:?}, hidden)", self.kind())
    }
}

impl TokenKind {
    /// The kind in words, for `auth status`.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Login => "a login token",
            Self::ApiKey => "an API key",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(text: &str) -> Option<Token> {
        Token::new(Zeroizing::new(text.to_owned()))
    }

    #[test]
    fn cmd_014_debug_output_never_holds_the_token() {
        let secret = "anima_s3cretS3cretS3cret";
        let shown = format!("{:?} {:#?}", token(secret), token("plain-login-token"));
        assert!(!shown.contains("s3cret"), "{shown}");
        assert!(!shown.contains("plain-login"), "{shown}");
        assert!(
            shown.contains("ApiKey") && shown.contains("Login"),
            "{shown}"
        );
    }

    #[test]
    fn cmd_011_a_token_is_one_word_of_visible_characters() {
        assert_eq!(
            token("  abc\n").map(|token| token.expose().to_owned()),
            Some("abc".into())
        );
        assert!(token("").is_none());
        assert!(token(" \n").is_none());
        assert!(token("two words").is_none());
        assert!(token("line\nbreak").is_none());
        assert!(token(&"x".repeat(MAX_LENGTH + 1)).is_none());
    }

    #[test]
    fn cmd_011_a_key_starts_with_anima_and_anything_else_is_a_login_token() {
        assert_eq!(
            token("anima_Ab12").map(|token| token.kind()),
            Some(TokenKind::ApiKey)
        );
        assert_eq!(
            token("Ab12").map(|token| token.kind()),
            Some(TokenKind::Login)
        );
    }
}
