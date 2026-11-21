//! The fit estimate against known public model shapes and common memory sizes. Each expected
//! number is computed here from the formula, so the table shows its own arithmetic.

use super::{FitError, FitRequest, FitVerdict, KvShape, OverheadTable, estimate_fit};

const GIB: u64 = 1024 * 1024 * 1024;
const F16: u32 = 2;

fn shape(layers: u32, kv_heads: u32, head_dim: u32) -> KvShape {
    KvShape {
        layers,
        kv_heads,
        head_dim,
        bytes_per_element: F16,
    }
}

// meta-llama/Llama-3.1-8B-Instruct config.json: 32 layers, 8 KV heads, head dim 4096/32 = 128.
// Weights: bartowski/Meta-Llama-3.1-8B-Instruct-GGUF, Q4_K_M file, about 4.92 GB.
const LLAMA_8B_Q4: u64 = 4_920_000_000;
// Qwen/Qwen2.5-32B-Instruct config.json: 64 layers, 8 KV heads, head dim 5120/40 = 128.
// Weights: bartowski/Qwen2.5-32B-Instruct-GGUF, Q4_K_M file, about 19.85 GB.
const QWEN_32B_Q4: u64 = 19_850_000_000;
// meta-llama/Llama-3.1-70B-Instruct config.json: 80 layers, 8 KV heads, head dim 8192/64 = 128.
// Weights: bartowski/Meta-Llama-3.1-70B-Instruct-GGUF, Q4_K_M file, about 42.52 GB.
const LLAMA_70B_Q4: u64 = 42_520_000_000;
// Qwen/Qwen2.5-7B-Instruct config.json: 28 layers, 4 KV heads, head dim 3584/28 = 128.
// Weights: bartowski/Qwen2.5-7B-Instruct-GGUF, Q4_K_M file, about 4.68 GB.
const QWEN_7B_Q4: u64 = 4_680_000_000;

fn overhead() -> u64 {
    OverheadTable::default().bytes_for("llama.cpp", "metal")
}

fn request(weights: u64, kv: KvShape, context: u64, usable: u64) -> FitRequest {
    FitRequest {
        weight_bytes: weights,
        kv,
        context_tokens: context,
        overhead_bytes: overhead(),
        usable_bytes: usable,
    }
}

fn round_down(tokens: u64) -> u64 {
    tokens / 256 * 256
}

#[test]
fn kv_bytes_per_token_follow_the_formula() {
    assert_eq!(
        shape(32, 8, 128).bytes_per_token(),
        Ok(2 * 32 * 8 * 128 * 2)
    );
    assert_eq!(shape(28, 4, 128).bytes_per_token(), Ok(57_344));
}

#[test]
fn llama_8b_fits_at_8k_on_16_gib() {
    let r = request(LLAMA_8B_Q4, shape(32, 8, 128), 8192, 16 * GIB);
    let used = LLAMA_8B_Q4 + overhead() + 131_072 * 8192;
    assert_eq!(estimate_fit(&r), Ok(FitVerdict::Fits { used_bytes: used }));
}

#[test]
fn llama_8b_at_128k_on_16_gib_gets_a_reduced_context() {
    let r = request(LLAMA_8B_Q4, shape(32, 8, 128), 131_072, 16 * GIB);
    let largest = round_down((16 * GIB - LLAMA_8B_Q4 - overhead()) / 131_072);
    let used = LLAMA_8B_Q4 + overhead() + 131_072 * largest;
    assert_eq!(
        estimate_fit(&r),
        Ok(FitVerdict::ReducedContext {
            largest_context: largest,
            used_bytes: used
        })
    );
    assert_eq!(largest % 256, 0);
}

