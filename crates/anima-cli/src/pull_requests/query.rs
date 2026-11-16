//! One page of pull requests, asked the same way by the command and by the MCP tool.

use super::documents::{
    PullRequestList, PullRequestListChecked, pull_request_list, pull_request_list_checked,
};
use crate::forge::StateArg;
use crate::forge::rows::Rows;
use crate::hub::{Hub, HubError};

/// What `pullRequests` is asked: `checked` adds mergeability, checks and merge methods, which
/// cost GitHub requests (PRJ-033).
#[derive(Debug)]
pub struct Query {
    pub repository: String,
    pub state: StateArg,
    pub first: i64,
    pub after: Option<String>,
    pub fresh: bool,
    pub checked: bool,
}

impl Query {
    /// The page the hub reads from GitHub.
    ///
    /// # Errors
    /// [`HubError`] for a refusal or a failed request.
    pub async fn run(&self, hub: &Hub) -> Result<Rows, HubError> {
        let (repository, first, after) = (
            self.repository.clone(),
            Some(self.first),
            self.after.clone(),
        );
        let fresh = Some(self.fresh);
        if self.checked {
            let state = match self.state {
                StateArg::Open => pull_request_list_checked::OpenOrClosed::OPEN,
                StateArg::Closed => pull_request_list_checked::OpenOrClosed::CLOSED,
                StateArg::All => pull_request_list_checked::OpenOrClosed::ALL,
            };
            let variables = pull_request_list_checked::Variables {
                repository,
                state: Some(state),
                first,
                after,
                fresh,
            };
            let page = hub.run::<PullRequestListChecked>(variables).await?;
            return Ok(Rows::new(&page.pull_requests));
        }
        let state = match self.state {
            StateArg::Open => pull_request_list::OpenOrClosed::OPEN,
            StateArg::Closed => pull_request_list::OpenOrClosed::CLOSED,
            StateArg::All => pull_request_list::OpenOrClosed::ALL,
        };
        let variables = pull_request_list::Variables {
            repository,
            state: Some(state),
            first,
            after,
            fresh,
        };
        let page = hub.run::<PullRequestList>(variables).await?;
        Ok(Rows::new(&page.pull_requests))
    }
}
