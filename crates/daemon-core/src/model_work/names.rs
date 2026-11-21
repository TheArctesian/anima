//! The fixed lists of words of the model work (Sections 1 and 2). The hub cannot add a word.

/// A model server program of the fixed list (Section 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Runtime {
    LlamaServer,
    MlxLm,
    Vllm,
}

/// The backend of a runtime release.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Backend {
    Metal,
    Cuda,
    Rocm,
    Vulkan,
    Cpu,
}

/// The format of a variant (Section 2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    Gguf,
    Safetensors,
    Mlx,
}

/// A fixed request of the daemon for `model_measure`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Probe {
    TextShort,
    VisionShort,
}

/// Who asked the hub for the item (Section 3, LOG-012).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Actor {
    User,
    Agent,
    Hub,
}

/// A word of a fixed list, read from and spelled as the bodies spell it.
pub trait Word: Sized + Copy + 'static {
    const ALL: &'static [Self];
    fn as_str(self) -> &'static str;
    fn parse(text: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|word| word.as_str() == text)
    }
}

impl Word for Runtime {
    const ALL: &'static [Self] = &[Self::LlamaServer, Self::MlxLm, Self::Vllm];
    fn as_str(self) -> &'static str {
        match self {
            Self::LlamaServer => "llama_server",
            Self::MlxLm => "mlx_lm",
            Self::Vllm => "vllm",
        }
    }
}

impl Word for Backend {
    const ALL: &'static [Self] = &[Self::Metal, Self::Cuda, Self::Rocm, Self::Vulkan, Self::Cpu];
    fn as_str(self) -> &'static str {
        match self {
            Self::Metal => "metal",
            Self::Cuda => "cuda",
            Self::Rocm => "rocm",
            Self::Vulkan => "vulkan",
            Self::Cpu => "cpu",
        }
    }
}

impl Word for Format {
    const ALL: &'static [Self] = &[Self::Gguf, Self::Safetensors, Self::Mlx];
    fn as_str(self) -> &'static str {
        match self {
            Self::Gguf => "gguf",
            Self::Safetensors => "safetensors",
            Self::Mlx => "mlx",
        }
    }
}

impl Word for Probe {
    const ALL: &'static [Self] = &[Self::TextShort, Self::VisionShort];
    fn as_str(self) -> &'static str {
        match self {
            Self::TextShort => "text_short",
            Self::VisionShort => "vision_short",
        }
    }
}

impl Word for Actor {
    const ALL: &'static [Self] = &[Self::User, Self::Agent, Self::Hub];
    fn as_str(self) -> &'static str {
        match self {
            Self::User => "User",
            Self::Agent => "Agent",
            Self::Hub => "Hub",
        }
    }
}

/// Whether `text` is a name that Hugging Face permits for an organisation or a repository:
/// letters, digits, `-`, `_` and `.`, at most 96 characters, not `.` or `..` (Section 2).
#[must_use]
pub fn is_source_name(text: &str) -> bool {
    (1..=96).contains(&text.len())
        && text != "."
        && text != ".."
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// Whether `text` is `length` hexadecimal characters in lower case.
#[must_use]
pub fn is_lower_hex(text: &str, length: usize) -> bool {
    text.len() == length && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}
