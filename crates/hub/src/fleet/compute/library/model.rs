//! The records of the library: a model, its variants and their files (LIBM-001, LIBM-002).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use chrono::{DateTime, Utc};

use super::capability::Capability;
use crate::api_errors::FieldError;

/// The file format of a variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, async_graphql::Enum)]
#[graphql(name = "ModelFormat")]
pub enum Format {
    Gguf,
    Mlx,
    Safetensors,
}

impl Format {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Gguf => "gguf",
            Self::Mlx => "mlx",
            Self::Safetensors => "safetensors",
        }
    }

    #[must_use]
    pub fn from_stored(text: &str) -> Option<Self> {
        [Self::Gguf, Self::Mlx, Self::Safetensors]
            .into_iter()
            .find(|format| format.as_str() == text)
    }
}

/// The layer shape that the model files give (LIBM-005), for the fit estimate later.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LayerShape {
    pub layers: Option<u32>,
    pub hidden_size: Option<u32>,
    pub attention_heads: Option<u32>,
    pub key_value_heads: Option<u32>,
}

/// One file of a variant: its path in the repository, its size and its SHA-256 value as 64
/// lower-case hex digits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantFile {
    pub path: String,
    pub size_bytes: u64,
    pub sha256: String,
}

/// One variant of a model, addressed `name:tag` (LIBM-002, LIBM-006).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    pub tag: String,
    pub format: Format,
    /// The quantisation as the files name it, such as `Q4_K_M` or `4bit`.
    pub quantisation: String,
    /// `organisation/name` on Hugging Face.
    pub repository: String,
    /// The commit of the repository that the file list is from.
    pub revision: String,
    pub files: Vec<VariantFile>,
}

impl Variant {
    #[must_use]
    pub fn size_bytes(&self) -> u64 {
        self.files.iter().map(|file| file.size_bytes).sum()
    }
}

/// The facts of a model that the files give (LIBM-001, LIBM-005).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub family: Option<String>,
    pub parameter_count: Option<u64>,
    pub context_length: Option<u32>,
    pub shape: LayerShape,
    pub capabilities: Vec<Capability>,
    pub licence: Option<String>,
    /// The organisation of the base model, as an approved repository names it (LIBM-003).
    pub publisher: Option<String>,
}

/// A model of the library with its variants and aliases.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model {
    pub name: String,
    pub facts: Facts,
    pub variants: Vec<Variant>,
    pub aliases: Vec<String>,
    pub added_at: DateTime<Utc>,
}

impl Model {
    /// Each repository of the variants once, in the order of the variants.
    #[must_use]
    pub fn repositories(&self) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        for variant in &self.variants {
            if !found.contains(&variant.repository) {
                found.push(variant.repository.clone());
            }
        }
        found
    }
}

/// An alias and the model it points at (LIBM-007, SEL-003).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alias {
    pub alias: String,
    pub model: String,
}

/// What the library cannot find, by the name given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Missing {
    Model,
    Repository,
}

/// Why a repository gave no variant that the library can keep.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    #[error("the organisation of the repository is not in the approved list")]
    NotApproved,
    #[error("the repository is private, gated or turned off")]
    Closed,
    #[error("the repository has no GGUF, MLX or safetensors model files")]
    NoVariant,
    #[error("a file of the repository has no SHA-256 value")]
    NoHash,
}

impl Refusal {
    /// The input field error that a GraphQL client shows.
    #[must_use]
    pub fn field(&self) -> FieldError {
        let message = match self {
            Self::NotApproved => {
                "The organisation is not an approved source. Add it to the approved sources first."
            }
            Self::Closed => "The repository is private, gated or turned off.",
            Self::NoVariant => "The repository has no GGUF, MLX or safetensors model files.",
            Self::NoHash => "A file of the repository has no SHA-256 value.",
        };
        FieldError::new("repository", message)
    }
}
