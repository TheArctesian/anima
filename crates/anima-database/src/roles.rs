//! The logins and the database of a bundled cluster, the same as `deploy/postgres/roles.sql`
//! makes them for a Docker installation (K2 "Roles", R-016, DB-003): `anima_owner` applies
//! migrations and owns every object, `anima_hub` serves as a member of `anima_runtime` and
//! owns nothing, and nobody connects to a database through `PUBLIC`.

use sqlx::postgres::{PgConnectOptions, PgConnection};
use sqlx::{ConnectOptions as _, Connection as _};

use crate::urls::{self, DATABASE, UrlError};
use crate::{Layout, Password, Role};

#[cfg(test)]
mod tests;

const RUNTIME_GROUP: &str = "anima_runtime";
/// What `roles.sql` gives a login: no right beyond signing in.
const PLAIN_LOGIN: &str = "login nosuperuser nocreatedb nocreaterole nobypassrls noreplication";

#[derive(Debug, thiserror::Error)]
pub enum RolesError {
    #[error("could not start the database client")]
    Client(#[source] std::io::Error),
    #[error("the superuser could not sign in ({cause})")]
    SignIn { cause: String },
    #[error("could not {step} (SQLSTATE {sqlstate})")]
    Sql {
        step: &'static str,
        sqlstate: String,
    },
    #[error(transparent)]
    Url(#[from] UrlError),
    #[error("the operating system gave no random bytes")]
    Random(#[source] getrandom::Error),
}

/// Makes the owner and hub logins, the runtime group and the `anima` database, and writes the
/// owner's and the hub's URL files. Safe to run again after a crash at any point: a login
/// that is there gets a new password, and the URL files are written last.
///
/// # Errors
/// When the superuser cannot sign in, a statement fails or a URL file cannot be written. An
/// error never carries statement text: a statement here holds a password (LOG-007).
pub fn prepare(layout: &Layout, superuser_password: &Password) -> Result<(), RolesError> {
    let owner = Password::random().map_err(RolesError::Random)?;
    let hub = Password::random().map_err(RolesError::Random)?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(RolesError::Client)?
        .block_on(apply(layout, superuser_password, &owner, &hub))?;
    urls::write(layout, Role::Owner, &owner)?;
    urls::write(layout, Role::Hub, &hub)?;
    Ok(())
}

async fn apply(
    layout: &Layout,
    superuser_password: &Password,
    owner: &Password,
    hub: &Password,
) -> Result<(), RolesError> {
    let mut session = PgConnectOptions::new()
        .socket(layout.socket_folder())
        .port(5432)
        .username(Role::Superuser.login())
        .password(superuser_password.expose())
        .database(Role::Superuser.database())
        .disable_statement_logging()
        .connect()
        .await
        .map_err(|error| RolesError::SignIn {
            cause: sqlstate(&error),
        })?;
    // The server starts with these already; a session can never turn them on by accident.
    for setting in [
        "set log_statement = none",
        "set log_min_error_statement = panic",
        "set log_min_duration_statement = -1",
    ] {
        run(&mut session, "set the session's logging off", setting).await?;
    }
    login_group(&mut session).await?;
    login(&mut session, Role::Owner, "", owner).await?;
    login(&mut session, Role::Hub, RUNTIME_GROUP, hub).await?;
    database(&mut session).await?;
    session.close().await.ok();
    Ok(())
}

async fn exists(
    session: &mut PgConnection,
    step: &'static str,
    question: &'static str,
    name: &str,
) -> Result<bool, RolesError> {
    sqlx::query_scalar(question)
        .bind(name)
        .fetch_one(session)
        .await
        .map_err(|error| sql_error(step, &error))
}

async fn login_group(session: &mut PgConnection) -> Result<(), RolesError> {
    const STEP: &str = "make the runtime group";
    let question = "select exists (select from pg_catalog.pg_roles where rolname = $1)";
    if !exists(session, STEP, question, RUNTIME_GROUP).await? {
        run(
            session,
            STEP,
            &format!("create role {RUNTIME_GROUP} nologin"),
        )
        .await?;
    }
    Ok(())
}

/// Makes `role`'s login, or gives a login that is there the new password. `group` is the
/// group it joins, or empty. The password is hexadecimal, so it needs no quoting.
async fn login(
    session: &mut PgConnection,
    role: Role,
    group: &str,
    password: &Password,
) -> Result<(), RolesError> {
    const STEP: &str = "make a login";
    let name = role.login();
    let question = "select exists (select from pg_catalog.pg_roles where rolname = $1)";
    let statement = if exists(session, STEP, question, name).await? {
        format!(
            "alter role {name} {PLAIN_LOGIN} password '{}'",
            password.expose()
        )
    } else {
        let membership = if group.is_empty() {
            String::new()
        } else {
            format!("in role {group}")
        };
        format!(
            "create role {name} {PLAIN_LOGIN} {membership} password '{}'",
            password.expose()
        )
    };
    run(session, STEP, &statement).await
}

async fn database(session: &mut PgConnection) -> Result<(), RolesError> {
    const STEP: &str = "make the database";
    let owner = Role::Owner.login();
    let question = "select exists (select from pg_catalog.pg_database where datname = $1)";
    if !exists(session, STEP, question, DATABASE).await? {
        run(
            session,
            STEP,
            &format!("create database {DATABASE} owner {owner}"),
        )
        .await?;
    }
    for grant in [
        format!("revoke connect, temporary on database {DATABASE} from public"),
        format!("grant connect on database {DATABASE} to {RUNTIME_GROUP}"),
        "revoke connect on database postgres from public".to_owned(),
        "revoke connect on database template1 from public".to_owned(),
    ] {
        run(session, "set who may connect", &grant).await?;
    }
    Ok(())
}

async fn run(
    session: &mut PgConnection,
    step: &'static str,
    statement: &str,
) -> Result<(), RolesError> {
    sqlx::raw_sql(statement)
        .execute(session)
        .await
        .map(|_| ())
        .map_err(|error| sql_error(step, &error))
}

fn sql_error(step: &'static str, error: &sqlx::Error) -> RolesError {
    RolesError::Sql {
        step,
        sqlstate: sqlstate(error),
    }
}

/// The SQLSTATE of a database error, or the kind of any other fault. Never the message: the
/// server can quote the statement in it.
fn sqlstate(error: &sqlx::Error) -> String {
    match error {
        sqlx::Error::Database(database) => database
            .code()
            .map_or_else(|| "none".to_owned(), std::borrow::Cow::into_owned),
        sqlx::Error::Io(source) => format!("no answer: {}", source.kind()),
        _ => "no answer".to_owned(),
    }
}
