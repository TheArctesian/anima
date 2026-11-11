//! The store against PostgreSQL: what it reads (`read`) and what it writes (`write`).

use super::*;

/// `store_test` one folder deeper: sqlx reads the fixture path from the calling file.
macro_rules! store_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../../tests/fixtures", scripts("devices"))
        )]
        async fn $name($pool: sqlx::PgPool) $body
    )+};
}

mod read;
mod write;
