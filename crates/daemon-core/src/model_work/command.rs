//! The command line and the environment of a runtime (Section 2.1, SEC-C-001, SEC-C-007): made
//! from the code of the daemon, the typed settings and the values that the daemon owns. No word
//! of the hub reaches the command line but the numbers of [`ServerSettings`].

use std::path::Path;

use super::settings::{GpuLayers, ServerSettings};

/// The variable that holds the API key of a `llama_server` (never on its command line).
pub const LLAMA_KEY_VARIABLE: &str = "LLAMA_API_KEY";

/// The parallel slots until the fit estimate of task L1-15 exists.
pub const PARALLEL_SLOTS: u32 = 1;

/// The arguments of a `llama_server` in router mode (RTC-005) on `127.0.0.1:<port>`.
#[must_use]
pub fn llama_arguments(settings: &ServerSettings, port: u16) -> Vec<String> {
    let mut words: Vec<String> = [
        "--host",
        "127.0.0.1",
        "--port",
        &port.to_string(),
        "--log-disable",
        "--verbosity",
        "0",
        "--no-slots",
        "--parallel",
        &PARALLEL_SLOTS.to_string(),
    ]
    .map(str::to_owned)
    .to_vec();
    let mut add = |flag: &str, value: String| words.extend([flag.to_owned(), value]);
    if let Some(tokens) = settings.context_tokens {
        add("--ctx-size", tokens.to_string());
    }
    if let Some(tokens) = settings.batch_tokens {
        add("--batch-size", tokens.to_string());
    }
    match settings.gpu_layers {
        Some(GpuLayers::Count(layers)) => add("--n-gpu-layers", layers.to_string()),
        Some(GpuLayers::All) => add("--n-gpu-layers", "999".to_owned()),
        None => {}
    }
    if let Some(kind) = settings.kv_cache {
        add("--cache-type-k", kind.as_str().to_owned());
        add("--cache-type-v", kind.as_str().to_owned());
    }
    words
}

/// The whole environment of a runtime process: its temporary folder as `HOME` and `TMPDIR`,
/// `PATH` set to the `bin` folder of the runtime only, the fixed offline values, and the
/// variable of the API key. Nothing else: no proxy, no token, no `DYLD_*`, no `LD_PRELOAD`.
#[must_use]
pub fn environment(temporary: &Path, bin: &Path, key: &str) -> Vec<(String, String)> {
    let temporary = temporary.display().to_string();
    vec![
        ("HOME".to_owned(), temporary.clone()),
        ("TMPDIR".to_owned(), temporary),
        ("PATH".to_owned(), bin.display().to_string()),
        ("HF_HUB_OFFLINE".to_owned(), "1".to_owned()),
        ("TRANSFORMERS_OFFLINE".to_owned(), "1".to_owned()),
        ("VLLM_NO_USAGE_STATS".to_owned(), "1".to_owned()),
        ("DO_NOT_TRACK".to_owned(), "1".to_owned()),
        (LLAMA_KEY_VARIABLE.to_owned(), key.to_owned()),
    ]
}
