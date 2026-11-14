//! `anima` and `anima mcp` against the real hub, not a scripted one: the hub's router, access
//! check, resolvers, stores and migrations, served in the test on a loopback port over a
//! database of its own (R-009, R-019). Each test starts its own hub. Every test reads what it
//! changed back through the hub, not only from the command's output.

mod actors;
mod breadth;
mod checks;
mod devices;
mod keys;
mod mcp;
mod paging;
mod projects;
mod sessions;
mod sign_in;
mod watch;

/// The arguments of a command line written as one line: words split at spaces.
fn words(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}
