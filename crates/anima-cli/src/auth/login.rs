//! `anima auth login`: a login token from the user name, password and authenticator code at the
//! terminal (CMD-010), or an API key from standard input (CMD-011). The token is checked with
//! the hub before it is stored.

use std::io::{self, Read as _};

use zeroize::{Zeroize as _, Zeroizing};

use super::documents::{Login, Me, login, me};
use crate::failure::Failure;
use crate::hub::{Hub, HubError, Origin, Token};
use crate::prompt::Prompter;

/// The longest API key read from standard input.
const MAX_KEY: usize = 4096;

/// A token the hub accepted, and the user it belongs to.
pub struct SignedIn {
    pub token: Token,
    pub username: String,
}

/// Reads a token from standard input (echo off at a terminal) and checks it with `me`.
///
/// # Errors
/// [`Failure::Input`] for an unreadable or empty input; the hub's refusal of the token.
pub async fn with_token(
    origin: Origin,
    prompter: Prompter,
    stdin_terminal: bool,
) -> Result<SignedIn, Failure> {
    let text = if stdin_terminal {
        prompter.ask_secret("API key", "the key on standard input")?
    } else {
        read_stdin()?
    };
    let token = Token::new(text).ok_or_else(|| Failure::Input {
        what: "the token".to_owned(),
        reason: "standard input held no token".to_owned(),
    })?;
    let hub = Hub::new(origin, Some(token))?;
    let answer = hub.run::<Me>(me::Variables).await?;
    let token = hub.into_token().ok_or(HubError::Setup)?;
    Ok(SignedIn {
        token,
        username: answer.me.username,
    })
}

/// Asks for the user name (unless given), the password and the authenticator code, and signs
/// in. The hub does not say whether an account needs a code (a missing code is a wrong
/// password, D-8), so the code is always asked for and may be left empty.
///
/// # Errors
/// [`Failure::Usage`] naming `--with-token` when no terminal can be asked; the hub's refusal.
pub async fn with_password(
    origin: Origin,
    prompter: Prompter,
    username: Option<String>,
) -> Result<SignedIn, Failure> {
    if !prompter.interactive() {
        return Err(Failure::missing(
            "A password, read only at a terminal,",
            "--with-token",
        ));
    }
    let username = match username {
        Some(name) => name,
        None => prompter.ask("User name", "--username")?,
    };
    let password = prompter.ask_secret("Password", "--with-token")?;
    let code = prompter.ask(
        "Authenticator code (Enter if you have none)",
        "--with-token",
    )?;
    if !(code.is_empty() || code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit())) {
        return Err(Failure::Usage {
            message: "The authenticator code is six digits.".to_owned(),
            hint: None,
        });
    }
    let input = login::LoginInput {
        username,
        password: password.as_str().to_owned(),
        totp_code: Some(code).filter(|code| !code.is_empty()),
    };
    let mut body = <Login as graphql_client::GraphQLQuery>::build_query(login::Variables { input });
    let hub = Hub::new(origin, None)?;
    let answer: Result<login::ResponseData, HubError> = hub.send(&body).await;
    body.variables.input.password.zeroize();
    let payload = answer?.login;
    let token =
        Token::new(Zeroizing::new(payload.token)).ok_or(HubError::Malformed { status: 200 })?;
    Ok(SignedIn {
        token,
        username: payload.operator.username,
    })
}

fn read_stdin() -> Result<Zeroizing<String>, Failure> {
    let mut bytes = Zeroizing::new(Vec::with_capacity(MAX_KEY + 1));
    io::stdin()
        .lock()
        .take(MAX_KEY as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| Failure::Input {
            what: "standard input".to_owned(),
            reason: error.to_string(),
        })?;
    let text = std::str::from_utf8(&bytes)
        .ok()
        .filter(|_| bytes.len() <= MAX_KEY);
    text.map(|text| Zeroizing::new(text.to_owned()))
        .ok_or_else(|| Failure::Input {
            what: "the token".to_owned(),
            reason: "standard input is not one short line of text".to_owned(),
        })
}
