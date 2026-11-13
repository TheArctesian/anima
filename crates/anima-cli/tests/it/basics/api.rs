//! `anima api`: a document from a file or standard input, and the hub's whole answer printed.

use serde_json::json;

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data, refusal};

#[tokio::test]
async fn cmd_003_api_sends_a_document_from_standard_input_and_prints_the_answer() {
    let hub = FakeHub::start().await;
    let answer = data(json!({ "tags": ["infra", "proxmox"] }));
    hub.answer("Tags", answer.clone());
    let anima = Anima::new().with("ANIMA_TOKEN", "a-login-token");
    let run = anima
        .run_with_input(&["api", "-", "--hub", &hub.origin], "query Tags { tags }")
        .await;
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.json(), answer);
    assert_eq!(hub.seen()[0].variables, json!({}));
}

#[tokio::test]
async fn cmd_003_api_takes_variables_from_a_file_and_an_error_sets_the_exit_code() {
    let hub = FakeHub::start().await;
    let answer = refusal("FORBIDDEN", "This token may only read.");
    hub.answer("Device", answer.clone());
    let folder = tempfile::tempdir().expect("a folder");
    let variables = folder.path().join("variables.json");
    std::fs::write(
        &variables,
        r#"{"id":"0190a000-0000-7000-8002-000000000001"}"#,
    )
    .expect("write");
    let document = folder.path().join("device.graphql");
    std::fs::write(
        &document,
        "query Device($id: ID!) { device(id: $id) { name } }",
    )
    .expect("write");
    let anima = Anima::new().with("ANIMA_TOKEN", "a-login-token");
    let args = [
        "api",
        document.to_str().expect("path"),
        "--variables",
        variables.to_str().expect("path"),
        "--hub",
        &hub.origin,
    ];
    let run = anima.run(&args).await;
    assert_eq!(run.code, 5, "{run:?}");
    assert_eq!(
        run.json(),
        answer,
        "the raw answer is printed even when it is an error"
    );
    assert_eq!(
        hub.seen()[0].variables["id"],
        "0190a000-0000-7000-8002-000000000001"
    );

    let run = anima
        .run_with_input(&["api", "-", "--variables", "-", "--hub", &hub.origin], "")
        .await;
    assert_eq!(run.code, 2, "only one input can be standard input: {run:?}");
}

#[tokio::test]
async fn cmd_022_c8_bad_variables_json_is_exit_2_like_every_other_bad_document() {
    let hub = FakeHub::start().await;
    let folder = tempfile::tempdir().expect("a folder");
    let anima = Anima::new().with("ANIMA_TOKEN", "a-login-token");
    for (text, why) in [(r#"{"id": "#, "broken"), ("[1, 2]", "not an object")] {
        let variables = folder.path().join("variables.json");
        std::fs::write(&variables, text).expect("write");
        let run = anima
            .run_with_input(
                &[
                    "api",
                    "-",
                    "--variables",
                    variables.to_str().expect("path"),
                    "--hub",
                    &hub.origin,
                ],
                "query Tags { tags }",
            )
            .await;
        assert_eq!(run.code, 2, "{why}: {run:?}");
    }
    assert!(hub.seen().is_empty(), "nothing was sent");
}
