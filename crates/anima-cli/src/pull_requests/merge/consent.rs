//! What the operator agrees to before a merge: the head commit they saw, and for a pull
//! request from a fork its head repository typed exactly (PRJ-029, PRJ-033, PRO-013).

use serde_json::Value;

use crate::failure::Failure;
use crate::forge::render::{flag, number, text};
use crate::prompt::Prompter;

/// Whether `text` is a full commit: 40 hex digits.
fn is_sha(text: &str) -> bool {
    text.len() == 40 && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// The first seven digits of a commit.
#[must_use]
pub fn short(sha: &str) -> &str {
    sha.get(..7).unwrap_or(sha)
}

/// The head commit to send as `expectedHeadSha`: the one given with `--sha`, which must be the
/// head just read, or the head just read when a person is `asked` about it at the terminal.
/// `--yes` asks no one, so it never stands for the head (PRJ-029).
///
/// # Errors
/// [`Failure::Usage`] for a `--sha` that is not 40 hex digits, or no `--sha` when no one is
/// asked; [`Failure::Conflict`] when the head moved since the operator looked.
pub fn expected_head(given: Option<&str>, head: &str, asked: bool) -> Result<String, Failure> {
    match given {
        Some(given) if !is_sha(given) => Err(Failure::Usage {
            message: "--sha takes the 40-digit head commit, as `anima pr get` shows it.".to_owned(),
            hint: None,
        }),
        Some(given) if !given.eq_ignore_ascii_case(head) => Err(Failure::Conflict {
            message: format!(
                "The pull request changed since you looked: its head is now {}. Nothing was merged; read it again.",
                short(head)
            ),
        }),
        Some(_) => Ok(head.to_owned()),
        None if asked => Ok(head.to_owned()),
        None => Err(Failure::Usage {
            message: "Without a question (--yes, --no-input or no terminal) a merge needs the head commit you looked at, as `anima pr get` shows it: pass --sha.".to_owned(),
            hint: None,
        }),
    }
}

/// `forkConfirmed` for the pull request: `None` when it is not from a fork. For one from a
/// fork the operator names its head repository exactly, with `--fork` or typed at the
/// terminal. `--yes` never stands for it (PRO-013).
///
/// # Errors
/// [`Failure::Usage`] for `--fork` on a pull request that is not from a fork, or no name when no
/// one can type it; [`Failure::Forbidden`] when the name is not the fork's, or the fork was
/// deleted.
pub fn fork(
    given: Option<&str>,
    pull: &Value,
    prompter: Prompter,
) -> Result<Option<String>, Failure> {
    if !flag(pull, "fromFork") {
        return match given {
            Some(_) => Err(Failure::Usage {
                message: "The pull request is not from a fork: leave out --fork.".to_owned(),
                hint: None,
            }),
            None => Ok(None),
        };
    }
    let Some(head) = pull.get("headRepository").and_then(Value::as_str) else {
        return Err(Failure::Forbidden {
            message: "The pull request is from a fork that was deleted, so its name cannot be confirmed. Nothing was merged."
                .to_owned(),
            hint: None,
        });
    };
    let typed = match given {
        Some(given) => given.to_owned(),
        None if !prompter.interactive() => {
            return Err(Failure::missing(
                "The fork's name, typed (--yes does not stand for it)",
                "--fork <owner/name>",
            ));
        }
        None => prompter.ask(
            &format!("It is from the fork {head}. Type that name to merge it"),
            "--fork",
        )?,
    };
    if typed == head {
        Ok(Some(typed))
    } else {
        Err(Failure::Forbidden {
            message: format!("That is not the fork's name, {head}. Nothing was merged."),
            hint: None,
        })
    }
}

/// What the operator sees before a merge: the title, the head, the fork, whether GitHub says it
/// can merge, the checks and the method.
#[must_use]
pub fn summary(shown: &str, pull: &Value, method: &str) -> String {
    let mut lines = vec![
        format!("{shown}: {}", text(pull, "title")),
        format!(
            "Head: {} at {}",
            text(pull, "headBranch"),
            text(pull, "headSha")
        ),
    ];
    if flag(pull, "fromFork") {
        lines.push(format!("From a fork: {}", text(pull, "headRepository")));
    }
    let mergeability = match pull.get("mergeability").filter(|value| !value.is_null()) {
        Some(found) if flag(found, "canMerge") => format!("{} (can merge)", text(found, "state")),
        Some(found) => format!("{} (cannot merge now)", text(found, "state")),
        None => "not known".to_owned(),
    };
    lines.push(format!("Merge state: {mergeability}"));
    if let Some(checks) = pull.get("checks").filter(|value| !value.is_null()) {
        lines.push(format!(
            "Checks: {}: {} passed, {} failed, {} pending",
            text(checks, "state"),
            number(checks, "passed"),
            number(checks, "failed"),
            number(checks, "pending")
        ));
    }
    lines.push(format!("Method: {method}"));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";

    #[test]
    fn prj_033_the_head_sent_is_the_head_the_operator_saw() {
        assert_eq!(expected_head(None, HEAD, true).ok().as_deref(), Some(HEAD));
        assert_eq!(
            expected_head(Some(HEAD), HEAD, false).ok().as_deref(),
            Some(HEAD)
        );
        let upper = HEAD.to_ascii_uppercase();
        assert_eq!(
            expected_head(Some(&upper), HEAD, false).ok().as_deref(),
            Some(HEAD)
        );
        let moved = "f".repeat(40);
        let failure = expected_head(Some(&moved), HEAD, true).expect_err("moved");
        assert_eq!(failure.exit_code(), 6);
        for wrong in ["0123456", "", &"g".repeat(40)] {
            let failure = expected_head(Some(wrong), HEAD, true).expect_err("not a sha");
            assert_eq!(failure.exit_code(), 2, "{wrong}");
        }
        let failure = expected_head(None, HEAD, false).expect_err("no one sees it");
        assert!(failure.to_string().contains("--sha"), "{failure}");
    }

    #[test]
    fn pro_013_yes_never_stands_for_the_name_of_the_fork() {
        let pull = json!({ "fromFork": true, "headRepository": "mallory/anima" });
        let yes_without_terminal = Prompter::new(false, false, true);
        let failure = fork(None, &pull, yes_without_terminal).expect_err("--yes is not a name");
        assert_eq!(failure.exit_code(), 2);
        assert!(failure.to_string().contains("--fork"), "{failure}");
        let confirmed = fork(Some("mallory/anima"), &pull, yes_without_terminal);
        assert_eq!(confirmed.ok().flatten().as_deref(), Some("mallory/anima"));
        for wrong in ["Mallory/anima", "mallory/anima ", "acme/anima"] {
            let failure = fork(Some(wrong), &pull, yes_without_terminal).expect_err(wrong);
            assert_eq!(failure.exit_code(), 5, "{wrong}");
        }
        let deleted = json!({ "fromFork": true, "headRepository": null });
        let failure = fork(Some("mallory/anima"), &deleted, yes_without_terminal);
        assert_eq!(failure.expect_err("deleted").exit_code(), 5);
    }

    #[test]
    fn pro_013_a_pull_request_from_the_same_repository_needs_no_fork_name() {
        let pull = json!({ "fromFork": false, "headRepository": "acme/anima" });
        let prompter = Prompter::new(false, false, false);
        assert_eq!(fork(None, &pull, prompter).ok(), Some(None));
        let failure = fork(Some("acme/anima"), &pull, prompter).expect_err("not a fork");
        assert_eq!(failure.exit_code(), 2);
    }

    #[test]
    fn cmd_020_a_title_cannot_add_a_line_to_the_merge_summary() {
        let pull = json!({
            "title": "Fix typo\nChecks: PASSED: 9 passed, 0 failed, 0 pending",
            "headBranch": "fix", "headSha": HEAD, "fromFork": false,
            "mergeability": { "state": "CLEAN", "canMerge": true },
            "checks": { "state": "FAILED", "passed": 0, "failed": 3, "pending": 0 },
        });
        let shown = crate::output::text::clean_lines(&summary("acme/anima#7", &pull, "squash"));
        assert_eq!(shown.lines().count(), 5, "{shown}");
    }
}
