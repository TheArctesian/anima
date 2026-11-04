//! Every hub fault reaches the process log with its cause chain, coded or not, and never with
//! SQL text, a value or a token (H-5, API-010, LOG-007).

use std::{borrow::Cow, error::Error as StdError, fmt, sync::Arc};

use async_graphql::{
    EmptyMutation, EmptySubscription, Error as GraphqlError, ErrorExtensions as _, Object, Schema,
};
use serde_json::Value;
use sqlx::error::{DatabaseError, ErrorKind};

use super::{capture::capture_log, leaks::SECRET};
use crate::schema::harden;

/// A PostgreSQL error as the server sends it: its message quotes a value.
#[derive(Debug)]
struct ServerFault {
    code: &'static str,
    constraint: Option<&'static str>,
}

impl fmt::Display for ServerFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid input syntax: \"{SECRET}\" in select * from secret_table"
        )
    }
}

impl StdError for ServerFault {}

impl DatabaseError for ServerFault {
    fn message(&self) -> &'static str {
        "invalid input syntax"
    }

    fn code(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.code))
    }

    fn constraint(&self) -> Option<&str> {
        self.constraint
    }

    fn as_error(&self) -> &(dyn StdError + Send + Sync + 'static) {
        self
    }

    fn as_error_mut(&mut self) -> &mut (dyn StdError + Send + Sync + 'static) {
        self
    }

    fn into_error(self: Box<Self>) -> Box<dyn StdError + Send + Sync + 'static> {
        self
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

fn database(code: &'static str, constraint: Option<&'static str>) -> sqlx::Error {
    sqlx::Error::Database(Box::new(ServerFault { code, constraint }))
}

/// A domain error over a database error, as `devices::Error::Database` is.
#[derive(Debug, thiserror::Error)]
#[error("database error while saving token={SECRET}")]
struct Domain(#[source] sqlx::Error);

/// A chain of `depth` links, each wrapping the next.
#[derive(Debug, thiserror::Error)]
#[error("link {depth}")]
struct Link {
    depth: usize,
    #[source]
    next: Option<Box<Link>>,
}

fn link(depth: usize) -> Link {
    Link {
        depth,
        next: (depth > 1).then(|| Box::new(link(depth - 1))),
    }
}

/// An error with a hidden cause, as the domains build it, and `code`.
fn hidden(cause: impl StdError + Send + Sync + 'static, code: &'static str) -> GraphqlError {
    let cause: Arc<dyn StdError + Send + Sync> = Arc::new(cause);
    let mut error = GraphqlError::new_with_source(cause);
    "Something went wrong on the hub. Try again.".clone_into(&mut error.message);
    error.extend_with(|_, extensions| extensions.set("code", code))
}

struct Query;

#[Object]
impl Query {
    async fn deadlock(&self) -> async_graphql::Result<bool> {
        Err(hidden(Domain(database("40P01", None)), "INTERNAL"))
    }

    async fn duplicate(&self) -> async_graphql::Result<bool> {
        Err(hidden(
            Domain(database("23505", Some("devices_name_key"))),
            "INTERNAL",
        ))
    }

    async fn unreachable(&self) -> async_graphql::Result<bool> {
        Err(hidden(sqlx::Error::PoolTimedOut, "UNAVAILABLE"))
    }

    async fn deep(&self) -> async_graphql::Result<bool> {
        Err(hidden(link(12), "INTERNAL"))
    }

    async fn coded(&self) -> async_graphql::Result<bool> {
        Err(GraphqlError::new(format!("pool broke, password={SECRET}"))
            .extend_with(|_, extensions| extensions.set("code", "INTERNAL")))
    }

    async fn uncoded(&self) -> async_graphql::Result<bool> {
        Err(database("57014", None))?
    }
}

fn schema() -> Schema<Query, EmptyMutation, EmptySubscription> {
    harden(Schema::build(Query, EmptyMutation, EmptySubscription), true).finish()
}

/// Runs `query` and returns its only error and the log lines at error level.
async fn run(query: &str) -> (Value, Vec<String>) {
    let (capture, _guard) = capture_log("trace");
    let response = schema().execute(query).await;
    let [error] = response.errors.as_slice() else {
        panic!("one error: {:?}", response.errors);
    };
    let lines = capture
        .lines()
        .into_iter()
        .filter(|line| line["level"] == "ERROR")
        .map(|line| line.to_string())
        .collect();
    (serde_json::to_value(error).unwrap(), lines)
}

fn assert_clean(lines: &[String]) {
    for line in lines {
        for leak in [SECRET, "select", "secret_table", "invalid input syntax"] {
            assert!(!line.contains(leak), "{leak} in {line}");
        }
    }
}

#[tokio::test]
async fn h_5_a_coded_internal_error_is_logged_with_its_chain_and_masked() {
    let (error, lines) = run("{ deadlock }").await;
    assert_eq!(error["message"], "Something went wrong on the hub.");
    assert_eq!(error["extensions"]["code"], "INTERNAL");
    let [line] = lines.as_slice() else {
        panic!("one error line: {lines:?}");
    };
    assert!(
        line.contains("database error while saving token=[redacted]"),
        "{line}"
    );
    assert!(line.contains("database error 40P01"), "{line}");
    assert!(line.contains(r#""code":"INTERNAL""#), "{line}");
    assert!(line.contains("deadlock"), "the path: {line}");
    assert_clean(&lines);

    let (_, lines) = run("{ duplicate }").await;
    assert!(
        lines[0].contains("database error 23505 on devices_name_key"),
        "{lines:?}"
    );
    assert_clean(&lines);
}

#[tokio::test]
async fn h_5_a_coded_internal_error_without_a_cause_is_logged_by_its_masked_message() {
    let (error, lines) = run("{ coded }").await;
    assert_eq!(error["message"], "Something went wrong on the hub.");
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].contains("pool broke, password=[redacted]"),
        "{lines:?}"
    );
    assert_clean(&lines);
}

#[tokio::test]
async fn h_5_a_database_error_from_a_resolver_is_logged_by_its_sqlstate() {
    let (error, lines) = run("{ uncoded }").await;
    assert_eq!(error["extensions"]["code"], "INTERNAL");
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("database error 57014"), "{lines:?}");
    assert_clean(&lines);
}

