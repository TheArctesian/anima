//! The `anima-hub operator` commands: the one operator account and its second factor, made on
//! the hub computer only (D-9, OD-2, FRN-003, FRN-004). Secrets come in on standard input and
//! go out on standard output only, never in an argument or the process log (LOG-007).

use std::io::{BufRead, IsTerminal as _, Read as _};

use anima_hub::{
    clock::{Clock as _, SystemClock},
    config::Config,
    fleet::{access, builds},
};
use zeroize::Zeroizing;

use crate::{StartError, connect, link_audit_entries, vault};

/// Bytes `operator create` reads at most: a password of up to 1024 bytes, a 32-character
/// secret and a 6-digit code, each with its line end, fit; longer input is cut and refused.
const CREATE_INPUT_LIMIT: usize = 2048;

/// Reads the password (without echo on a terminal, one line when piped) and sets the operator.
pub(crate) async fn set(config: &Config, username: &str) -> Result<(), StartError> {
    let typed = read_password().map_err(|_| StartError::Stdin)?;
    let pool = connect(config).await?;
    link_audit_entries(&pool).await?;
    let now = SystemClock.now();
    let before = builds::builders(&pool).await?;
    access::set_operator(&pool, username, access::Password::new(typed), now).await?;
    builds::audit_removed(&pool, &before, now, builds::Removal::CredentialsChanged).await?;
    println!("Operator {username} is set.");
    Ok(())
}

/// Reads the password, a secret from `operator totp --new` and a code of it from standard
/// input, and creates or replaces the operator with both factors only when all three pass
/// (FRN-004). The builders that went with the tokens are audited, as `operator set` audits
/// them. A terminal on standard input is refused before anything is read: it would echo the
/// password and the secret (LOG-007).
pub(crate) async fn create(config: &Config, username: &str) -> Result<(), StartError> {
    if std::io::stdin().is_terminal() {
        return Err(StartError::CreateFromTerminal);
    }
    let new = read_new_operator(username).map_err(|_| StartError::CreateInput)?;
    let vault = vault(config)?;
    let pool = connect(config).await?;
    link_audit_entries(&pool).await?;
    let before = builds::builders(&pool).await?;
    let now = SystemClock.now();
    access::create_operator(&pool, &vault, new, now).await?;
    builds::audit_removed(&pool, &before, now, builds::Removal::CredentialsChanged).await?;
    println!("Operator {username} is set, with TOTP.");
    Ok(())
}

/// Prints the one operator as one JSON line, `{"name":…,"totp":…}`, or `null` (FRN-003).
pub(crate) async fn show(config: &Config) -> Result<(), StartError> {
    let pool = connect(config).await?;
    let status = access::operator_status(&pool).await?;
    let line = status.map(|s| serde_json::json!({ "name": s.name, "totp": s.totp }));
    println!("{}", line.unwrap_or_default());
    Ok(())
}

/// Enrols TOTP and writes the `otpauth://` URI once, to the terminal only: never to a pipe,
/// a file or the process log (LOG-007).
pub(crate) async fn enrol_totp(config: &Config, username: &str) -> Result<(), StartError> {
    if !std::io::stdout().is_terminal() {
        return Err(StartError::NotATerminal);
    }
    let vault = vault(config)?;
    let pool = connect(config).await?;
    link_audit_entries(&pool).await?;
    let before = builds::builders(&pool).await?;
    let enrolment = access::enrol_totp(&pool, &vault, username).await?;
    let removal = builds::Removal::CredentialsChanged;
    builds::audit_removed(&pool, &before, SystemClock.now(), removal).await?;
    println!(
        "Add this to an authenticator app, then sign in again with its code:\n{}",
        enrolment.uri()
    );
    Ok(())
}

/// Prints a new TOTP secret and its URI as one JSON line and saves nothing (FRN-004). The
/// line goes to standard output, the caller's pipe, and nowhere else (LOG-007).
pub(crate) fn new_totp(username: &str) -> Result<(), StartError> {
    let made = access::new_totp(username)?;
    // Base32 and the URI of a checked username (D-9) need no JSON escaping, and a string built
    // here is wiped on drop, as a serializer's buffer is not.
    let line = Zeroizing::new(format!(
        "{{\"secret\":\"{}\",\"uri\":\"{}\"}}",
        made.secret(),
        made.uri()
    ));
    println!("{}", line.as_str());
    Ok(())
}

/// Turns TOTP off and signs every client out. Outside the link mode the password alone signs
/// in again; in the link mode no sign-in passes until a new secret is set (FRN-007).
pub(crate) async fn disable_totp(config: &Config, username: &str) -> Result<(), StartError> {
    let pool = connect(config).await?;
    link_audit_entries(&pool).await?;
    let before = builds::builders(&pool).await?;
    let now = SystemClock.now();
    access::disable_totp(&pool, username, now).await?;
    builds::audit_removed(&pool, &before, now, builds::Removal::CredentialsChanged).await?;
    if config.link_socket.is_some() {
        println!(
            "TOTP is off for operator {username}. In the link mode the hub then refuses \
             sign-in until a new TOTP is set (anima-hub operator create)."
        );
    } else {
        println!("TOTP is off for operator {username}.");
    }
    Ok(())
}

/// The password from standard input: prompted without echo on a terminal, else one line.
fn read_password() -> std::io::Result<String> {
    if std::io::stdin().is_terminal() {
        return rpassword::prompt_password("Password: ");
    }
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    let typed = line.trim_end_matches(['\r', '\n']).to_owned();
    zeroize::Zeroize::zeroize(&mut line);
    Ok(typed)
}

/// The password, the secret and the code, one line each, from at most [`CREATE_INPUT_LIMIT`]
/// bytes of standard input. Nothing is echoed: the caller pipes them in.
fn read_new_operator(username: &str) -> std::io::Result<access::NewOperator> {
    let limit = u64::try_from(CREATE_INPUT_LIMIT).unwrap_or(u64::MAX);
    let mut input = std::io::stdin().lock().take(limit);
    let password = read_secret_line(&mut input)?;
    let secret = read_secret_line(&mut input)?;
    let code = read_secret_line(&mut input)?;
    Ok(access::NewOperator {
        username: username.to_owned(),
        password: access::Password::new(password.as_str().to_owned()),
        secret,
        code,
    })
}

/// One line without its line end. The buffer holds the whole limit, so it never moves, and
/// is wiped on drop.
fn read_secret_line(input: &mut impl BufRead) -> std::io::Result<Zeroizing<String>> {
    let mut line = Zeroizing::new(String::with_capacity(CREATE_INPUT_LIMIT));
    input.read_line(&mut line)?;
    Ok(Zeroizing::new(
        line.trim_end_matches(['\r', '\n']).to_owned(),
    ))
}
