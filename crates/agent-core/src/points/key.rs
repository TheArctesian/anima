//! Which series a value belongs to (MET-014, K8 Section 4).

use super::{Kind, Sample};

/// The highest GPU index a series or a GPU may carry (K8 Section 4).
pub const MAX_GPU_INDEX: u8 = 63;

/// The most series one point or one batch holds (K8 Section 4).
pub const MAX_SERIES: usize = 256;

/// The longest label or GPU name, in bytes (K8 Section 4).
const LABEL_BYTES: usize = 128;

/// One series of a device: its kind, its label (disk, network and drive kinds only) and its GPU index
/// (GPU kinds only). Built only by [`SeriesKey::new`], so every key is one K8 Section 4 allows.
/// Ordered by kind, then label, then GPU, so a batch lists its series the same way each time.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SeriesKey {
    kind: Kind,
    label: Option<String>,
    gpu: Option<u8>,
}

impl SeriesKey {
    /// The key of a series of `kind`, or `None` when K8 Section 4 does not allow it: a label is
    /// given exactly for the disk, network and drive kinds (1 to 128 bytes, no control character), and
    /// a GPU index (0 to 63) exactly for the GPU kinds.
    #[must_use]
    pub fn new(kind: Kind, label: Option<String>, gpu: Option<u8>) -> Option<Self> {
        let label_fits = match &label {
            Some(text) => kind.is_labelled() && is_label(text),
            None => !kind.is_labelled(),
        };
        let gpu_fits = match gpu {
            Some(index) => kind.is_per_gpu() && index <= MAX_GPU_INDEX,
            None => !kind.is_per_gpu(),
        };
        (label_fits && gpu_fits).then_some(Self { kind, label, gpu })
    }

    /// The key of a collection sample: its label for a disk, network or drive kind, its GPU index for
    /// a GPU kind (whose label, the same index, stays out of the key), and `None` when K8
    /// Section 4 does not allow it, for instance a GPU kind's sample with no index (R-099).
    #[must_use]
    pub fn of(sample: &Sample) -> Option<Self> {
        let label = sample.kind.is_labelled().then(|| sample.label.clone());
        Self::new(sample.kind, label, sample.gpu_index)
    }

    #[must_use]
    pub const fn kind(&self) -> Kind {
        self.kind
    }

    /// The mount point, interface or drive name; `None` for the kinds without one.
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// The GPU index; `None` for the kinds that belong to no GPU.
    #[must_use]
    pub const fn gpu(&self) -> Option<u8> {
        self.gpu
    }
}

/// Whether `text` can be a series label or a GPU name: 1 to 128 bytes, no control character
/// (K8 Section 4).
pub(crate) fn is_label(text: &str) -> bool {
    (1..=LABEL_BYTES).contains(&text.len()) && !text.chars().any(char::is_control)
}
