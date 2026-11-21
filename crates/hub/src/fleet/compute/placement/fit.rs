//! The fit estimate: weights, fixed overhead and KV cache against the usable memory of a node.

use thiserror::Error;

/// Contexts are given and reduced in steps of this many tokens (FIT-004).
pub const CONTEXT_STEP: u64 = 256;

/// The KV cache shape of a model, from its model files (FIT-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KvShape {
    pub layers: u32,
    pub kv_heads: u32,
    pub head_dim: u32,
    pub bytes_per_element: u32,
}

impl KvShape {
    /// KV bytes for each token: 2 (K and V) × layers × KV heads × head dimension × element bytes.
    ///
    /// # Errors
    ///
    /// `FitError::Overflow` when the product does not fit in 64 bits.
    pub fn bytes_per_token(&self) -> Result<u64, FitError> {
        [
            self.layers,
            self.kv_heads,
            self.head_dim,
            self.bytes_per_element,
        ]
        .into_iter()
        .try_fold(2u64, |acc, factor| acc.checked_mul(u64::from(factor)))
        .ok_or(FitError::Overflow)
    }
}

/// One variant on one node: what the model needs and what the node can give (FIT-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FitRequest {
    pub weight_bytes: u64,
    pub kv: KvShape,
    pub context_tokens: u64,
    /// The fixed overhead of the runtime and backend (FIT-003, see `OverheadTable`).
    pub overhead_bytes: u64,
    /// The usable memory of the node (CAP-001, CAP-005), not its total memory.
    pub usable_bytes: u64,
}

/// The verdict for one variant on one node (FIT-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitVerdict {
    /// The model fits at the requested context and uses this many bytes.
    Fits { used_bytes: u64 },
    /// The model does not fit at the requested context; this smaller context fits (FIT-004).
    ReducedContext {
        largest_context: u64,
        used_bytes: u64,
    },
    /// The model does not fit at any context of at least one step; this many bytes are missing.
    NoContext { missing_bytes: u64 },
}

impl FitVerdict {
    /// The reason in words, for the operator.
    #[must_use]
    pub fn reason(&self, request: &FitRequest) -> String {
        match self {
            Self::Fits { used_bytes } => format!(
                "fits: {used_bytes} of {} usable bytes at {} tokens of context",
                request.usable_bytes, request.context_tokens
            ),
            Self::ReducedContext {
                largest_context, ..
            } => format!(
                "does not fit at {} tokens of context; the largest context that fits is {largest_context}",
                request.context_tokens
            ),
            Self::NoContext { missing_bytes } => format!(
                "does not fit at any context: weights and overhead need {missing_bytes} bytes more than the {} usable",
                request.usable_bytes
            ),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FitError {
    #[error("the memory sizes of the fit estimate overflow 64 bits")]
    Overflow,
}

/// Estimate whether a variant fits on a node, and if not, the largest context that fits.
///
/// # Errors
///
/// `FitError::Overflow` when a memory size does not fit in 64 bits.
pub fn estimate_fit(request: &FitRequest) -> Result<FitVerdict, FitError> {
    let per_token = request.kv.bytes_per_token()?;
    let fixed = request
        .weight_bytes
        .checked_add(request.overhead_bytes)
        .ok_or(FitError::Overflow)?;
    let used = per_token
        .checked_mul(request.context_tokens)
        .and_then(|kv| kv.checked_add(fixed))
        .ok_or(FitError::Overflow)?;
    if used <= request.usable_bytes {
        return Ok(FitVerdict::Fits { used_bytes: used });
    }
    let free_for_kv = request.usable_bytes.saturating_sub(fixed);
    let largest = match free_for_kv.checked_div(per_token) {
        Some(tokens) => tokens - tokens % CONTEXT_STEP,
        None => 0,
    };
    if largest == 0 {
        let minimum = per_token
            .checked_mul(CONTEXT_STEP)
            .and_then(|kv| kv.checked_add(fixed))
            .ok_or(FitError::Overflow)?;
        let missing_bytes = minimum - request.usable_bytes.min(minimum);
        return Ok(FitVerdict::NoContext { missing_bytes });
    }
    Ok(FitVerdict::ReducedContext {
        largest_context: largest,
        used_bytes: fixed + per_token * largest,
    })
}
