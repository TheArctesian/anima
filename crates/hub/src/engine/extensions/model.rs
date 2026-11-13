//! What the registry lists: keys, data classes, navigation and the entry itself (K-E7).
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;

use super::registry::registry;

/// The stable identifier of an extension, one of the registry's keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Key(&'static str);

impl Key {
    pub(super) const fn new(key: &'static str) -> Self {
        Self(key)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }

    /// The registry's key with this text, if there is one.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        registry()
            .iter()
            .map(|entry| entry.key)
            .find(|key| key.0 == text)
    }

    /// The registry entry of this key.
    ///
    /// # Panics
    /// Never: a `Key` is made only from the registry.
    #[must_use]
    pub fn entry(self) -> &'static Entry {
        registry()
            .iter()
            .find(|entry| entry.key == self)
            .expect("a Key is made from the registry")
    }

    /// The operator's name for it, for example `Projects`.
    #[must_use]
    pub fn name(self) -> &'static str {
        self.entry().name
    }
}

/// How sensitive the data of an extension is (EXT-004).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum DataClass {
    A,
    B,
    C,
}

/// The navigation group an extension appears in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum NavigationGroup {
    Infrastructure,
    Life,
    Belongings,
    Vault,
    Work,
    Bottom,
}

/// Where the client shows an extension while it is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Navigation {
    pub group: NavigationGroup,
    pub label: &'static str,
    pub route: &'static str,
}

/// One extension the hub knows about.
#[derive(Debug)]
pub struct Entry {
    pub key: Key,
    pub name: &'static str,
    pub data_class: DataClass,
    pub needs: &'static [Key],
    pub works_with: &'static [Key],
    pub navigation: Option<Navigation>,
    /// The K-E8 migration range, if it has tables.
    pub migrations: Option<(u16, u16)>,
    pub built: bool,
    /// Part of the engine: always on, no switch.
    pub fixed: bool,
    /// The audit target of its switch, `extensions.<key>`.
    pub setting: &'static str,
}
