//! The K1 object types of the check of a Compose file and of the `docker run` converter
//! (HOST-022).

use async_graphql::Object;

use crate::work::containers::compose_file::{
    Check, Converted, Note, Problem, ProblemCode, Refusal, RefusalCode, Severity,
};

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

    async fn service(&self) -> Option<&str> {
        self.service.as_deref()
    }

    async fn key(&self) -> Option<&str> {
        self.key.as_deref()
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
