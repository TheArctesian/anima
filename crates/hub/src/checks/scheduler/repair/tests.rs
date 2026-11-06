//! Tests for method repairs (CHK-013, R-064). The rule is tested alone; the edge is tested
//! against the database with K6 devices on 127.0.0.1, local listeners and a policy that
//! allows loopback. Nothing reaches the internet.

/// Wraps each async test in a throwaway database holding the K6 devices.
macro_rules! repair_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../../../tests/fixtures", scripts("devices"))
        )]
        async fn $name($pool: sqlx::PgPool) $body
    )+};
}

mod guards;
mod races;
mod rule;
mod support;
mod ticks;
