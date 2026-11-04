//! The process log: `tracing` output as JSON lines on standard output (LOG-001 to LOG-003),
//! with the HTTP and GraphQL request spans and the error boundary that feed it.

mod causes;
mod echoes;
mod http_span;
mod internal_errors;
mod request_span;
#[cfg(test)]
pub(crate) mod tests;

use tracing::Subscriber;
use tracing_subscriber::{EnvFilter, fmt::MakeWriter};

pub use http_span::http_request;
pub use internal_errors::InternalErrors;
pub use request_span::RequestSpan;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("ANIMA_LOG={filter:?} is not a valid log filter")]
    Filter {
        filter: String,
        #[source]
        source: tracing_subscriber::filter::ParseError,
    },
    #[error("the process log is already set up")]
    AlreadySet(#[from] tracing::subscriber::SetGlobalDefaultError),
}

/// A subscriber that writes one JSON object per line to `writer`, limited by `filter`.
///
/// Each line has `timestamp`, `level`, `target` and `fields.message`; inside a GraphQL request it
/// also has `span.request_id` (LOG-002).
///
/// # Errors
///
/// `Error::Filter` if `filter` is not a valid `tracing` filter directive list.
pub fn subscriber<W>(filter: &str, writer: W) -> Result<impl Subscriber + Send + Sync, Error>
where
    W: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter = EnvFilter::try_new(filter).map_err(|source| Error::Filter {
        filter: filter.to_owned(),
        source,
    })?;
    Ok(tracing_subscriber::fmt()
        .json()
        .with_env_filter(env_filter)
        .with_current_span(true)
        .with_span_list(false)
        .with_writer(writer)
        .finish())
}

/// Sets up the process-wide log on standard output.
///
/// # Errors
///
/// `Error::Filter` for a bad `filter`; `Error::AlreadySet` if a log is already installed.
pub fn init(filter: &str) -> Result<(), Error> {
    tracing::subscriber::set_global_default(subscriber(filter, std::io::stdout)?)?;
    Ok(())
}
