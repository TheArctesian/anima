//! What git prints about a remote, read without trusting it: the refs of `ls-remote`, the
//! result lines of `push --porcelain`, and error text made safe to show.

use serde::Serialize;

use super::scratch_ref::{PREFIX, ScratchRef, is_hash};

/// How much of git's error text a failure carries.
const REASON_CHARACTERS: usize = 400;

/// One scratch ref on the remote and the commit it holds.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct RemoteScratchRef {
    #[serde(rename = "ref")]
    pub scratch: ScratchRef,
    pub commit: String,
}

/// The scratch refs in `git ls-remote` output (`<hash>\t<ref>` lines). A line that is not a
/// hash and a safe ref under [`PREFIX`] is left out: git matches its pattern against the end
/// of a ref, so `refs/x/refs/anima/wip/y` would match too.
#[must_use]
pub fn scratch_refs(stdout: &str) -> Vec<RemoteScratchRef> {
    stdout
        .lines()
        .filter_map(|line| {
            let (commit, name) = line.split_once('\t')?;
            let scratch = ScratchRef::parse(name.strip_prefix(PREFIX)?).ok()?;
            is_hash(commit).then(|| RemoteScratchRef {
                scratch,
                commit: commit.to_owned(),
            })
        })
        .collect()
}

/// Why `git push --porcelain` refused a ref: the text of its first `!` line.
#[must_use]
pub fn rejection(stdout: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        let rest = line.strip_prefix('!')?;
        let reason = rest.rsplit('\t').next().unwrap_or(rest).trim();
        Some(safe(reason))
    })
}

/// git's error text, safe to show: the last lines, the user and password of every URL taken
/// out (LOG-007), and at most 400 characters.
#[must_use]
pub fn safe(stderr: &str) -> String {
    let lines: Vec<&str> = stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let start = lines.len().saturating_sub(3);
    let text = lines[start..]
        .iter()
        .map(|line| without_credentials(line))
        .collect::<Vec<_>>()
        .join(" ");
    let mut shown: String = text.chars().take(REASON_CHARACTERS).collect();
    if text.chars().count() > REASON_CHARACTERS {
        shown.push('…');
    }
    if shown.is_empty() {
        "git gave no reason".to_owned()
    } else {
        shown
    }
}

/// `line` with each `scheme://user:password@host` written as `scheme://***@host`.
fn without_credentials(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(at) = rest.find("://") {
        let (before, after) = rest.split_at(at + 3);
        out.push_str(before);
        let authority_end = after
            .find(|c: char| c == '/' || c == '\'' || c == '"' || c.is_whitespace())
            .unwrap_or(after.len());
        let authority = &after[..authority_end];
        match authority.rfind('@') {
            Some(user_end) => {
                out.push_str("***");
                out.push_str(&authority[user_end..]);
            }
            None => out.push_str(authority),
        }
        rest = &after[authority_end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "0123456789abcdef0123456789abcdef01234567";
    const B: &str = "fedcba9876543210fedcba9876543210fedcba98";

    #[test]
    fn job_017_ls_remote_keeps_only_safe_scratch_refs() {
        let stdout = format!(
            "{A}\trefs/anima/wip/0123456789abcdef\n\
             {B}\trefs/heads/main\n\
             {B}\trefs/remotes/x/refs/anima/wip/aaaa\n\
             {B}\trefs/anima/wip/a/b\n\
             not-a-hash\trefs/anima/wip/bbbb\n\
             {B}\n\
             \n\
             {B}\trefs/anima/wip/fedcba9876543210"
        );
        let found = scratch_refs(&stdout);
        let names: Vec<&str> = found.iter().map(|row| row.scratch.as_str()).collect();
        assert_eq!(
            names,
            [
                "refs/anima/wip/0123456789abcdef",
                "refs/anima/wip/fedcba9876543210"
            ]
        );
        assert_eq!(found[0].commit, A);
    }

    #[test]
    fn job_016_a_porcelain_bang_line_is_the_rejection() {
        let stdout = "To ../remote.git\n\
                      !\tabc:refs/anima/wip/x\t[rejected] (non-fast-forward)\n\
                      Done\n";
        assert_eq!(
            rejection(stdout).as_deref(),
            Some("[rejected] (non-fast-forward)")
        );
        assert_eq!(
            rejection("To x\n*\tabc:refs/anima/wip/x\t[new reference]\nDone\n"),
            None
        );
    }

    #[test]
    fn log_007_credentials_in_a_url_are_never_shown() {
        let stderr = "fatal: unable to access 'https://user:ghp_s3cret@github.com/o/r.git/': \
                      Could not resolve host\n";
        let shown = safe(stderr);
        assert!(
            !shown.contains("ghp_s3cret") && !shown.contains("user:"),
            "{shown}"
        );
        assert!(shown.contains("https://***@github.com/o/r.git/"), "{shown}");
        assert_eq!(
            safe("fatal: https://github.com/o/r and ssh://git@h:22/x"),
            "fatal: https://github.com/o/r and ssh://***@h:22/x"
        );
    }

    #[test]
    fn job_016_error_text_is_short_and_never_empty() {
        assert_eq!(safe(""), "git gave no reason");
        let long = "x".repeat(1000);
        assert_eq!(safe(&long).chars().count(), REASON_CHARACTERS + 1);
        assert_eq!(safe("one\ntwo\nthree\nfour\n"), "two three four");
    }
}
