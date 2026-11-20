//! Each form that the converter refuses, with its fixed code and the position.

use super::super::{LARGEST_COMMAND, Refusal, RefusalCode, convert};

/// A command, the code of its refusal and the position where the converter stops.
const REFUSED: [(&str, RefusalCode, u32); 28] = [
    (
        "docker run -v $(pwd):/app nginx",
        RefusalCode::CommandSubstitution,
        15,
    ),
    (
        "docker run -v \"$(pwd)\":/app nginx",
        RefusalCode::CommandSubstitution,
        16,
    ),
    (
        "docker run -v `pwd`:/app nginx",
        RefusalCode::CommandSubstitution,
        15,
    ),
    (
        "docker run -e \"A=`id`\" nginx",
        RefusalCode::CommandSubstitution,
        18,
    ),
    ("docker run nginx | tee log", RefusalCode::Pipe, 18),
    (
        "docker run nginx; reboot",
        RefusalCode::MoreThanOneCommand,
        17,
    ),
    (
        "docker run nginx && reboot",
        RefusalCode::MoreThanOneCommand,
        18,
    ),
    (
        "docker run nginx || reboot",
        RefusalCode::MoreThanOneCommand,
        18,
    ),
    ("docker run nginx &", RefusalCode::MoreThanOneCommand, 18),
    (
        "docker run nginx\nreboot",
        RefusalCode::MoreThanOneCommand,
        17,
    ),
    ("docker run nginx > out.txt", RefusalCode::Redirection, 18),
    ("docker run nginx 2>&1", RefusalCode::Redirection, 19),
    ("docker run -i nginx < in.txt", RefusalCode::Redirection, 21),
    ("(docker run nginx)", RefusalCode::ShellSyntax, 1),
    (
        "docker run nginx # the web server",
        RefusalCode::ShellSyntax,
        18,
    ),
    ("docker run -e A=$1 nginx", RefusalCode::VariableNotKept, 17),
    ("docker run -e A=$? nginx", RefusalCode::VariableNotKept, 17),
    ("docker run -e A=$$ nginx", RefusalCode::VariableNotKept, 17),
    (
        "docker run -e A=${B%.txt} nginx",
        RefusalCode::VariableNotKept,
        17,
    ),
    (
        "docker run -e A=${#B} nginx",
        RefusalCode::VariableNotKept,
        17,
    ),
    (
        "docker run -e A=${B:-$C} nginx",
        RefusalCode::VariableNotKept,
        17,
    ),
    (
        "docker run -e A=${B nginx",
        RefusalCode::VariableNotKept,
        17,
    ),
    (
        "docker run -e A=$'x' nginx",
        RefusalCode::VariableNotKept,
        17,
    ),
    ("docker run -e 'A=b nginx", RefusalCode::UnclosedQuote, 15),
    ("docker run -e \"A=b nginx", RefusalCode::UnclosedQuote, 15),
    (
        "docker run -e A=\u{7} nginx",
        RefusalCode::ControlCharacter,
        17,
    ),
    (
        "docker run -e 'A=\u{0}' nginx",
        RefusalCode::ControlCharacter,
        18,
    ),
    (
        "docker run -e \"A=\u{1b}[0m\" nginx",
        RefusalCode::ControlCharacter,
        18,
    ),
];

#[test]
fn host_022_each_form_that_a_shell_would_run_or_fill_is_refused() {
    for (command, code, at) in REFUSED {
        assert_eq!(convert(command), Err(Refusal { code, at }), "{command}");
    }
}

#[test]
fn host_022_a_text_that_is_not_one_docker_run_command_is_refused() {
    for (command, code, at) in [
        ("", RefusalCode::NoCommand, 1),
        ("  \n\t ", RefusalCode::NoCommand, 1),
        ("podman run nginx", RefusalCode::NotDockerRun, 1),
        ("docker ps", RefusalCode::NotDockerRun, 8),
        ("docker", RefusalCode::NotDockerRun, 6),
        ("docker compose up", RefusalCode::NotDockerRun, 8),
        ("docker -H tcp://x run nginx", RefusalCode::NotDockerRun, 8),
        ("docker container ls", RefusalCode::NotDockerRun, 18),
        ("sudo -E docker run nginx", RefusalCode::NotDockerRun, 6),
        ("A=b docker run nginx", RefusalCode::NotDockerRun, 1),
        ("run nginx", RefusalCode::NotDockerRun, 1),
        ("docker run", RefusalCode::NoImage, 10),
        ("docker run -d --name web", RefusalCode::NoImage, 24),
        ("docker run -d --", RefusalCode::NoImage, 16),
        ("docker run --name", RefusalCode::OptionWithoutValue, 17),
        ("docker run -dp", RefusalCode::OptionWithoutValue, 14),
        ("docker run -e", RefusalCode::OptionWithoutValue, 13),
    ] {
        assert_eq!(convert(command), Err(Refusal { code, at }), "{command:?}");
    }
}

#[test]
fn host_022_a_command_of_more_than_64_kib_is_not_read() {
    let large = format!("docker run -e A={} nginx", "a".repeat(LARGEST_COMMAND));
    let refusal = Refusal {
        code: RefusalCode::TooLarge,
        at: 1,
    };
    assert_eq!(convert(&large), Err(refusal));
    let fits = format!(
        "docker run -e A={} nginx",
        "a".repeat(LARGEST_COMMAND - 100)
    );
    assert!(convert(&fits).is_ok());
}
