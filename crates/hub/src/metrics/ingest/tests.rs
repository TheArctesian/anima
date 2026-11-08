//! Tests of the agent ingest parts that need no HTTP client: the throttle, the bearer header,
//! the refusal answers and the answer deadline.

mod credential;
mod deadline;
mod refusals;
mod throttle;
