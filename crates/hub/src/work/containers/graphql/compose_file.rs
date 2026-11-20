//! The K1 object types of the check of a Compose file and of the `docker run` converter
//! (HOST-022).

use async_graphql::{Object, Result};

use crate::{
    api_errors::internal,
    work::containers::compose_file::{
        Check, Converted, Note, Problem, ProblemCode, Refusal, RefusalCode, Severity,
    },
};

/// The complexity of one check or one conversion. With the limit of the schema (500), one
/// request holds at most four: each reads up to 256 KiB of the caller's text.
pub(super) const TEXT_COMPLEXITY: usize = 100;

/// Runs `work` on a thread for blocking work, so that a long text does not hold a worker of
/// the runtime. The thread writes under the span and to the log of the request.
pub(super) async fn off_the_runtime<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T> {
    let dispatch = tracing::dispatcher::get_default(Clone::clone);
    let span = tracing::Span::current();
    let run = move || tracing::dispatcher::with_default(&dispatch, || span.in_scope(work));
    tokio::task::spawn_blocking(run).await.map_err(internal)
}

#[Object(name = "ComposeFileProblem")]
impl Problem {
    async fn code(&self) -> ProblemCode {
        self.code
    }

    async fn severity(&self) -> Severity {
        self.code.severity()
    }

    /// The line of the problem in the file, from 1.
    async fn line(&self) -> u32 {
        self.place.line
    }

    /// The column of the problem in its line, from 1.
    async fn column(&self) -> u32 {
        self.place.column
    }

    /// The number of the service in `services`, from 1, when the problem is in one.
    async fn service_number(&self) -> Option<u32> {
        self.service_number
    }

    /// The name of the key, when the problem is about a key that Compose defines.
    async fn key(&self) -> Option<&str> {
        self.key
    }
}

#[Object(name = "ComposeFileCheck")]
impl Check {
    /// True when the check found no error. It is a check of the syntax only.
    #[graphql(name = "syntaxValid")]
    async fn valid(&self) -> bool {
        self.syntax_valid()
    }

    async fn problems(&self) -> &[Problem] {
        &self.problems
    }

    async fn cut(&self) -> bool {
        self.cut
    }
}

/// The answer of `composeFromDockerRun`: the file, or why there is none.
pub struct FromDockerRun(pub Result<Converted, Refusal>);

#[Object(name = "ComposeFromDockerRun")]
impl FromDockerRun {
    async fn compose_file(&self) -> Option<&str> {
        self.0.as_ref().ok().map(|done| done.compose_file.as_str())
    }

    async fn service(&self) -> Option<&str> {
        self.0.as_ref().ok().map(|done| done.service.as_str())
    }

    async fn refusal(&self) -> Option<RefusalCode> {
        self.0.as_ref().err().map(|refusal| refusal.code)
    }

    /// The position in the command where the converter stopped, in characters from 1.
    async fn refused_at(&self) -> Option<u32> {
        self.0.as_ref().err().map(|refusal| refusal.at)
    }

    async fn notes(&self) -> &[Note] {
        self.0.as_ref().map_or(&[], |done| done.notes.as_slice())
    }

    async fn not_converted(&self) -> &[String] {
        let none: &[String] = &[];
        self.0
            .as_ref()
            .map_or(none, |done| done.not_converted.as_slice())
    }
}