#[test]
fn qwen_32b_fits_at_32k_on_48_gib_and_not_on_24_gib() {
    let per_token = 2 * 64 * 8 * 128 * 2;
    let fits = request(QWEN_32B_Q4, shape(64, 8, 128), 32_768, 48 * GIB);
    let used = QWEN_32B_Q4 + overhead() + per_token * 32_768;
    assert_eq!(
        estimate_fit(&fits),
        Ok(FitVerdict::Fits { used_bytes: used })
    );
    let tight = request(QWEN_32B_Q4, shape(64, 8, 128), 32_768, 24 * GIB);
    let largest = round_down((24 * GIB - QWEN_32B_Q4 - overhead()) / per_token);
    let verdict = estimate_fit(&tight).unwrap();
    assert!(
        matches!(verdict, FitVerdict::ReducedContext { largest_context, .. } if largest_context == largest)
    );
}

#[test]
fn llama_70b_on_32_48_and_128_gib() {
    let kv = shape(80, 8, 128);
    let per_token = 2 * 80 * 8 * 128 * 2;
    let missing = LLAMA_70B_Q4 + overhead() + per_token * 256 - 32 * GIB;
    let small = request(LLAMA_70B_Q4, kv, 8192, 32 * GIB);
    assert_eq!(
        estimate_fit(&small),
        Ok(FitVerdict::NoContext {
            missing_bytes: missing
        })
    );

    let medium = request(LLAMA_70B_Q4, kv, 131_072, 48 * GIB);
    let largest = round_down((48 * GIB - LLAMA_70B_Q4 - overhead()) / per_token);
    let used = LLAMA_70B_Q4 + overhead() + per_token * largest;
    assert_eq!(
        estimate_fit(&medium),
        Ok(FitVerdict::ReducedContext {
            largest_context: largest,
            used_bytes: used
        })
    );

    let large = request(LLAMA_70B_Q4, kv, 131_072, 128 * GIB);
    let used = LLAMA_70B_Q4 + overhead() + per_token * 131_072;
    assert_eq!(
        estimate_fit(&large),
        Ok(FitVerdict::Fits { used_bytes: used })
    );
}

#[test]
fn qwen_7b_at_128k_on_8_gib_gets_a_reduced_context() {
    let r = request(QWEN_7B_Q4, shape(28, 4, 128), 131_072, 8 * GIB);
    let largest = round_down((8 * GIB - QWEN_7B_Q4 - overhead()) / 57_344);
    let verdict = estimate_fit(&r).unwrap();
    assert!(
        matches!(verdict, FitVerdict::ReducedContext { largest_context, .. } if largest_context == largest)
    );
    assert!(largest > 0 && largest < 131_072);
    assert!(verdict.reason(&r).contains("largest context"));
}

#[test]
fn context_below_one_step_is_no_context() {
    let per_token = 131_072;
    let usable = LLAMA_8B_Q4 + overhead() + per_token * 255;
    let r = request(LLAMA_8B_Q4, shape(32, 8, 128), 4096, usable);
    assert_eq!(
        estimate_fit(&r),
        Ok(FitVerdict::NoContext {
            missing_bytes: per_token
        })
    );
}

#[test]
fn huge_sizes_give_an_overflow_error() {
    let r = request(u64::MAX, shape(32, 8, 128), 1, 16 * GIB);
    assert_eq!(estimate_fit(&r), Err(FitError::Overflow));
    let kv = KvShape {
        layers: u32::MAX,
        kv_heads: u32::MAX,
        head_dim: u32::MAX,
        bytes_per_element: 2,
    };
    assert_eq!(kv.bytes_per_token(), Err(FitError::Overflow));
}

#[test]
fn a_measurement_replaces_the_default_overhead() {
    let mut table = OverheadTable::default();
    table.record_measurement("llama.cpp", "metal", 700);
    table.record_measurement("vllm", "cuda", 900);
    assert_eq!(table.bytes_for("llama.cpp", "metal"), 700);
    assert_eq!(table.bytes_for("vllm", "cuda"), 900);
    assert_eq!(table.bytes_for("other", "cpu"), table.fallback_bytes);
}
