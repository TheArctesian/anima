//! The builder's surface: the root fields `access` lets a builder token select, the size of a
//! live update and the builder name (BLD-022, BLD-081, D-1).

use async_graphql::{EmptySubscription, Schema, SimpleObject};

use super::{BUILDER, builder, queued_job};
use crate::{
    access::BUILDER_FIELDS,
    builds::{
        builders,
        graphql::BuilderMutation,
        model::{JobUpdate, LogLine},
        updates::{MAX_UPDATE_LINE_BYTES, bounded},
    },
};

#[derive(Default, SimpleObject)]
struct Query {
    ready: bool,
}

#[tokio::test]
async fn d_1_the_builder_fields_access_allows_are_the_builder_mutations() {
    let schema = Schema::build(Query::default(), BuilderMutation, EmptySubscription).finish();
    let response = schema
        .execute("{ __schema { mutationType { fields { name } } } }")
        .await;
    assert!(response.errors.is_empty(), "{:?}", response.errors);
    let data = response.data.into_json().unwrap();
    let mut served: Vec<String> = data["__schema"]["mutationType"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| field["name"].as_str().unwrap().to_owned())
        .collect();
    served.sort();
    let mut allowed = BUILDER_FIELDS.map(str::to_owned).to_vec();
    allowed.sort();
    assert_eq!(served, allowed);
}

fn update(lines: usize, bytes: usize) -> JobUpdate {
    JobUpdate {
        job: crate::builds::queue::claim(&queued_job(), &builder(BUILDER), super::start()),
        lines: (1..=lines)
            .map(|sequence| LogLine {
                sequence: i64::try_from(sequence).unwrap(),
                text: "x".repeat(bytes),
            })
            .collect(),
    }
}

#[test]
fn bld_081_a_live_update_carries_only_its_newest_lines_up_to_64_kib() {
    let sent = bounded(update(1000, 4096));
    let total: usize = sent.lines.iter().map(|line| line.text.len()).sum();
    assert!(total <= MAX_UPDATE_LINE_BYTES, "{total}");
    assert_eq!(sent.lines.len(), MAX_UPDATE_LINE_BYTES / 4096);
    assert_eq!(sent.lines.last().unwrap().sequence, 1000);
    assert_eq!(bounded(update(3, 10)).lines.len(), 3);
}

#[test]
fn bld_021_a_builder_name_has_no_control_or_format_character() {
    for bad in [
        "",
        "  ",
        "pop\u{202e}so",
        "pop\u{200b}os",
        "pop\u{7}os",
        &"x".repeat(65),
    ] {
        assert_eq!(builders::name(bad).unwrap_err().field, "name", "{bad:?}");
    }
    assert_eq!(builders::name("  pop-os ").unwrap(), "pop-os");
}
