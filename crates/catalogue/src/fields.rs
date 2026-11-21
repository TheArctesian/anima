//! The fields of an entry that need no other file: the identifier, the texts, the addresses
//! and the dates.

use compose_rules::forms::{is_name, is_target};

use crate::{
    Code,
    entry::{Anima, Class, Entry, MachineTest, Mark, Optional, Reviewed},
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
        MachineTest::Passed { date, .. } | MachineTest::Failed { date } => Some(date.as_str()),
    };
    let read = entry
        .anima
        .origin
        .as_ref()
        .map(|origin| origin.read.as_str());
    let reviewed = match &entry.anima.reviewed {
        Some(Reviewed::Reviewed { on, .. }) => Some(on.as_str()),
        Some(Reviewed::NotReviewed) | None => None,
    };
    let dates = std::iter::once(entry.anima.pinned.as_str())
        .chain(read)
        .chain(reviewed)
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
    codes.extend(parts(&entry.anima));
    codes
}

/// The words that name an agent, a session or a program, not a person.
const NOT_A_PERSON: [&str; 7] = [
    "agent",
    "claude",
    "automated",
    "session",
    "orchestrator",
    "subagent",
    "local_",
];
/// The shortest run of hexadecimal digits that is taken for the identifier of an agent.
const LONGEST_HEX: usize = 12;

/// True when `by` can be the name of a person: not empty, and with no word of an agent, a
/// session or a program, no word `bot`, and no long run of hexadecimal digits (SF-255).
fn is_person(by: &str) -> bool {
    let lower = by.to_lowercase();
    let mut run = 0;
    let longest_hex = lower.chars().fold(0, |longest, character| {
        run = if character.is_ascii_hexdigit() {
            run + 1
        } else {
            0
        };
        longest.max(run)
    });
    let bot = lower
        .split(|character: char| !character.is_alphanumeric())
        .any(|word| word == "bot" || word == "bots");
    !lower.trim().is_empty()
        && !NOT_A_PERSON.iter().any(|word| lower.contains(word))
        && !bot
        && longest_hex < LONGEST_HEX
}

/// The codes of the optional parts, of the class and of the review of `anima`.
fn parts(anima: &Anima) -> Vec<Code> {
    let mut codes = Vec::new();
    match &anima.reviewed {
        None if anima.origin.is_some() => codes.push(Code::NotReviewed),
        Some(Reviewed::Reviewed { by, .. }) if !is_person(by) => {
            codes.push(Code::ReviewerNotPermitted);
        }
        _ => {}
    }
    let devices = anima.optional.iter().all(|part| match part {
        Optional::Device { path } => path.starts_with("/dev/") && is_target(path),
    });
    if !devices {
        codes.push(Code::OptionalNotPermitted);
    }
    let on_network = anima
        .marks
        .iter()
        .any(|mark| matches!(mark, Mark::NetworkPort { .. }));
    if anima.class == Class::Shell && (anima.generated.is_empty() || on_network) {
        codes.push(Code::ShellNotContained);
    }
    codes
}
