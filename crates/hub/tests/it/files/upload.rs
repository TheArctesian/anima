//! Uploads and downloads (FIL-001, FIL-003, FIL-005).

use anima_hub::engine::files::{MAX_FILE, Store};
use axum::{
    body::{Body, Bytes},
    http::StatusCode,
};
use futures_util::stream;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{BOOK, FileHub, count, photo, read_key, sha256};

const QUERY: &str = "owner=book:isbn-9780141439518&name=cover.jpg&key=engine";
const QUERY_BIN: &str = "owner=book:isbn-9780141439518&name=pages.bin&key=engine";

#[sqlx::test(migrations = "./migrations")]
async fn fil_001_the_same_bytes_are_stored_once_for_each_key(pool: PgPool) {
    let hub = FileHub::new(&pool).await;
    let bytes = b"the same bytes".repeat(10);

    let first = hub.upload(QUERY, &bytes).await;
    let again = hub.upload(QUERY, &bytes).await;
    let other_key = hub
        .upload("owner=book:2&name=cover.jpg&key=wardrobe", &bytes)
        .await;

    assert_eq!(first["hash"], sha256(&bytes));
    assert_eq!(first["hash"], again["hash"]);
    assert_eq!(first["hash"], other_key["hash"]);
    assert_ne!(first["id"], again["id"]);
    assert_eq!(count(&pool, "attachment").await, 3);
    assert_eq!(count(&pool, "piece").await, 2);
    let pieces: Vec<_> = hub
        .stored_files()
        .into_iter()
        .filter(|path| !path.to_string_lossy().contains(".staging"))
        .collect();
    assert_eq!(pieces.len(), 2, "{pieces:?}");
    let hash = sha256(&bytes);
    assert!(pieces[0].ends_with(format!("engine/{}/{hash}", &hash[..2])));
    let stored = std::fs::read(&pieces[0]).unwrap();
    assert!(
        !stored.windows(14).any(|w| w == b"the same bytes"),
        "the piece is sealed"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn fil_005_a_body_over_256_mb_is_refused_while_it_streams_and_leaves_no_piece(pool: PgPool) {
    let hub = FileHub::new(&pool).await;
    // 4 097 parts of 64 KiB, made one at a time: one byte more than the limit after the
    // 4 096th, and never the whole body in memory. No length is declared.
    let part = Bytes::from(vec![7_u8; 64 * 1024]);
    let parts = MAX_FILE / (64 * 1024) + 1;
    let body = stream::iter((0..parts).map(move |_| Ok::<_, std::io::Error>(part.clone())));

    let (status, answer) = hub
        .upload_as(&hub.token.clone(), QUERY, Body::from_stream(body))
        .await;

    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE, "{answer}");
    assert_eq!(count(&pool, "piece").await, 0);
    assert_eq!(count(&pool, "attachment").await, 0);
    assert_eq!(hub.stored_files(), Vec::<std::path::PathBuf>::new());
}

#[sqlx::test(migrations = "./migrations")]
async fn fil_005_a_declared_length_over_256_mb_is_refused_before_the_body_is_read(pool: PgPool) {
    let hub = FileHub::new(&pool).await;
    let request = axum::http::Request::post(format!("/files/v1/attachments?{QUERY}"))
        .header("authorization", format!("Bearer {}", hub.token))
        .header("content-length", (MAX_FILE + 1).to_string())
        .body(Body::from_stream(stream::pending::<
            Result<Bytes, std::io::Error>,
        >()))
        .unwrap();

    let (status, _, _) = hub.send(request).await;

    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[sqlx::test(migrations = "./migrations")]
async fn fil_003_a_jpeg_is_sent_without_gps_and_the_stored_piece_does_not_change(pool: PgPool) {
    let hub = FileHub::new(&pool).await;
    let original = photo::photo();
    let made = hub.upload(QUERY, &original).await;
    let piece = hub.stored_files().pop().unwrap();
    let sealed = std::fs::read(&piece).unwrap();

    let (status, headers, sent) = hub.download(made["id"].as_str().unwrap()).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["anima-location-removed"], "true");
    assert_eq!(headers["content-type"], "image/jpeg");
    assert_eq!(
        headers["content-disposition"],
        "attachment; filename*=UTF-8''cover.jpg"
    );
    assert_eq!(headers["x-content-type-options"], "nosniff");
    assert_eq!(sent.len(), original.len());
    assert!(
        !sent.windows(48).any(|w| w == photo::gps_rationals()),
        "the GPS values are gone"
    );
    assert_eq!(
        sent[sent.len() - 70_002..],
        original[original.len() - 70_002..]
    );
    assert_eq!(
        made["hash"],
        sha256(&original),
        "the store keeps the bytes as sent"
    );
    assert_eq!(
        std::fs::read(&piece).unwrap(),
        sealed,
        "the piece does not change"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn fil_003_a_file_that_is_not_a_jpeg_is_sent_as_it_is(pool: PgPool) {
    let hub = FileHub::new(&pool).await;
    let made = hub.upload(QUERY, b"%PDF-1.7 not a photo").await;

    let (status, headers, sent) = hub.download(made["id"].as_str().unwrap()).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["anima-location-removed"], "false");
    assert_eq!(sent, b"%PDF-1.7 not a photo");
}

#[sqlx::test(migrations = "./migrations")]
async fn fil_003_keep_location_keeps_the_position_only_when_asked(pool: PgPool) {
    let hub = FileHub::new(&pool).await;
    let original = photo::photo();

    let kept = hub
        .upload(&format!("{QUERY}&keep_location=true"), &original)
        .await;
    let not_kept = hub.upload(QUERY, &original).await;

    assert!(
        (kept["latitude"].as_f64().unwrap() - 52.5).abs() < 1e-9,
        "{kept}"
    );
    assert!(
        (kept["longitude"].as_f64().unwrap() + 4.25).abs() < 1e-9,
        "{kept}"
    );
    assert_eq!(
        (&not_kept["latitude"], &not_kept["longitude"]),
        (&Value::Null, &Value::Null)
    );
    let listed = hub
        .post(
            "query($owner: String!) { attachments(owner: $owner) { id latitude longitude } }",
            json!({ "owner": BOOK }),
        )
        .await;
    let listed = listed["data"]["attachments"].as_array().unwrap().clone();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[1]["id"], kept["id"]);
    assert!(listed[1]["latitude"].is_f64());
    assert_eq!(listed[0]["latitude"], Value::Null);
}

#[sqlx::test(migrations = "./migrations")]
async fn fil_001_a_read_key_cannot_upload_and_no_token_gets_401(pool: PgPool) {
    let hub = FileHub::new(&pool).await;
    let reader = read_key(&pool).await;

    let (read_status, _) = hub.upload_as(&reader, QUERY, Body::from("x")).await;
    let (no_status, _) = hub.upload_as("not-a-token", QUERY, Body::from("x")).await;

    assert_eq!(read_status, StatusCode::FORBIDDEN);
    assert_eq!(no_status, StatusCode::UNAUTHORIZED);
    assert_eq!(count(&pool, "attachment").await, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn fil_001_without_a_file_store_an_upload_is_unavailable_and_the_hub_serves(pool: PgPool) {
    let folders = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let hub = FileHub::with_store(&pool, Store::off(), folders).await;

    let (status, answer) = hub
        .upload_as(&hub.token.clone(), QUERY, Body::from("x"))
        .await;
    let listed = hub
        .post("query { attachments(owner: \"book:1\") { id } }", json!({}))
        .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(answer["code"], "UNAVAILABLE");
    assert_eq!(listed["data"]["attachments"], json!([]));
}

#[sqlx::test(migrations = "./migrations")]
async fn fil_001_a_bad_owner_name_or_key_is_refused_by_field(pool: PgPool) {
    let hub = FileHub::new(&pool).await;

    let (status, answer) = hub
        .upload_as(
            &hub.token.clone(),
            "owner=nothing&name=%0A&key=..%2Fvault",
            Body::from("x"),
        )
        .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    let fields: Vec<&str> = answer["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|bad| bad["field"].as_str().unwrap())
        .collect();
    assert_eq!(fields, ["owner", "name", "key"]);
    assert!(!answer.to_string().contains("vault"), "no value is echoed");
}

#[sqlx::test(migrations = "./migrations")]
async fn fil_003_a_jpeg_with_broken_segments_is_sent_as_it_is(pool: PgPool) {
    let hub = FileHub::new(&pool).await;
    // The start of a JPEG, then a segment whose length says 1: no walk of the segments.
    let broken = [0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x01, 0x47, 0x50, 0x53];
    let cut = &photo::photo()[..40];

    for original in [broken.as_slice(), cut] {
        let made = hub.upload(QUERY, original).await;
        let (status, headers, sent) = hub.download(made["id"].as_str().unwrap()).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers["anima-location-removed"], "false");
        assert_eq!(sent, original, "the whole file arrives");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn fil_002_a_file_of_each_part_boundary_comes_back_as_it_was_sent(pool: PgPool) {
    const PART: usize = 64 * 1024;
    let hub = FileHub::new(&pool).await;

    for size in [0, 1, PART - 1, PART, PART + 1, 3 * PART, 3 * PART + 17] {
        let original: Vec<u8> = (0..=250_u8).cycle().take(size).collect();
        let (status, made) = hub
            .upload_as(&hub.token.clone(), QUERY_BIN, Body::from(original.clone()))
            .await;
        assert_eq!(status, StatusCode::CREATED, "{size} bytes");
        assert_eq!(made["hash"], sha256(&original), "{size} bytes");

        let (status, _, sent) = hub.download(made["id"].as_str().unwrap()).await;

        assert_eq!(status, StatusCode::OK, "{size} bytes");
        assert!(sent == original, "{size} bytes come back");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn fil_002_a_stored_piece_that_is_cut_short_or_reordered_is_not_sent(pool: PgPool) {
    const PART: usize = 64 * 1024;
    let hub = FileHub::new(&pool).await;
    let original: Vec<u8> = (0..=250_u8).cycle().take(3 * PART).collect();
    let made = hub.upload(QUERY_BIN, &original).await;
    let piece = hub.stored_files().pop().unwrap();
    let sealed = std::fs::read(&piece).unwrap();
    let stride = sealed.len() / 3;
    // Two parts change places: the same sealed bytes, in a different order.
    let mut reordered = sealed[..64].to_vec();
    reordered.extend_from_slice(&sealed[64 + stride..64 + 2 * stride]);
    reordered.extend_from_slice(&sealed[64..64 + stride]);
    reordered.extend_from_slice(&sealed[64 + 2 * stride..]);

    for damaged in [&sealed[..sealed.len() - stride], reordered.as_slice()] {
        std::fs::write(&piece, damaged).unwrap();

        let answer = hub.try_download(made["id"].as_str().unwrap()).await;

        // Either a refusal, or a body that ends with an error before the file is whole.
        if let Some((status, sent)) = answer {
            assert_ne!(status, StatusCode::OK, "{} bytes were sent", sent.len());
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn vlt_001_the_hub_does_not_seal_a_file_with_the_key_of_a_class_a_extension(pool: PgPool) {
    let hub = FileHub::new(&pool).await;

    let (status, answer) = hub
        .upload_as(
            &hub.token.clone(),
            "owner=document:1&name=passport.jpg&key=documents",
            Body::from("x"),
        )
        .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(answer["fields"][0]["field"], "key");
    assert_eq!(hub.stored_files().len(), 0);
}
