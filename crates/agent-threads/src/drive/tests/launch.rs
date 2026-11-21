//! The command lines of the two programs (THR-040, the closed list of flags of TH18).

use std::path::PathBuf;

use uuid::Uuid;

use super::*;

fn launch(program: Program, mode: Mode) -> Launch {
    Launch {
        program,
        path: PathBuf::from("/opt/homebrew/bin/claude"),
        mode,
        model: Some(Model::new("sonnet").expect("a model")),
        effort: Some(Effort::Low),
        session: SessionStart::New(Uuid::nil()),
    }
}

#[test]
fn a_claude_command_line_names_its_mode_and_its_session() {
    let args = launch(Program::ClaudeCode, Mode::FullAccess)
        .invocation()
        .expect("absolute")
        .args;
    let joined = args.join(" ");
    assert!(joined.starts_with("-p --input-format stream-json --output-format stream-json"));
    assert!(joined.contains("--permission-prompt-tool stdio --permission-mode bypassPermissions"));
    assert!(joined.ends_with(
        "--model sonnet --effort low --session-id 00000000-0000-0000-0000-000000000000"
    ));
    for mode in [Mode::FullAccess, Mode::AcceptEdits, Mode::Ask] {
        let args = launch(Program::ClaudeCode, mode)
            .invocation()
            .expect("absolute")
            .args;
        for refused in [
            "--settings",
            "--setting-sources",
            "--allowedTools",
            "--add-dir",
            "--mcp-config",
            "--dangerously-skip-permissions",
        ] {
            assert!(
                !args.iter().any(|arg| arg == refused),
                "{refused} is not a flag of the runner"
            );
        }
    }
    let mut fork = launch(Program::ClaudeCode, Mode::Ask);
    fork.session = SessionStart::Fork(Uuid::nil());
    assert!(
        fork.invocation()
            .expect("absolute")
            .args
            .join(" ")
            .ends_with("--fork-session")
    );
}

#[test]
fn a_model_name_cannot_become_a_flag_or_a_second_argument() {
    for name in ["sonnet", "claude-opus-5-5", "opus[1m]", "gpt-6.1-sol"] {
        assert!(Model::new(name).is_ok(), "{name}");
    }
    for name in [
        "",
        "-p",
        "--settings",
        "a b",
        "a;b",
        "a\nb",
        &"x".repeat(65),
    ] {
        assert_eq!(
            Model::new(name),
            Err(DriveError::Refused { what: "model" }),
            "{name:?}"
        );
    }
}

#[test]
fn a_codex_command_line_is_the_server_alone_and_a_path_must_be_absolute() {
    let args = launch(Program::Codex, Mode::FullAccess)
        .invocation()
        .expect("absolute")
        .args;
    assert_eq!(args, ["app-server"]);
    let mut relative = launch(Program::Codex, Mode::Ask);
    relative.path = PathBuf::from("codex");
    assert_eq!(relative.invocation(), Err(DriveError::RelativeProgram));
}
