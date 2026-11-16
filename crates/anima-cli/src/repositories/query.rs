//! One page of commits, branches or releases, asked the same way by the command and by the
//! MCP tool.

use super::documents::{
    BranchList, CommitList, CommitListChecked, RepositoryReleaseList, branch_list, commit_list,
    commit_list_checked, repository_release_list,
};
use crate::forge::rows::Rows;
use crate::hub::{Hub, HubError};

/// Which part of a repository's history is read.
#[derive(Debug)]
pub enum History {
    /// `commits` from `reference` (a branch or a commit; none is the default branch); `checked`
    /// adds the checks of each commit, which cost two GitHub requests each (PRJ-036).
    Commits {
        reference: Option<String>,
        checked: bool,
    },
    Branches,
    Releases,
}

/// What a history root is asked.
#[derive(Debug)]
pub struct Query {
    pub repository: String,
    pub history: History,
    pub first: i64,
    pub after: Option<String>,
    pub fresh: bool,
}

impl Query {
    /// The page the hub reads from GitHub.
    ///
    /// # Errors
    /// [`HubError`] for a refusal or a failed request.
    pub async fn run(&self, hub: &Hub) -> Result<Rows, HubError> {
        let (repository, first) = (self.repository.clone(), Some(self.first));
        let (after, fresh) = (self.after.clone(), Some(self.fresh));
        match &self.history {
            History::Commits {
                reference,
                checked: true,
            } => {
                let variables = commit_list_checked::Variables {
                    repository,
                    ref_: reference.clone(),
                    first,
                    after,
                    fresh,
                };
                let page = hub.run::<CommitListChecked>(variables).await?;
                Ok(Rows::new(&page.commits))
            }
            History::Commits {
                reference,
                checked: false,
            } => {
                let variables = commit_list::Variables {
                    repository,
                    ref_: reference.clone(),
                    first,
                    after,
                    fresh,
                };
                let page = hub.run::<CommitList>(variables).await?;
                Ok(Rows::new(&page.commits))
            }
            History::Branches => {
                let variables = branch_list::Variables {
                    repository,
                    first,
                    after,
                    fresh,
                };
                let page = hub.run::<BranchList>(variables).await?;
                Ok(Rows::new(&page.branches))
            }
            History::Releases => {
                let variables = repository_release_list::Variables {
                    repository,
                    first,
                    after,
                    fresh,
                };
                let page = hub.run::<RepositoryReleaseList>(variables).await?;
                Ok(Rows::new(&page.repository_releases))
            }
        }
    }
}
