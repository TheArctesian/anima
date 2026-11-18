//! One session made of the tallies of its files (its own and its sub-agents'), and the session
//! days a batch sends of it (K8 Section 4).

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};

use super::{
    Day, MAX_COUNT, MAX_MODELS, ModelTokens, OTHER_MODEL, SessionDay, Source, Tally, Tokens,
    is_repository, is_session_id,
};

/// The counts of one session so far, from all its files.
#[derive(Clone, Debug)]
pub struct Session {
    source: Source,
    id: String,
    repository: Option<String>,
    days: BTreeMap<NaiveDate, Day>,
    span: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

impl Session {
    /// A session with no counts. A `repository` that breaks the K8 rule is not kept.
    #[must_use]
    pub fn new(source: Source, id: &str, repository: Option<String>) -> Self {
        Self {
            source,
            id: id.to_owned(),
            repository: repository.filter(|name| is_repository(name)),
            days: BTreeMap::new(),
            span: None,
        }
    }

    /// Adds the counts of one of the session's files.
    pub fn add(&mut self, tally: &Tally) {
        for (date, more) in tally.days() {
            let day = self.days.entry(*date).or_default();
            day.prompts = day.prompts.saturating_add(more.prompts).min(MAX_COUNT);
            day.tool_calls = day
                .tool_calls
                .saturating_add(more.tool_calls)
                .min(MAX_COUNT);
            for (model, tokens) in &more.models {
                let kept = day.models.entry(model.clone()).or_default();
                *kept = kept.plus(*tokens);
            }
        }
        if let Some((first, last)) = tally.span() {
            self.span = Some(match self.span {
                Some((start, end)) => (start.min(first), end.max(last)),
                None => (first, last),
            });
        }
    }

    /// The session's days, in date order; none when its ID breaks the K8 rule or no line had
    /// a time.
    #[must_use]
    pub fn session_days(&self) -> Vec<SessionDay> {
        let Some((started_at, ended_at)) = self.span.filter(|_| is_session_id(&self.id)) else {
            return Vec::new();
        };
        self.days
            .iter()
            .map(|(day, counts)| SessionDay {
                source: self.source,
                id: self.id.clone(),
                day: *day,
                started_at,
                ended_at,
                repository: self.repository.clone(),
                prompts: counts.prompts,
                tool_calls: counts.tool_calls,
                models: folded(&counts.models),
            })
            .collect()
    }
}

/// The models of a day, in name order, at most [`MAX_MODELS`]: past that, the models with the
/// fewest output tokens are added up under [`OTHER_MODEL`].
fn folded(models: &BTreeMap<String, Tokens>) -> Vec<ModelTokens> {
    let mut ranked: Vec<(&String, &Tokens)> = models.iter().collect();
    if ranked.len() > MAX_MODELS {
        ranked.sort_by(|a, b| b.1.output.cmp(&a.1.output).then_with(|| a.0.cmp(b.0)));
    }
    // The first 15 keep their names and the rest share `other`: at most 16 names.
    let mut kept: BTreeMap<String, Tokens> = BTreeMap::new();
    for (rank, (model, tokens)) in ranked.into_iter().enumerate() {
        let name = if rank < MAX_MODELS - 1 || models.len() <= MAX_MODELS {
            model.clone()
        } else {
            OTHER_MODEL.to_owned()
        };
        let entry = kept.entry(name).or_default();
        *entry = entry.plus(*tokens);
    }
    kept.into_iter()
        .map(|(model, tokens)| ModelTokens { model, tokens })
        .collect()
}
