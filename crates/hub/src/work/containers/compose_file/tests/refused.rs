//! Each form that the converter refuses, with its fixed code and the position.

use super::super::{LARGEST_COMMAND, LARGEST_FILE, Refusal, RefusalCode, convert};

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

/// Expansions that a shell does with no `$`: each is refused, so that the file never holds
/// a text that the shell would have changed.
const EXPANDED: [(&str, RefusalCode, u32); 11] = [
    (
        "docker run -p {80,443}:80 nginx",
        RefusalCode::BraceExpansion,
        15,
    ),
    (
        "docker run -e A={1..3} nginx",
        RefusalCode::BraceExpansion,
        17,
    ),
    (
        "docker run -v ~/data:/data nginx",
        RefusalCode::TildeExpansion,
        15,
    ),
    (
        "docker run -v data:~/x nginx",
        RefusalCode::TildeExpansion,
        20,
    ),
    ("docker run -e A=* nginx", RefusalCode::Pattern, 17),
    (
        "docker run -e URL=http://x?a nginx",
        RefusalCode::Pattern,
        27,
    ),
    ("docker run -e A=[x] nginx", RefusalCode::Pattern, 17),
    (
        "docker run -e A=$[1+2] nginx",
        RefusalCode::ArithmeticExpansion,
        17,
    ),
    (
        "docker run -e A=${B:-'x'} nginx",
        RefusalCode::VariableNotKept,
        17,
    ),
    (
        "docker run -e A=${B:-\"x\"} nginx",
        RefusalCode::VariableNotKept,
        17,
    ),
    ("docker run\rnginx", RefusalCode::ControlCharacter, 11),
];

#[test]
fn host_022_each_expansion_of_a_shell_with_no_dollar_is_refused() {
    for (command, code, at) in EXPANDED {
        assert_eq!(convert(command), Err(Refusal { code, at }), "{command:?}");
    }
    // The same characters in quotes, or where a shell keeps them, stay as text.
    let kept = "docker run -e 'A={80,443}' -e \"B=~/x\" -e C='*?[' -e D={} -e E={a} -e F=a~b nginx";
    let done = convert(kept).expect("converted");
    for text in ["A={80,443}", "B=~/x", "C=*?[", "D={}", "E={a}", "F=a~b"] {
        assert!(
            done.compose_file.contains(text),
            "{text}\n{}",
            done.compose_file
        );
    }
}

/// The words that `word` gives for each number below `count`, each after a space.
fn each(count: usize, word: impl Fn(usize) -> String) -> String {
    (0..count).fold(String::new(), |mut line, number| {
        line.push(' ');
        line.push_str(&word(number));
        line
    })
}

#[test]
fn host_022_more_than_32_networks_or_aliases_is_refused() {
    let networks = |count: usize| each(count, |n| format!("--net n{n}"));
    let aliases = |count: usize| each(count, |n| format!("--network-alias a{n}"));
    let most = format!("docker run{}{} nginx", networks(32), aliases(32));
    assert!(convert(&most).is_ok());
    let command = format!("docker run{} nginx", networks(33));
    let refusal = convert(&command).expect_err("refused");
    assert_eq!(refusal.code, RefusalCode::TooManyNetworks);
    assert_eq!(
        refusal.at,
        u32::try_from(command.find(" --net n32").unwrap() + 2).unwrap()
    );
    let command = format!("docker run --net n{} nginx", aliases(33));
    assert_eq!(
        convert(&command).map_err(|refusal| refusal.code),
        Err(RefusalCode::TooManyNetworks)
    );
}

#[test]
fn host_022_a_file_larger_than_the_check_reads_is_refused() {
    // Each alias is written for each network: 32 aliases of 1,900 characters on 32 networks
    // is a command of about 62 KiB and a file of about 2 MiB.
    let networks = each(32, |n| format!("--net n{n}"));
    let aliases = each(32, |n| format!("--network-alias {n}{}", "a".repeat(1_900)));
    let command = format!("docker run{networks}{aliases} nginx");
    assert!(command.len() <= LARGEST_COMMAND);
    let refusal = Refusal {
        code: RefusalCode::FileTooLarge,
        at: 1,
    };
    assert_eq!(convert(&command), Err(refusal));
    // A file that is converted always passes the check of its size.
    let fits = format!("docker run -e A={} nginx", "\u{2028}".repeat(20_000));
    let done = convert(&fits).expect("converted");
    assert!(done.compose_file.len() <= LARGEST_FILE);
}
