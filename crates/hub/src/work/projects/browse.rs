//! The Projects list's sort and filters.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{Enum, ID, InputObject};

use crate::{api_errors::FieldError, ids::ProjectGroupId};

#[derive(Clone, Copy, Debug, Default, Enum, Eq, PartialEq)]
pub(super) enum ProjectOrder {
    #[default]
    Name,
    LastCommit,
    Created,
    CommitCount,
    Stars,
}

impl ProjectOrder {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::LastCommit => "commit",
            Self::Created => "created",
            Self::CommitCount => "count",
            Self::Stars => "stars",
        }
    }
}

#[derive(Default, InputObject)]
pub(super) struct ProjectBrowse {
    pub order: Option<ProjectOrder>,
    pub language: Option<String>,
    pub min_size_kib: Option<i32>,
    pub max_size_kib: Option<i32>,
    /// Only the projects of this group (PRO-061).
    pub group: Option<ID>,
    /// True: only the projects in no group (PRO-061).
    pub ungrouped: Option<bool>,
}

impl ProjectBrowse {
    pub fn validate(&self) -> Result<(), FieldError> {
        if self
            .language
            .as_ref()
            .is_some_and(|name| name.trim().is_empty() || name.len() > 128 || name.contains('\0'))
        {
            return Err(FieldError::new(
                "language",
                "Give a primary language of 1 to 128 bytes.",
            ));
        }
        if self.min_size_kib.is_some_and(|size| size < 0)
            || self.max_size_kib.is_some_and(|size| size < 0)
            || matches!((self.min_size_kib, self.max_size_kib), (Some(min), Some(max)) if min > max)
        {
            return Err(FieldError::new(
                "browse",
                "Use nonnegative repository sizes, with the minimum first.",
            ));
        }
        if self.group.is_some() && self.ungrouped == Some(true) {
            return Err(FieldError::new(
                "group",
                "Ask for one group or for the projects in no group, not both.",
            ));
        }
        if self
            .group
            .as_ref()
            .is_some_and(|id| id.parse::<ProjectGroupId>().is_err())
        {
            return Err(FieldError::new("group", "That id is not valid."));
        }
        Ok(())
    }

    /// The group that the list asks for, once [`ProjectBrowse::validate`] passed.
    pub fn group_id(&self) -> Option<ProjectGroupId> {
        self.group.as_ref().and_then(|id| id.parse().ok())
    }
}
