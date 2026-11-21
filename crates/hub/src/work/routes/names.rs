//! The rules of a public name of an application (Section 12.3 and 12.4 of the design): a DNS
//! name in lower case that is never the name of the hub, never below it, and never at or below
//! a domain that holds a cookie of a hub (AC-D45, SF-NEW-routetoken). Pure.

use std::fmt;

/// The longest DNS name in its text form (RFC 1035).
const NAME_MAX: usize = 253;
/// The longest label.
const LABEL_MAX: usize = 63;

/// A public name that passed [`RouteName::parse`]: lower case, ASCII, at least two labels.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RouteName(String);

/// What a name must not touch: the name of the hub and the domains that hold a cookie of a hub.
#[derive(Clone, Debug, Default)]
pub struct NameRules {
    /// The host of the hub's public origin, in lower case.
    pub hub: String,
    /// Each domain whose cookies a hub sets or reads, in lower case.
    pub cookie_domains: Vec<String>,
}

/// Why a name is refused. Holds no part of the name (LOG-007).
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum NameRefusal {
    #[error("a name is a DNS name of 1 to 253 characters")]
    Length,
    #[error(
        "a name has at least two labels of 1 to 63 letters, digits or hyphens, with no hyphen at the start or the end of a label"
    )]
    Form,
    #[error("a name with a wildcard is not accepted")]
    Wildcard,
    #[error("a name is a host name, not an address, a path or a port")]
    NotAHost,
    #[error("a name is never the name of the hub or a name below it")]
    Hub,
    #[error("a name is never at or below a domain that holds a cookie of a hub")]
    CookieDomain,
}

impl RouteName {
    /// Checks `text` and gives it in lower case. Surrounding white space is removed; a final
    /// dot is removed.
    ///
    /// # Errors
    /// The first [`NameRefusal`] that applies.
    pub fn parse(text: &str, rules: &NameRules) -> Result<Self, NameRefusal> {
        let text = text.trim();
        let text = text.strip_suffix('.').unwrap_or(text).to_ascii_lowercase();
        if text.is_empty() || text.len() > NAME_MAX {
            return Err(NameRefusal::Length);
        }
        if text.contains('*') {
            return Err(NameRefusal::Wildcard);
        }
        if text.contains(['/', ':', '@', '?', '#', '[', ']']) {
            return Err(NameRefusal::NotAHost);
        }
        let labels: Vec<&str> = text.split('.').collect();
        if labels.len() < 2 || !labels.iter().all(|label| good_label(label)) {
            return Err(NameRefusal::Form);
        }
        // A last label of digits only is an address, as `10.0.0.1` is.
        if labels
            .last()
            .is_some_and(|last| last.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(NameRefusal::NotAHost);
        }
        if at_or_below(&text, &rules.hub) {
            return Err(NameRefusal::Hub);
        }
        if rules
            .cookie_domains
            .iter()
            .any(|domain| at_or_below(&text, domain))
        {
            return Err(NameRefusal::CookieDomain);
        }
        Ok(Self(text))
    }

    /// The name as stored, from a row that passed [`RouteName::parse`] before.
    pub(super) const fn stored(text: String) -> Self {
        Self(text)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The name and each domain above it, from the name up, without the last label alone:
    /// `a.b.example.com`, `b.example.com`, `example.com`. The order in which CAA is read.
    #[must_use]
    pub fn with_parents(&self) -> Vec<&str> {
        let mut found = vec![self.0.as_str()];
        let mut rest = self.0.as_str();
        while let Some((_, parent)) = rest.split_once('.') {
            if !parent.contains('.') {
                break;
            }
            found.push(parent);
            rest = parent;
        }
        found
    }

    /// Whether this name and the hub share the domain above the hub's name (`hub.example.com`
    /// and `photos.example.com`): then the cookie attribute `SameSite` does not stop a request
    /// from the page of the application (SF-NEW-routeabuse). Without the Public Suffix List
    /// the parent of the hub stands for its registered domain.
    #[must_use]
    pub fn same_site_as(&self, hub: &str) -> bool {
        hub.split_once('.')
            .map(|(_, parent)| parent)
            .filter(|parent| parent.contains('.'))
            .is_some_and(|parent| at_or_below(&self.0, parent))
    }
}

impl fmt::Display for RouteName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One label: letters, digits and hyphens, no hyphen at an end.
fn good_label(label: &str) -> bool {
    (1..=LABEL_MAX).contains(&label.len())
        && label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && !label.starts_with('-')
        && !label.ends_with('-')
}

/// Whether `name` is `domain` or a name below it. An empty domain holds nothing.
fn at_or_below(name: &str, domain: &str) -> bool {
    let domain = domain.trim_end_matches('.');
    !domain.is_empty()
        && (name == domain
            || name
                .strip_suffix(domain)
                .is_some_and(|head| head.ends_with('.')))
}
