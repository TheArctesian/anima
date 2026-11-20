//! The repositories that this machine works for (K10 Section 3; R-185, SF-003): a list that the
//! owner of the machine sets at the machine. A machine with no list refuses each item that
//! names a repository.

use daemon_core::wire::WorkRepository;

/// The forges that this daemon knows (K10 Section 4).
pub const FORGES: [&str; 1] = ["github"];

/// A line of the list that cannot be read.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("line {line} of the list of repositories is not `owner` or `owner/name`")]
pub struct BadLine {
    /// The number of the line, from 1.
    pub line: usize,
}

/// One permitted owner, or one permitted repository of an owner.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Permitted {
    owner: String,
    name: Option<String>,
}

/// The list of the owner of the machine. An empty list permits nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Repositories(Vec<Permitted>);

/// Whether `owner` has only the characters that GitHub permits in the name of an account.
#[must_use]
pub fn is_owner(owner: &str) -> bool {
    (1..=39).contains(&owner.len())
        && owner
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// Whether `name` has only the characters that GitHub permits in the name of a repository.
#[must_use]
pub fn is_name(name: &str) -> bool {
    (1..=100).contains(&name.len())
        && name != "."
        && name != ".."
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

impl Repositories {
    /// Reads the list: one `owner` or `owner/name` in each line; a line that starts with `#`
    /// and an empty line are not entries. GitHub does not tell capital letters apart, so the
    /// list does not.
    ///
    /// # Errors
    /// [`BadLine`] for the first line that is not an entry: a list with a fault permits
    /// nothing, it is not read in part.
    pub fn parse(text: &str) -> Result<Self, BadLine> {
        let mut permitted = Vec::new();
        for (index, raw) in text.lines().enumerate() {
            let entry = raw.trim();
            if entry.is_empty() || entry.starts_with('#') {
                continue;
            }
            let (owner, name) = match entry.split_once('/') {
                Some((owner, name)) => (owner, Some(name)),
                None => (entry, None),
            };
            if !is_owner(owner) || !name.is_none_or(is_name) {
                return Err(BadLine { line: index + 1 });
            }
            permitted.push(Permitted {
                owner: owner.to_ascii_lowercase(),
                name: name.map(str::to_ascii_lowercase),
            });
        }
        Ok(Self(permitted))
    }

    /// Whether the machine works for `repository`: its forge is known, its two names have only
    /// permitted characters, and the list names its owner or the repository itself.
    #[must_use]
    pub fn permits(&self, repository: &WorkRepository) -> bool {
        let owner = repository.owner.to_ascii_lowercase();
        let name = repository.name.to_ascii_lowercase();
        FORGES.contains(&repository.forge.as_str())
            && is_owner(&repository.owner)
            && is_name(&repository.name)
            && self.0.iter().any(|each| {
                each.owner == owner
                    && each
                        .name
                        .as_ref()
                        .is_none_or(|permitted| *permitted == name)
            })
    }
}
