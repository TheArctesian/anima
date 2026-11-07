//! Docker and systemd checks: the commands, the pure readers of their output and the
//! configuration (CHK-009, CHK-012, D-3).

mod config;
mod docker;
mod systemd;

use crate::{checks::probe::ProbeResult, sessions::CommandOutput};

/// Output that finished with exit status 0 and was read whole.
pub fn finished(stdout: &str) -> CommandOutput {
    CommandOutput {
        exit_status: Some(0),
        stdout: stdout.as_bytes().to_vec(),
        cut: false,
    }
}

/// What the reader answered, as a status word and the message: `up`, `starting`, `down`.
pub fn verdict(result: &ProbeResult) -> (&'static str, Option<&str>) {
    use crate::checks::probe::Judgement::{Answered, Starting, Unjudged};
    let word = match (result.ok, result.judgement) {
        (true, Answered) => "up",
        (true, Starting) => "starting",
        (false, Answered) => "down",
        (_, Unjudged) | (false, Starting) => "unexpected",
    };
    (word, result.message.as_deref())
}

/// The reasons no reader may read the output, shared by both readers. `read` is the reader
/// with the configured name already given.
pub fn assert_unreadable_outputs(read: impl Fn(&CommandOutput) -> ProbeResult, valid: &str) {
    let cut = CommandOutput {
        cut: true,
        exit_status: None,
        ..finished(valid)
    };
    assert_eq!(
        verdict(&read(&cut)),
        (
            "down",
            Some("the command output was longer than the hub reads")
        )
    );
    let cut_with_status = CommandOutput {
        cut: true,
        ..finished(valid)
    };
    assert_eq!(
        verdict(&read(&cut_with_status)),
        (
            "down",
            Some("the command output was longer than the hub reads")
        )
    );
    for exit_status in [Some(1), Some(127), None] {
        let failed = CommandOutput {
            exit_status,
            ..finished(valid)
        };
        assert_eq!(
            verdict(&read(&failed)),
            ("down", Some("the command exited with an error")),
            "{exit_status:?}"
        );
    }
    let not_utf8 = CommandOutput {
        stdout: b"\xff\xfe\x80 not text".to_vec(),
        ..finished("")
    };
    assert_eq!(
        verdict(&read(&not_utf8)),
        ("down", Some("the command output cannot be read"))
    );
}
