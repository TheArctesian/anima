//! Where a credential comes from and how it is sent (CMD-025): a prompt with echo off, standard
//! input (`-`) or a private file, never an argument; and the request's copy is wiped after.

use std::path::Path;

use graphql_client::GraphQLQuery;
use zeroize::Zeroizing;

use crate::failure::Failure;
use crate::hub::{Hub, HubError};
use crate::input;
use crate::prompt::Prompter;

/// The path that means standard input.
const STDIN: &str = "-";

/// The longest password or key; a private key file is a few kilobytes.
const LIMIT: usize = 64 * 1024;

/// A password from `from_file` (a path, or `-` for standard input), or else asked for at the
/// terminal. One line break at its end is not part of the password.
///
/// # Errors
/// [`Failure::Usage`] naming `--from-file` when it cannot ask; [`Failure::Input`] when the
/// file is exposed, unreadable, empty or not text.
pub fn password(
    prompter: Prompter,
    what: &str,
    from_file: Option<&Path>,
) -> Result<Zeroizing<String>, Failure> {
    let mut text = match from_file {
        Some(path) => read(prompter, path, what)?,
        None => prompter.ask_secret(what, "--from-file")?,
    };
    let kept = text.trim_end_matches(['\n', '\r']).len();
    text.truncate(kept);
    nonempty(text, what)
}

/// A secret that cannot be typed on one line (a private key) from `from_file`.
///
/// # Errors
/// As [`password`]; [`Failure::Usage`] when `from_file` is missing, as a prompt cannot take
/// several lines.
pub fn from_file_only(
    prompter: Prompter,
    what: &str,
    from_file: Option<&Path>,
) -> Result<Zeroizing<String>, Failure> {
    let path = from_file.ok_or_else(|| Failure::missing(what, "--from-file"))?;
    nonempty(read(prompter, path, what)?, what)
}

/// Sends `variables` of document `Q` and wipes the secrets in them with `wipe` whether or not
/// the hub accepted the request.
///
/// # Errors
/// [`HubError`] for a refusal or a failed request.
pub async fn send_wiping<Q: GraphQLQuery>(
    hub: &Hub,
    variables: Q::Variables,
    wipe: impl FnOnce(&mut Q::Variables),
) -> Result<Q::ResponseData, HubError> {
    let mut body = Q::build_query(variables);
    let answer = hub.send(&body).await;
    wipe(&mut body.variables);
    answer
}

fn nonempty(text: Zeroizing<String>, what: &str) -> Result<Zeroizing<String>, Failure> {
    if text.is_empty() {
        return Err(unreadable(what, "it is empty"));
    }
    Ok(text)
}

/// The text of a private file, or of standard input for `-`. The reason never holds the text.
fn read(prompter: Prompter, path: &Path, what: &str) -> Result<Zeroizing<String>, Failure> {
    let bytes = if path.as_os_str() == STDIN {
        input::read(prompter, path, what, LIMIT)?
    } else {
        crate::private_file::read(path)
            .map_err(|error| unreadable(what, &format!("{}: {error}", path.display())))?
            .ok_or_else(|| unreadable(what, &format!("{} does not exist", path.display())))?
    };
    if bytes.len() > LIMIT {
        return Err(unreadable(what, "it is longer than 64 KiB"));
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| unreadable(what, "it is not text"))?;
    Ok(Zeroizing::new(text.to_owned()))
}

fn unreadable(what: &str, reason: &str) -> Failure {
    Failure::Input {
        what: what.to_owned(),
        reason: reason.to_owned(),
    }
}