#[tokio::test]
async fn h_5_another_code_with_a_hidden_cause_is_logged_and_keeps_its_message() {
    let (error, lines) = run("{ unreachable }").await;
    assert_eq!(error["extensions"]["code"], "UNAVAILABLE");
    assert_eq!(
        error["message"],
        "Something went wrong on the hub. Try again."
    );
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("database pool timed out"), "{lines:?}");
    assert!(lines[0].contains(r#""code":"UNAVAILABLE""#), "{lines:?}");
}

#[tokio::test]
async fn h_5_a_long_chain_is_cut_after_eight_links() {
    let (_, lines) = run("{ deep }").await;
    let line = &lines[0];
    assert!(line.contains("link 12: link 11"), "{line}");
    assert!(line.contains("link 5: …"), "{line}");
    assert!(!line.contains("link 4"), "{line}");
}

#[test]
fn h_5_a_database_error_is_described_by_kind_never_by_its_text() {
    let io = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, SECRET);
    let boxed = || -> Box<dyn StdError + Send + Sync> { SECRET.into() };
    let cases = [
        (
            sqlx::Error::Io(io),
            "database connection: ConnectionRefused",
        ),
        (sqlx::Error::PoolTimedOut, "database pool timed out"),
        (sqlx::Error::PoolClosed, "database pool closed"),
        (sqlx::Error::Tls(boxed()), "database tls"),
        (
            sqlx::Error::Protocol(SECRET.to_owned()),
            "database protocol",
        ),
        (sqlx::Error::RowNotFound, "database row not found"),
        (
            sqlx::Error::TypeNotFound {
                type_name: SECRET.to_owned(),
            },
            "database type not found",
        ),
        (
            sqlx::Error::ColumnNotFound(SECRET.to_owned()),
            "database column not found",
        ),
        (
            sqlx::Error::ColumnIndexOutOfBounds { index: 1, len: 0 },
            "database column not found",
        ),
        (
            sqlx::Error::ColumnDecode {
                index: SECRET.to_owned(),
                source: boxed(),
            },
            "database decode",
        ),
        (sqlx::Error::Decode(boxed()), "database decode"),
        (sqlx::Error::Encode(boxed()), "database encode"),
        (
            sqlx::Error::Configuration(boxed()),
            "database configuration",
        ),
        (sqlx::Error::WorkerCrashed, "database worker crashed"),
        (sqlx::Error::BeginFailed, "database"),
        (database("40001", None), "database error 40001"),
    ];
    for (error, kind) in cases {
        assert_eq!(super::super::causes::database_kind(&error), kind);
    }
}
