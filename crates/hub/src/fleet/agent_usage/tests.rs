//! The pure part of the agent usage: the estimate of the cost from the price table (AIS-021),
//! and the fold of the sums by model into the groups asked for (AIS-020).

use chrono::NaiveDate;
use daemon_core::agent_usage::Source;

use super::{
    Dimension, DimensionSet,
    grouping::fold,
    model::{Grain, GroupKey, Sums, TokenSums},
    prices::{PRICES_AS_OF, estimate},
};

fn tokens(input: f64, output: f64, cache_read: f64, cache_write: f64) -> TokenSums {
    TokenSums {
        input,
        output,
        cache_read,
        cache_write,
    }
}

fn assert_close(actual: Option<f64>, expected: f64) {
    let actual = actual.unwrap_or_else(|| panic!("no estimate, expected {expected}"));
    assert!(
        (actual - expected).abs() < 1e-9,
        "{actual} is not {expected}"
    );
}

#[test]
fn ais_021_a_known_model_is_priced_per_million_tokens_of_each_kind() {
    let million = tokens(1e6, 1e6, 1e6, 1e6);
    // Opus 4.5: 5 input, 25 output, 0.50 cache read, 6.25 cache write.
    assert_close(estimate("claude-opus-4-5", million), 36.75);
    // Sonnet 4.5: 3, 15, 0.30, 3.75.
    assert_close(estimate("claude-sonnet-4-5", million), 22.05);
    // GPT-5: 1.25 input, 10 output, 0.125 cached; a cache write is input.
    assert_close(estimate("gpt-5-codex", million), 12.625);
    assert_close(
        estimate("claude-haiku-4-5", tokens(2e6, 0.0, 0.0, 0.0)),
        2.0,
    );
}

#[test]
fn ais_021_a_dated_model_name_has_the_price_of_its_name() {
    let few = tokens(1000.0, 2000.0, 0.0, 0.0);
    assert_eq!(
        estimate("claude-sonnet-4-5-20250929", few),
        estimate("claude-sonnet-4-5", few)
    );
    assert_eq!(estimate("gpt-5-2025-08-07", few), estimate("gpt-5", few));
    assert_eq!(
        estimate("claude-sonnet-4-20250514", few),
        estimate("claude-sonnet-4", few),
        "Sonnet 4 is not Sonnet 4.5"
    );
}

#[test]
fn ais_021_an_unknown_model_gives_no_estimate_never_a_guess() {
    let few = tokens(1000.0, 2000.0, 0.0, 0.0);
    for model in [
        "claude-opus-5-5",
        "other",
        "claude-opus",
        "gpt-5-codex-max",
        "claude-sonnet-4-5-2025",
        "claude-sonnet-4-5-99999999x",
    ] {
        assert_eq!(estimate(model, few), None, "{model}");
    }
}

#[test]
fn ais_021_the_price_table_names_its_day() {
    assert!(NaiveDate::parse_from_str(PRICES_AS_OF, "%Y-%m-%d").is_ok());
}

fn grain(source: Source, model: Option<&str>, sums: Sums) -> Grain {
    Grain {
        key: GroupKey {
            source: Some(source),
            model: model.map(str::to_owned),
            ..GroupKey::default()
        },
        sums,
    }
}

fn activity(sessions: i64, prompts: f64) -> Sums {
    Sums {
        sessions,
        prompts,
        ..Sums::default()
    }
}

fn used(tokens: TokenSums) -> Sums {
    Sums {
        tokens,
        ..Sums::default()
    }
}

#[test]
fn ais_020_folding_the_models_adds_their_counts_and_their_estimates() {
    let grains = vec![
        grain(
            Source::ClaudeCode,
            Some("claude-opus-4-5"),
            activity(2, 6.0),
        ),
        grain(
            Source::ClaudeCode,
            Some("claude-opus-4-5"),
            used(tokens(1e6, 0.0, 0.0, 0.0)),
        ),
        grain(
            Source::ClaudeCode,
            Some("claude-haiku-4-5"),
            used(tokens(1e6, 0.0, 0.0, 0.0)),
        ),
        grain(
            Source::ClaudeCode,
            Some("claude-new-1"),
            used(tokens(500.0, 250.0, 0.0, 0.0)),
        ),
        grain(Source::Codex, None, activity(1, 3.0)),
    ];
    let by_source = DimensionSet::of(&[Dimension::Source]);
    let mut groups = fold(grains.clone(), by_source);
    groups.sort_by_key(|group| group.key.source);
    assert_eq!(groups.len(), 2);
    let claude = &groups[0];
    assert_eq!(claude.key.model, None, "not grouped by model");
    assert_eq!((claude.sums.sessions, claude.sums.prompts), (2, 6.0));
    assert_close(claude.estimated_cost_usd, 6.0);
    assert!((claude.unpriced_tokens - 750.0).abs() < 1e-9);
    let codex = &groups[1];
    assert_eq!(
        (codex.estimated_cost_usd, codex.unpriced_tokens),
        (None, 0.0),
        "no tokens: no estimate"
    );

    let by_model = DimensionSet::of(&[Dimension::Model, Dimension::Model]);
    let groups = fold(grains, by_model);
    assert_eq!(groups.len(), 4, "opus, haiku, the unknown model and none");
    let unknown = groups
        .iter()
        .find(|group| group.key.model.as_deref() == Some("claude-new-1"))
        .unwrap();
    assert_eq!(unknown.estimated_cost_usd, None);
}
