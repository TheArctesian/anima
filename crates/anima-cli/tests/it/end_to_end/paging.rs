//! Paging over the real hub's cursors (CMD-026): `--limit`, the next-page hint followed exactly
//! as printed, and `--all`, on more rows than one page.

use serde_json::Value;
use sqlx::PgPool;

use super::words;
use crate::kit::real_hub::RealHub;
use crate::kit::{Anima, Outcome};

/// The command line the next-page hint on standard error prints, after `anima`, as a shell
/// would read it; none on the last page.
fn next_page(outcome: &Outcome) -> Option<String> {
    let (_, hint) = outcome.stderr.split_once("Next page: ")?;
    let command = hint.split(" (or --all)").next().unwrap_or_default();
    let rest = command.strip_prefix("anima ");
    Some(
        rest.unwrap_or_else(|| panic!("not a command: {outcome:?}"))
            .to_owned(),
    )
}

/// The `name` of every node in a printed connection.
fn names(outcome: &Outcome) -> Vec<String> {
    assert_eq!(outcome.code, 0, "{outcome:?}");
    let page = outcome.json();
    let edges = page["edges"]
        .as_array()
        .unwrap_or_else(|| panic!("a page: {page}"));
    let name = |edge: &Value| edge["node"]["name"].as_str().map(str::to_owned);
    edges.iter().filter_map(name).collect()
}

/// Runs `anima <line>` through the shell, as a person pastes a hint, and the same with
/// `--json` for its rows.
async fn paste(anima: &Anima, line: &str) -> (Outcome, Vec<String>) {
    let bin = env!("CARGO_BIN_EXE_anima");
    let mut outcomes = Vec::new();
    for json in ["", " --json"] {
        let mut command = anima.command("/bin/sh");
        command.args(["-c", &format!("'{bin}' {line}{json}")]);
        let output = command.output().await.expect("the shell runs");
        outcomes.push(Outcome {
            code: output.status.code().expect("an exit code"),
            stdout: String::from_utf8(output.stdout).expect("UTF-8"),
            stderr: String::from_utf8(output.stderr).expect("UTF-8"),
        });
    }
    let rows = names(&outcomes[1]);
    (outcomes.swap_remove(0), rows)
}

/// Follows the printed hints from `first` to the last page; the names of each page.
async fn follow(anima: &Anima, first: &str) -> Vec<Vec<String>> {
    let (mut outcome, rows) = paste(anima, first).await;
    let mut pages = vec![rows];
    while let Some(line) = next_page(&outcome) {
        assert!(pages.len() < 10, "the hints never end: {line}");
        let (next, rows) = paste(anima, &line).await;
        assert_eq!(next.code, 0, "{next:?}");
        (outcome, pages) = (next, [pages, vec![rows]].concat());
    }
    pages
}

#[sqlx::test(migrations = "../hub/migrations")]
async fn cmd_026_device_pages_follow_the_printed_hint_and_all_joins_them(pool: PgPool) {
    let hub = RealHub::start(pool).await;
    let reader = hub.login_token().await;
    let anima = hub.anima().with("ANIMA_TOKEN", &reader);
    let site = anima
        .run(&words("site create home --subnet 10.10.0.0/24"))
        .await;
    assert_eq!(site.code, 0, "{site:?}");
    let all: Vec<String> = (1..=5).map(|n| format!("rack-{n}")).collect();
    for (n, name) in all.iter().enumerate() {
        let line = format!(
            "device create {name} --site home --host 10.10.0.{} --no-default-check",
            n + 10
        );
        let made = anima.run(&words(&line)).await;
        assert_eq!(made.code, 0, "{made:?}");
    }
    let other = "device create other --site home --host 10.10.0.99 --no-default-check";
    assert_eq!(anima.run(&words(other)).await.code, 0);

    let pages = follow(&anima, "device list --name rack --limit 2").await;
    assert_eq!(
        pages.iter().map(Vec::len).collect::<Vec<_>>(),
        [2, 2, 1],
        "{pages:?}"
    );
    assert_eq!(pages.concat(), all, "the hint keeps the filter");

    let joined = anima
        .run(&words("device list --name rack --limit 2 --all --json"))
        .await;
    assert_eq!(names(&joined), all);
    let table = anima
        .run(&words("device list --name rack --limit 2 --all"))
        .await;
    assert_eq!(next_page(&table), None, "{table:?}");
    let shown = all.iter().all(|name| table.stdout.contains(name.as_str()));
    assert!(shown, "{table:?}");
    let everything = anima
        .run(&words("device list --all --limit 4 --json"))
        .await;
    assert_eq!(names(&everything).len(), 6);
}
