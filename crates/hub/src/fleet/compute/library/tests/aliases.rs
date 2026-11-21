//! Aliases and approved sources in the database (LIBM-003, LIBM-007, SEL-003).

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::Recorded;
use crate::fleet::compute::library::{self, Error};

fn now() -> DateTime<Utc> {
    "2026-11-20T09:00:00Z".parse().unwrap()
}

async fn add(pool: &PgPool, repository: &str, name: &str) {
    let read = library::read(pool, &Recorded::default(), repository, Some(name))
        .await
        .unwrap();
    let mut conn = pool.acquire().await.unwrap();
    library::save(&mut conn, &read, now()).await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn an_alias_moves_to_a_different_model(pool: PgPool) {
    add(&pool, "unsloth/Qwen3-0.6B-GGUF", "small").await;
    add(&pool, "mlx-community/Qwen3-0.6B-4bit", "small-mlx").await;
    let mut conn = pool.acquire().await.unwrap();
    let (alias, before) = library::set_alias(&mut conn, "fast", "small")
        .await
        .unwrap();
    assert_eq!((alias.model.as_str(), before), ("small", None));
    assert_eq!(library::get(&pool, "fast").await.unwrap().name, "small");
    let (_, before) = library::set_alias(&mut conn, "fast", "small-mlx:4bit")
        .await
        .unwrap();
    assert_eq!(before.as_deref(), Some("small"));
    let model = library::get(&pool, "fast").await.unwrap();
    assert_eq!(
        (model.name.as_str(), model.aliases.as_slice()),
        ("small-mlx", &["fast".to_owned()][..])
    );
    let refused = library::set_alias(&mut conn, "small", "small-mlx").await;
    assert!(matches!(refused, Err(Error::Validation(_))));
}

#[sqlx::test(migrations = "./migrations")]
async fn the_stored_model_keeps_its_files_and_values(pool: PgPool) {
    add(&pool, "unsloth/Qwen3-0.6B-GGUF", "small").await;
    let model = library::get(&pool, "small:q4_k_m").await.unwrap();
    assert_eq!(model.facts.context_length, Some(40_960));
    assert_eq!(
        model.repositories(),
        vec!["unsloth/Qwen3-0.6B-GGUF".to_owned()]
    );
    let q4 = model
        .variants
        .iter()
        .find(|each| each.tag == "q4_k_m")
        .unwrap();
    assert_eq!(
        q4.files[0].sha256,
        "ac2d97712095a558e31573f62f466a3f9d93990898b0ec79d7c974c1780d524a"
    );
    assert_eq!(library::list(&pool).await.unwrap().len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn the_operator_adds_an_approved_source(pool: PgPool) {
    let refused = library::read(&pool, &Recorded::default(), "someone/Qwen3-0.6B-GGUF", None).await;
    assert!(matches!(refused, Err(Error::Refused(_))));
    let mut conn = pool.acquire().await.unwrap();
    assert!(
        library::add_source(&mut conn, "someone", now())
            .await
            .unwrap()
    );
    assert!(
        !library::add_source(&mut conn, "SOMEONE", now())
            .await
            .unwrap()
    );
    assert_eq!(
        library::sources(&pool).await.unwrap(),
        vec![
            "mlx-community".to_owned(),
            "someone".to_owned(),
            "unsloth".to_owned()
        ]
    );
    assert!(
        library::read(&pool, &Recorded::default(), "someone/Qwen3-0.6B-GGUF", None)
            .await
            .is_ok()
    );
}
