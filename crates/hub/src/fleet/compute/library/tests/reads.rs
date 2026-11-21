//! The read of a repository: the metadata, the SHA-256 values, the variants and the refusal
//! of a source that is not approved (LIBM-002 to LIBM-006).

use sha2::{Digest, Sha256};

use super::{MLX_CONFIG, Recorded};
use crate::fleet::compute::library::{
    Capability, Error, Format, Refusal,
    naming::{self, Repository},
    reading::{self, Known, Read},
};

fn approved() -> Vec<String> {
    vec!["unsloth".to_owned(), "mlx-community".to_owned()]
}

fn repository(text: &str) -> Repository {
    naming::repository(text).unwrap()
}

async fn read(fetch: &Recorded, text: &str, publisher: Option<&str>) -> Result<Read, Error> {
    let source = repository(text);
    let name = naming::proposed(&source);
    let approved = approved();
    let known = Known {
        approved: &approved,
        publisher,
    };
    reading::read(fetch, &source, name, known).await
}

#[tokio::test]
async fn a_gguf_repository_gives_the_facts_of_its_files() {
    let read = read(&Recorded::default(), "unsloth/Qwen3-0.6B-GGUF", None)
        .await
        .unwrap();
    assert_eq!(read.name, "qwen3-0.6b");
    let facts = &read.facts;
    assert_eq!(facts.family.as_deref(), Some("qwen3"));
    assert_eq!(facts.parameter_count, Some(596_049_920));
    assert_eq!(facts.context_length, Some(40_960));
    assert_eq!(facts.shape.layers, Some(28));
    assert_eq!(facts.shape.hidden_size, Some(1024));
    assert_eq!(facts.shape.attention_heads, Some(16));
    assert_eq!(facts.shape.key_value_heads, Some(8));
    assert_eq!(facts.licence.as_deref(), Some("apache-2.0"));
    assert_eq!(facts.publisher.as_deref(), Some("Qwen"));
    assert_eq!(facts.capabilities, vec![Capability::Chat]);
}

#[tokio::test]
async fn each_gguf_quantisation_is_a_variant_with_the_lfs_sha256() {
    let read = read(&Recorded::default(), "unsloth/Qwen3-0.6B-GGUF", None)
        .await
        .unwrap();
    assert_eq!(read.variants.len(), 26);
    let q4 = read
        .variants
        .iter()
        .find(|each| each.tag == "q4_k_m")
        .unwrap();
    assert_eq!(q4.format, Format::Gguf);
    assert_eq!(q4.quantisation, "Q4_K_M");
    assert_eq!(q4.revision, "50968a4468ef4233ed78cd7c3de230dd1d61a56b");
    assert_eq!(q4.files.len(), 1);
    assert_eq!(q4.files[0].path, "Qwen3-0.6B-Q4_K_M.gguf");
    assert_eq!(q4.files[0].size_bytes, 396_705_472);
    assert_eq!(
        q4.files[0].sha256,
        "ac2d97712095a558e31573f62f466a3f9d93990898b0ec79d7c974c1780d524a"
    );
    assert!(read.variants.iter().any(|each| each.tag == "ud-q4_k_xl"));
    assert!(read.variants.iter().all(|each| naming::is_name(&each.tag)));
}

#[tokio::test]
async fn an_mlx_repository_is_one_variant_and_a_small_file_is_hashed_by_the_hub() {
    let fetch = Recorded::default();
    let read = read(&fetch, "mlx-community/Qwen3-0.6B-4bit", None)
        .await
        .unwrap();
    assert_eq!(read.name, "qwen3-0.6b");
    let [variant] = read.variants.as_slice() else {
        panic!("one variant")
    };
    assert_eq!(
        (variant.format, variant.tag.as_str()),
        (Format::Mlx, "4bit")
    );
    let file = |path: &str| variant.files.iter().find(|each| each.path == path);
    assert_eq!(
        file("model.safetensors").unwrap().sha256,
        "392e8d466d56100ada00eb82031fb854297fc9e389b7d303eba3af114e87bce2"
    );
    let config = reading::hex(&Sha256::digest(MLX_CONFIG));
    assert_eq!(file("config.json").unwrap().sha256, config);
    assert_eq!(file("config.json").unwrap().size_bytes, 937);
    assert!(file("README.md").is_none() && file(".gitattributes").is_none());
}

#[tokio::test]
async fn a_source_that_is_not_approved_is_refused_before_any_read() {
    let fetch = Recorded::default();
    let refused = read(&fetch, "someone/Qwen3-0.6B-GGUF", None).await;
    assert!(matches!(refused, Err(Error::Refused(Refusal::NotApproved))));
    assert_eq!(fetch.reads(), 0);
}

#[tokio::test]
async fn the_publisher_of_a_model_in_the_library_is_approved_for_it() {
    let fetch = Recorded::default();
    let refused = read(&fetch, "someone/Qwen3-0.6B-GGUF", Some("Qwen")).await;
    assert!(matches!(refused, Err(Error::Refused(Refusal::NotApproved))));
    let read = read(&fetch, "someone/Qwen3-0.6B-GGUF", Some("someone"))
        .await
        .unwrap();
    assert_eq!(read.facts.publisher.as_deref(), Some("someone"));
}

#[test]
fn names_have_the_ollama_form() {
    assert_eq!(naming::address("qwen3-0.6b", "q4_k_m"), "qwen3-0.6b:q4_k_m");
    assert_eq!(naming::model_part("qwen3-0.6b:q4_k_m"), "qwen3-0.6b");
    assert!(naming::is_name("coder"));
    assert!(!naming::is_name("Coder") && !naming::is_name("-x") && !naming::is_name(""));
    assert!(naming::repository("unsloth").is_err());
    assert!(naming::repository("unsloth/../x").is_err());
    assert!(naming::organisation("organisation", "a b").is_err());
}
