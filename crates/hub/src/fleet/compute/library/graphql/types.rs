//! The output types of the library (K1 `LibraryModel`, `ModelVariant`, `ModelFile`,
//! `ModelAlias`).

use async_graphql::{Object, SimpleObject};
use chrono::{DateTime, Utc};

use super::super::{Alias, Capability, Format, Model, Variant, VariantFile, naming};

/// A count as a GraphQL `Float`: exact up to 2^53, which no model size reaches.
#[expect(clippy::cast_precision_loss, reason = "exact below 2^53")]
fn float(value: u64) -> f64 {
    value as f64
}

#[Object(name = "LibraryModel")]
impl Model {
    /// The name in the form `name` of `name:tag` (LIBM-006).
    async fn name(&self) -> &str {
        &self.name
    }

    async fn family(&self) -> Option<&str> {
        self.facts.family.as_deref()
    }

    async fn parameter_count(&self) -> Option<f64> {
        self.facts.parameter_count.map(float)
    }

    async fn context_length(&self) -> Option<u32> {
        self.facts.context_length
    }

    async fn layer_count(&self) -> Option<u32> {
        self.facts.shape.layers
    }

    async fn hidden_size(&self) -> Option<u32> {
        self.facts.shape.hidden_size
    }

    async fn attention_heads(&self) -> Option<u32> {
        self.facts.shape.attention_heads
    }

    async fn key_value_heads(&self) -> Option<u32> {
        self.facts.shape.key_value_heads
    }

    /// What the model can do (LIBM-008); not evidence that a runtime runs it (LIBM-009).
    async fn capabilities(&self) -> &[Capability] {
        &self.facts.capabilities
    }

    async fn licence(&self) -> Option<&str> {
        self.facts.licence.as_deref()
    }

    async fn publisher(&self) -> Option<&str> {
        self.facts.publisher.as_deref()
    }

    async fn source_repositories(&self) -> Vec<String> {
        self.repositories()
    }

    async fn aliases(&self) -> &[String] {
        &self.aliases
    }

    async fn variants(&self) -> Vec<VariantOf<'_>> {
        self.variants
            .iter()
            .map(|variant| VariantOf {
                model: &self.name,
                variant,
            })
            .collect()
    }

    async fn added_at(&self) -> DateTime<Utc> {
        self.added_at
    }
}

/// A variant with the name of its model, for its address.
pub(super) struct VariantOf<'a> {
    model: &'a str,
    variant: &'a Variant,
}

#[Object(name = "ModelVariant")]
impl VariantOf<'_> {
    /// `name:tag`, the address of the variant (LIBM-006).
    async fn address(&self) -> String {
        naming::address(self.model, &self.variant.tag)
    }

    async fn tag(&self) -> &str {
        &self.variant.tag
    }

    async fn format(&self) -> Format {
        self.variant.format
    }

    async fn quantisation(&self) -> &str {
        &self.variant.quantisation
    }

    async fn source_repository(&self) -> &str {
        &self.variant.repository
    }

    async fn revision(&self) -> &str {
        &self.variant.revision
    }

    async fn size_bytes(&self) -> f64 {
        float(self.variant.size_bytes())
    }

    async fn files(&self) -> &[VariantFile] {
        &self.variant.files
    }
}

#[Object(name = "ModelFile")]
impl VariantFile {
    async fn path(&self) -> &str {
        &self.path
    }

    async fn size_bytes(&self) -> f64 {
        float(self.size_bytes)
    }

    /// 64 lower-case hex digits; a pull verifies each file against it (PULL-005).
    async fn sha256(&self) -> &str {
        &self.sha256
    }
}

/// An alias and the model it points at now (SEL-003).
#[derive(SimpleObject)]
#[graphql(name = "ModelAlias")]
pub(super) struct AliasView {
    alias: String,
    model: String,
}

impl From<Alias> for AliasView {
    fn from(alias: Alias) -> Self {
        Self {
            alias: alias.alias,
            model: alias.model,
        }
    }
}
