//! The issue changes (K1; OD-E14, PRJ-027, PRJ-028): make an issue, comment on an issue or a
//! pull request, close or open an issue.
//!
//! Each one checks its input into the parts of a [`Change`] first, then the repository; a
//! refusal there sends nothing and records nothing. The change is sent once and recorded by
//! [`forge::make`]; the result is read back from GitHub. None takes a confirmation (K-E1
//! Section 13.4) and each takes a write key (Section 13.5).

use async_graphql::{Context, ID, InputObject, Object, Result};
use provider_github::live::issues::{CloseReason as Read, Issue};
use source_exchange::{
    Change, CloseReason, IssueState, Label, Labels, Number, PartError, Receipt, RepositoryName,
    Text, Title,
};

use super::{
    back,
    views::{CommentView, IssueCloseReason, IssueView, issue},
};
use crate::{
    api_errors::{FieldError, invalid},
    fleet::access::Authenticated,
    work::projects::forge::{self, Subject, Target, failure},
};

#[derive(InputObject)]
#[graphql(name = "NewIssueInput")]
pub struct NewIssueInput {
    repository: ID,
    /// 1 to 256 characters.
    title: String,
    /// At most 65,536 characters.
    body: Option<String>,
    /// At most 20, each 1 to 50 characters.
    labels: Option<Vec<String>>,
}

#[derive(Default)]
pub struct Mutation;

#[Object(name = "IssuesMutation")]
impl Mutation {
    /// Makes an issue on GitHub. Not repeated by the hub: after UNAVAILABLE the issue may exist. Writes an audit entry (PRJ-028).
    #[graphql(guard = "Authenticated")]
    async fn create_issue(&self, ctx: &Context<'_>, input: NewIssueInput) -> Result<IssueView> {
        let (title, body, labels) = new_issue(&input)?;
        let target = forge::target(ctx, &input.repository).await?;
        let change = Change::CreateIssue {
            repository: named(&target)?,
            title,
            body,
            labels,
        };
        let receipt = forge::make(ctx, &target, &change, Subject::Issue).await?;
        let Receipt::Created { number } = receipt else {
            return Err(failure::unread());
        };
        back::issue(ctx, &target, number).await
    }

    /// number is an issue or a pull request. body is 1 to 65,536 characters. Not repeated by the hub. Writes an audit entry (PRJ-028).
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Mutation.addIssueComment"
    )]
    async fn add_issue_comment(
        &self,
        ctx: &Context<'_>,
        repository: ID,
        number: i32,
        body: String,
    ) -> Result<CommentView> {
        let (number, body) = both(numbered(number), checked("body", Text::parse(&body)))?;
        let target = forge::target(ctx, &repository).await?;
        let change = Change::AddIssueComment {
            repository: named(&target)?,
            number,
            body,
        };
        let receipt = forge::make(ctx, &target, &change, Subject::IssueOrPullRequest).await?;
        let Receipt::Created { number: id } = receipt else {
            return Err(failure::unread());
        };
        back::comment(ctx, &target, number.get(), id).await
    }

    /// Closes or opens an issue. reason only with open false. No change when it is already so. VALIDATION when the number is a pull request. Writes an audit entry (PRJ-028).
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Mutation.setIssueOpen"
    )]
    async fn set_issue_open(
        &self,
        ctx: &Context<'_>,
        repository: ID,
        number: i32,
        open: bool,
        reason: Option<IssueCloseReason>,
    ) -> Result<IssueView> {
        let (number, state) = both(numbered(number), state(open, reason))?;
        let target = forge::target(ctx, &repository).await?;
        let repository_name = named(&target)?;
        let found = back::before(ctx, &target, number.get()).await?;
        if already(&found, state) {
            return Ok(issue(found, target.repository));
        }
        let change = Change::SetIssueOpen {
            repository: repository_name,
            number,
            state,
        };
        forge::make(ctx, &target, &change, Subject::Issue).await?;
        back::issue(ctx, &target, number.get()).await
    }
}

/// The checked title, body and labels of `input`, or every field that is not one. An empty
/// body is no body.
fn new_issue(input: &NewIssueInput) -> Result<(Title, Option<Text>, Labels)> {
    let title = checked("title", Title::parse(&input.title));
    let body = match input.body.as_deref() {
        None | Some("") => Ok(None),
        Some(body) => checked("body", Text::parse(body)).map(Some),
    };
    let labels = checked(
        "labels",
        labels(input.labels.as_deref().unwrap_or_default()),
    );
    match (title, body, labels) {
        (Ok(title), Ok(body), Ok(labels)) => Ok((title, body, labels)),
        (title, body, labels) => Err(invalid(
            &[title.err(), body.err(), labels.err()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
        )),
    }
}

fn labels(names: &[String]) -> Result<Labels, PartError> {
    let labels = names
        .iter()
        .map(|name| Label::parse(name))
        .collect::<Result<Vec<_>, _>>()?;
    Labels::new(labels)
}

/// What the issue becomes: a reason only closes one.
fn state(open: bool, reason: Option<IssueCloseReason>) -> Result<IssueState, FieldError> {
    match (open, reason) {
        (true, None) => Ok(IssueState::Open),
        (true, Some(_)) => Err(FieldError::new(
            "reason",
            "Give a reason only to close an issue.",
        )),
        (false, reason) => Ok(IssueState::Closed(reason.map(|reason| match reason {
            IssueCloseReason::Completed => CloseReason::Completed,
            IssueCloseReason::NotPlanned => CloseReason::NotPlanned,
        }))),
    }
}

/// Whether `found` is already as `state` asks: open, or closed with the reason asked (any
/// reason when none is asked).
fn already(found: &Issue, state: IssueState) -> bool {
    match state {
        IssueState::Open => found.open,
        IssueState::Closed(None) => !found.open,
        IssueState::Closed(Some(reason)) => {
            let read = match reason {
                CloseReason::Completed => Read::Completed,
                CloseReason::NotPlanned => Read::NotPlanned,
            };
            !found.open && found.close_reason == Some(read)
        }
    }
}

/// The repository's stored name as the part a change holds.
///
/// # Errors
/// `CONFLICT` when the name is not one the hub sends (nothing is sent).
fn named(target: &Target) -> Result<RepositoryName> {
    RepositoryName::parse(&target.name).map_err(|_| failure::unnamed())
}

fn numbered(number: i32) -> Result<Number, FieldError> {
    u64::try_from(number)
        .ok()
        .and_then(|n| Number::new(n).ok())
        .ok_or_else(|| FieldError::new("number", "Give a number of 1 or more."))
}

/// A part, or the fixed words of `field`: never the value (LOG-007).
fn checked<T>(field: &'static str, part: Result<T, PartError>) -> Result<T, FieldError> {
    part.map_err(|refused| FieldError::new(field, words(refused)))
}

const fn words(refused: PartError) -> &'static str {
    match refused {
        PartError::Title => "Give a title of 1 to 256 characters on one line.",
        PartError::Text => {
            "Give 1 to 65,536 characters with no NUL or control character but tab and line breaks."
        }
        PartError::Label => "Give at most 20 labels, each 1 to 50 characters on one line.",
        _ => "This value cannot be sent to GitHub.",
    }
}

/// Both parts, or one `VALIDATION` error listing each that is not.
fn both<A, B>(a: Result<A, FieldError>, b: Result<B, FieldError>) -> Result<(A, B)> {
    match (a, b) {
        (Ok(a), Ok(b)) => Ok((a, b)),
        (a, b) => Err(invalid(
            &a.err().into_iter().chain(b.err()).collect::<Vec<_>>(),
        )),
    }
}
