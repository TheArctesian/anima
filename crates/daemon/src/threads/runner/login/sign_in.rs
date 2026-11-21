//! "Not logged in" of Claude Code under the holder (TH99). Claude Code 2.1.311 reads its sign-in
//! with `security find-generic-password -w` and waits 2 seconds; on a busy Mac the read takes
//! longer (14 to 22 seconds measured on 2026-11-21 at a load of 60), the read fails, and Claude
//! Code says "Not logged in" though the person is signed in. The runner looks whether the item
//! is in the Keychain, never at its secret, and says "sign in" only when it is not there.

use std::time::Duration;

/// The service of the Keychain item of Claude Code when no configuration folder is set.
const SERVICE: &str = "Claude Code-credentials";
/// The time the Keychain has to say whether the item is there.
const LOOK_WAIT: Duration = Duration::from_secs(5);
/// The exit code of `security` for an item that is not there.
const NOT_FOUND: i32 = 44;

/// What the runner found of the sign-in of Claude Code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignIn {
    /// The item is in the Keychain: Claude Code could not read it in time.
    Kept,
    /// The Keychain did not answer the runner in time either.
    Slow,
    /// No item: the person is not signed in.
    Missing,
    /// The runner cannot tell (a configuration folder of Claude Code, or not a Mac).
    Unknown,
}

/// Looks for the sign-in of Claude Code with the environment `env` of its program.
pub async fn of_claude(env: &[(String, String)]) -> SignIn {
    let named = |wanted: &str| env.iter().any(|(name, _)| name == wanted);
    if !cfg!(target_os = "macos")
        || named("CLAUDE_CONFIG_DIR")
        || named("CLAUDE_SECURESTORAGE_CONFIG_DIR")
    {
        return SignIn::Unknown;
    }
    let Some((_, user)) = env.iter().find(|(name, _)| name == "USER") else {
        return SignIn::Unknown;
    };
    // Without `-w`: `security` says whether the item is there and never prints its secret.
    let look = tokio::process::Command::new("/usr/bin/security")
        .args(["find-generic-password", "-a", user, "-s", SERVICE])
        .env_clear()
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .status();
    match tokio::time::timeout(LOOK_WAIT, look).await {
        Err(_) => SignIn::Slow,
        Ok(Ok(status)) if status.success() => SignIn::Kept,
        Ok(Ok(status)) if status.code() == Some(NOT_FOUND) => SignIn::Missing,
        Ok(_) => SignIn::Unknown,
    }
}

/// The words for the person after "Not logged in".
#[must_use]
pub const fn words(found: SignIn) -> &'static str {
    match found {
        SignIn::Kept => {
            "Claude Code said \"Not logged in\", but its sign-in is in the Keychain. Claude Code waits only 2 seconds for the Keychain, and on a busy computer the answer comes later. Send the prompt again; you do not need to sign in."
        }
        SignIn::Slow => {
            "Claude Code said \"Not logged in\", and the Keychain did not answer the runner in 5 seconds either: the computer is too busy for Claude Code to read its sign-in (it waits 2 seconds). Send the prompt again when the computer is less busy; you do not need to sign in."
        }
        SignIn::Missing => {
            "Claude Code is not signed in. Sign in to Claude Code in a terminal: run `claude`, then `/login`. Then send the prompt again."
        }
        SignIn::Unknown => {
            "Claude Code said \"Not logged in\". If `claude` works in a terminal, the computer was too busy for it to read its sign-in: send the prompt again. If it does not, sign in there (`claude`, then `/login`)."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn th99_a_configuration_folder_or_no_user_cannot_be_told() {
        let folder = [
            ("USER".to_owned(), "me".to_owned()),
            ("CLAUDE_CONFIG_DIR".to_owned(), "/x".to_owned()),
        ];
        assert_eq!(of_claude(&folder).await, SignIn::Unknown);
        assert_eq!(of_claude(&[]).await, SignIn::Unknown);
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn th99_an_account_with_no_item_is_missing() {
        let env = [("USER".to_owned(), "anima-test-no-such-account".to_owned())];
        assert_eq!(of_claude(&env).await, SignIn::Missing);
    }

    #[test]
    fn th99_only_a_missing_item_asks_the_person_to_sign_in() {
        assert!(words(SignIn::Missing).starts_with("Claude Code is not signed in. Sign in"));
        for found in [SignIn::Kept, SignIn::Slow] {
            assert!(words(found).contains("you do not need to sign in"));
        }
    }
}
