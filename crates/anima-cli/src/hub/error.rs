//! How a hub request fails: the hub refused it with a K7 code, or no readable answer came.
//! No variant holds the token or the request body (CMD-014).

use serde::Serialize;

/// The K7 `extensions.code` values (contracts/conventions.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Code {
    Unauthenticated,
    Forbidden,
    NotFound,
    Validation,
    Conflict,
    Unavailable,
    Internal,
}

/// One rejected input field of a `VALIDATION` error.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum HubError {
    /// The hub answered with a GraphQL error.
    #[error("{message}")]
    Refused {
        code: Code,
        message: String,
        fields: Vec<FieldError>,
    },
    /// No answer came: the connection, TLS or the time limit failed.
    #[error("The hub at {origin} did not answer ({reason}).")]
    Unreachable { origin: String, reason: String },
    /// An answer came that is not a GraphQL response, or not the shape the document asked for.
    #[error("The hub's answer could not be read (HTTP status {status}).")]
    Malformed { status: u16 },
    /// The HTTP client could not be built.
    #[error("The HTTP client could not be set up.")]
    Setup,
}

impl Code {
    /// The code a hub error names; an unknown or missing code counts as `INTERNAL`.
    #[must_use]
    pub fn parse(text: Option<&str>) -> Self {
        match text {
            Some("UNAUTHENTICATED") => Self::Unauthenticated,
            Some("FORBIDDEN") => Self::Forbidden,
            Some("NOT_FOUND") => Self::NotFound,
            Some("VALIDATION") => Self::Validation,
            Some("CONFLICT") => Self::Conflict,
            Some("UNAVAILABLE") => Self::Unavailable,
            _ => Self::Internal,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unauthenticated => "UNAUTHENTICATED",
            Self::Forbidden => "FORBIDDEN",
            Self::NotFound => "NOT_FOUND",
            Self::Validation => "VALIDATION",
            Self::Conflict => "CONFLICT",
            Self::Unavailable => "UNAVAILABLE",
            Self::Internal => "INTERNAL",
        }
    }
}

impl HubError {
    /// The K7 code that stands for this failure.
    #[must_use]
    pub const fn code(&self) -> Code {
        match self {
            Self::Refused { code, .. } => *code,
            Self::Unreachable { .. } => Code::Unavailable,
            Self::Malformed { .. } | Self::Setup => Code::Internal,
        }
    }

    /// The rejected fields of a `VALIDATION` error; empty for any other.
    #[must_use]
    pub fn fields(&self) -> &[FieldError] {
        match self {
            Self::Refused { fields, .. } => fields,
            _ => &[],
        }
    }
}
