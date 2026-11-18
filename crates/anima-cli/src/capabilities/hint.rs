//! The repair hint for a tool that is not usable: a fixed sentence chosen by the tool and the
//! reason, never text from the device (CAP-024).

use super::documents::capability_list::NotUsableReason;

/// What the operator can do about `tool` being not usable for `reason`.
#[must_use]
pub fn hint(tool: &str, reason: &NotUsableReason) -> &'static str {
    match (tool, reason) {
        ("xcodebuild", NotUsableReason::EXITED | NotUsableReason::NO_VERSION) => {
            "Install Xcode from the App Store, open it once, then run \
             sudo xcode-select --switch /Applications/Xcode.app on the device."
        }
        ("rustc" | "cargo" | "cargo-nextest", NotUsableReason::EXITED) => {
            "Install a default Rust toolchain on the device: rustup default stable. The daemon \
             never lets rustup download one."
        }
        ("claude" | "codex", NotUsableReason::EXITED) => {
            "Update or reinstall it on the device; the daemon runs it with an empty home folder, \
             so it must print its version without a sign-in."
        }
        (_, NotUsableReason::TIMED_OUT) => {
            "Its version check ran past 5 seconds: run it with --version on the device to see \
             what it waits for."
        }
        (_, NotUsableReason::EXITED) => {
            "It could not start or exited with a failure: run it with --version on the device \
             to see the error, then repair or reinstall it."
        }
        (_, NotUsableReason::NO_VERSION) => {
            "It printed no version the daemon reads: it may be another program of the same name, \
             or a newer output format that a newer daemon reads."
        }
        (_, NotUsableReason::FAILED | NotUsableReason::Other(_)) => {
            "Update the hub and this command line to the daemon's version to see why."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_024_each_reason_has_a_sentence_and_some_tools_a_sentence_of_their_own() {
        let reasons = [
            NotUsableReason::TIMED_OUT,
            NotUsableReason::EXITED,
            NotUsableReason::NO_VERSION,
            NotUsableReason::FAILED,
            NotUsableReason::Other("denied".to_owned()),
        ];
        for reason in &reasons {
            let sentence = hint("bun", reason);
            assert!(sentence.ends_with('.'), "{sentence}");
        }
        assert!(hint("xcodebuild", &NotUsableReason::EXITED).contains("xcode-select"));
        assert!(hint("cargo", &NotUsableReason::EXITED).contains("rustup default"));
        assert!(hint("codex", &NotUsableReason::EXITED).contains("empty home"));
        assert_eq!(
            hint("cargo", &NotUsableReason::TIMED_OUT),
            hint("bun", &NotUsableReason::TIMED_OUT)
        );
        assert_eq!(
            hint("bun", &NotUsableReason::Other("denied".to_owned())),
            hint("bun", &NotUsableReason::FAILED),
            "a reason this version does not know"
        );
    }
}
