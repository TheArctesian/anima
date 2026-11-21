//! The answer of Hugging Face for one repository (`/api/models/<repo>?blobs=true`) and the
//! `config.json` of its files, read into the facts of a model (LIBM-004, LIBM-005). Pure: the
//! text comes from the I/O edge in `fetch`.

use serde::Deserialize;
use serde_json::Value;

use super::{
    capability,
    model::{Facts, LayerShape},
};

/// The part of the answer that the library reads.
#[derive(Debug, Deserialize)]
pub struct Listing {
    pub id: String,
    /// The commit that the file list is from.
    pub sha: String,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub disabled: bool,
    /// `false`, or a text such as `"auto"` or `"manual"` when the repository is gated.
    #[serde(default)]
    pub gated: Value,
    #[serde(default)]
    pub tags: Vec<String>,
    pub pipeline_tag: Option<String>,
    pub library_name: Option<String>,
    #[serde(default, rename = "cardData")]
    pub card: Card,
    #[serde(default)]
    pub config: RepositoryConfig,
    pub gguf: Option<GgufSummary>,
    pub safetensors: Option<SafetensorsSummary>,
    #[serde(default)]
    pub siblings: Vec<Sibling>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Card {
    /// One repository name, or a list of them.
    #[serde(default)]
    pub base_model: Value,
    pub license: Option<Value>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RepositoryConfig {
    pub model_type: Option<String>,
}

/// What Hugging Face reads from the header of the GGUF files.
#[derive(Debug, Deserialize)]
pub struct GgufSummary {
    pub total: Option<u64>,
    pub architecture: Option<String>,
    pub context_length: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct SafetensorsSummary {
    pub total: Option<u64>,
}

/// One file of the repository. Only a file in LFS has its SHA-256 value in the answer.
#[derive(Debug, Deserialize)]
pub struct Sibling {
    pub rfilename: String,
    pub size: Option<u64>,
    pub lfs: Option<Lfs>,
}

#[derive(Debug, Deserialize)]
pub struct Lfs {
    pub sha256: String,
    pub size: u64,
}

impl Listing {
    /// Whether the files can be read without an account: not private, gated or turned off.
    #[must_use]
    pub fn is_open(&self) -> bool {
        !self.private && !self.disabled && matches!(self.gated, Value::Null | Value::Bool(false))
    }

    /// Whether a repository file is `config.json` at the top.
    #[must_use]
    pub fn has_config(&self) -> bool {
        self.siblings
            .iter()
            .any(|file| file.rfilename == "config.json")
    }

    /// The organisation of the first base model that the card names.
    #[must_use]
    pub fn base_organisation(&self) -> Option<String> {
        let first = match &self.card.base_model {
            Value::String(text) => Some(text.as_str()),
            Value::Array(list) => list.first().and_then(Value::as_str),
            _ => None,
        }?;
        first
            .split_once('/')
            .map(|(organisation, _)| organisation.to_owned())
    }

    fn licence(&self) -> Option<String> {
        let from_card = match &self.card.license {
            Some(Value::String(text)) => Some(text.clone()),
            Some(Value::Array(list)) => list.first().and_then(Value::as_str).map(str::to_owned),
            _ => None,
        };
        from_card.or_else(|| {
            self.tags
                .iter()
                .find_map(|tag| tag.strip_prefix("license:").map(str::to_owned))
        })
    }

    /// The facts of the answer, completed by `config.json` when the repository has one. The
    /// values of `config.json` come first: they are the files of the model itself.
    #[must_use]
    pub fn facts(&self, config: Option<&ModelConfig>, publisher: Option<String>) -> Facts {
        let gguf = self.gguf.as_ref();
        let config_family = config.and_then(|each| each.model_type.clone());
        Facts {
            family: config_family
                .or_else(|| self.config.model_type.clone())
                .or_else(|| gguf.and_then(|each| each.architecture.clone())),
            parameter_count: self
                .safetensors
                .as_ref()
                .and_then(|each| each.total)
                .or_else(|| gguf.and_then(|each| each.total)),
            context_length: config
                .and_then(ModelConfig::context_length)
                .or_else(|| gguf.and_then(|each| each.context_length)),
            shape: config.map_or_else(LayerShape::default, ModelConfig::shape),
            capabilities: capability::from_repository(self.pipeline_tag.as_deref(), &self.tags),
            licence: self.licence(),
            publisher,
        }
    }
}

/// The fields of `config.json` that the library reads. A model with vision or audio keeps
/// its language part in `text_config`.
#[derive(Debug, Default, Deserialize)]
pub struct ModelConfig {
    pub model_type: Option<String>,
    pub max_position_embeddings: Option<u32>,
    pub num_hidden_layers: Option<u32>,
    pub hidden_size: Option<u32>,
    pub num_attention_heads: Option<u32>,
    pub num_key_value_heads: Option<u32>,
    pub torch_dtype: Option<String>,
    pub quantization: Option<Quantization>,
    pub text_config: Option<Box<ModelConfig>>,
}

#[derive(Debug, Deserialize)]
pub struct Quantization {
    pub bits: Option<u32>,
}

impl ModelConfig {
    fn text(&self) -> &Self {
        self.text_config.as_deref().unwrap_or(self)
    }

    fn context_length(&self) -> Option<u32> {
        self.text()
            .max_position_embeddings
            .or(self.max_position_embeddings)
    }

    fn shape(&self) -> LayerShape {
        let text = self.text();
        LayerShape {
            layers: text.num_hidden_layers,
            hidden_size: text.hidden_size,
            attention_heads: text.num_attention_heads,
            key_value_heads: text.num_key_value_heads.or(text.num_attention_heads),
        }
    }

    /// The quantisation of safetensors files: `4bit` from the MLX field, else the weight
    /// type, such as `bf16`.
    #[must_use]
    pub fn quantisation(&self) -> Option<String> {
        if let Some(bits) = self.quantization.as_ref().and_then(|each| each.bits) {
            return Some(format!("{bits}bit"));
        }
        let dtype = self.torch_dtype.as_deref().or_else(|| {
            self.text_config
                .as_deref()
                .and_then(|text| text.torch_dtype.as_deref())
        })?;
        Some(
            match dtype {
                "bfloat16" => "bf16",
                "float16" => "f16",
                "float32" => "f32",
                other => other,
            }
            .to_owned(),
        )
    }
}

/// Reads the answer for one repository.
///
/// # Errors
/// The parser's error when the text is not an answer of the expected form.
pub fn listing(text: &[u8]) -> Result<Listing, serde_json::Error> {
    serde_json::from_slice(text)
}

/// Reads `config.json`.
///
/// # Errors
/// The parser's error when the text is not a JSON object.
pub fn model_config(text: &[u8]) -> Result<ModelConfig, serde_json::Error> {
    serde_json::from_slice(text)
}
