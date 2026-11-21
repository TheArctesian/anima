//! The rules of the routes, pure, and the face, the store and the interface of the proxy
//! against PostgreSQL with doubles for DNS, the issuer and the connection (task H21).

/// A test against a fresh database with the devices fixture.
macro_rules! db_test {
    ($(#[$meta:meta])* async fn $name:ident($pool:ident: PgPool) $body:block) => {
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../../tests/fixtures", scripts("devices"))
        )]
        $(#[$meta])*
        async fn $name($pool: sqlx::PgPool) $body
    };
}

mod caa;
mod door;
mod names;
mod proof;
mod store;
mod support;
mod target;
