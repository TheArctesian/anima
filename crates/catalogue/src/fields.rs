//! The fields of an entry that need no other file: the identifier, the texts, the addresses
//! and the dates.

use compose_rules::forms::is_name;

use crate::{
    Code,
    entry::{Entry, MachineTest},
};

/// The name of the Compose file in a recipe folder.
pub(crate) const COMPOSE: &str = "compose.yaml";
/// The name of the example environment file in a recipe folder. It does not start with
/// `.env`, so that no tool takes it for a file of secrets.
pub(crate) const ENVIRONMENT: &str = "environment.example";

/// A day as `YYYY-MM-DD`, with a month from 1 to 12 and a day from 1 to 31.
fn is_date(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let number = |part: &str, length: usize| {
        (part.len() == length && part.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| part.parse::<u32>().ok())
            .flatten()
    };
    match parts.as_slice() {
        [year, month, day] => {
            number(year, 4).is_some()
                && number(month, 2).is_some_and(|month| (1..=12).contains(&month))
                && number(day, 2).is_some_and(|day| (1..=31).contains(&day))
        }
        _ => false,
    }
}

fn is_https(address: &str) -> bool {
    address
        .strip_prefix("https://")
        .is_some_and(|rest| !rest.is_empty() && !rest.contains(char::is_whitespace))
}

/// The codes of the fields of `entry`, which is in the folder `folder`.
pub(crate) fn check(entry: &Entry, folder: &str) -> Vec<Code> {
    let mut codes = Vec::new();
    if !is_name(&entry.id) || entry.id != folder {
        codes.push(Code::IdNotPermitted);
    }
    let texts = [
        &entry.name,
        &entry.description,
        &entry.version,
        &entry.author,
    ];
    if texts.iter().any(|text| text.trim().is_empty())
        || entry.tags.is_empty()
        || entry.tags.iter().any(|tag| tag.trim().is_empty())
    {
        codes.push(Code::FieldEmpty);
    }
    let addresses = entry.compose_url == COMPOSE
        && entry.env_url == ENVIRONMENT
        && is_https(&entry.documentation_url)
        && entry.icon_url.as_deref().is_none_or(is_https);
    if !addresses {
        codes.push(Code::AddressNotPermitted);
    }
    let tested = match &entry.anima.last_test {
        MachineTest::NotTested => None,
        MachineTest::Passed { date } | MachineTest::Failed { date } => Some(date.as_str()),
    };
    let dates = std::iter::once(entry.anima.pinned.as_str())
        .chain(
            entry
                .anima
                .images
                .iter()
                .map(|image| image.resolved.as_str()),
        )
        .chain(tested);
    if !dates.into_iter().all(is_date) {
        codes.push(Code::DateNotPermitted);
    }
    codes
}
