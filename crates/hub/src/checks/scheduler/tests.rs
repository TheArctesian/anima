//! Tests for the scheduler. The pure decisions are tested alone; one `tick` is tested
//! against the database with a device on 127.0.0.1, local listeners and a policy that allows
//! loopback. These tests live in the crate for `AddressPolicy::allowing_loopback` and the
//! scheduler's private loop state. Nothing reaches the internet.

/// Wraps each async test in a throwaway database holding the K6 devices.
macro_rules! tick_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../../tests/fixtures", scripts("devices"))
        )]
        async fn $name($pool: sqlx::PgPool) $body
    )+};
}

mod daily;
mod decisions;
mod policy;
mod retries;
mod support;
mod tick;
mod watch;
