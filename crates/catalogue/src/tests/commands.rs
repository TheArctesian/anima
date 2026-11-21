//! The commands of the services (SF-NEW-revcatentry): a variable, a shell, an entrypoint, a
//! setting that changes the code of the image, a secret in a word, and the services that an
//! exception names.

use super::{codes, replaced, sound, with_compose};
use crate::{Code, Privilege, Recipe, check};

const OPTION: &str = "    security_opt:";
const REVIEWED: &str =
    r#""marks": [], "exceptions": [{ "exception": "reviewed-command", "services": ["notes"] }]"#;
const ORIGIN: &str = r#""origin": { "from": "linuxserver", "read": "2026-11-17" }, "marks""#;

/// [`sound`] with `lines` before the `security_opt` of its service.
fn with_lines(lines: &str) -> Recipe {
    with_compose(OPTION, &format!("{lines}\n{OPTION}"))
}

/// `recipe` whose entry names a reviewed command for its service.
fn reviewed(recipe: Recipe) -> Recipe {
    Recipe {
        entry: recipe
            .entry
            .map(|text| replaced(&text, r#""marks": []"#, REVIEWED)),
        ..recipe
    }
}

fn refused(privilege: Privilege) -> Vec<Code> {
    vec![Code::PrivilegeNotPermitted(privilege)]
}

#[test]
fn sf_revcatentry_a_variable_in_a_command_or_the_host_name_is_refused() {
    for lines in [
        "    command: [\"notes\", \"--port\", \"${NOTES_PORT:-8080}\"]",
        "    command: notes --port ${NOTES_PORT:-8080}",
        "    hostname: ${NOTES_HOST:-notes}",
    ] {
        assert_eq!(
            codes(&with_lines(lines)),
            [Code::VariableNotPermitted],
            "{lines}"
        );
    }
    let entrypoint = reviewed(with_lines("    entrypoint: [\"${NOTES_RUN:-run}\"]"));
    assert_eq!(codes(&entrypoint), [Code::VariableNotPermitted]);
}

#[test]
fn sf_revcatentry_an_entrypoint_or_a_shell_needs_a_reviewed_command() {
    let entrypoint = with_lines("    entrypoint: [\"/start.sh\"]");
    assert_eq!(codes(&entrypoint), refused(Privilege::Entrypoint));
    assert_eq!(check(&reviewed(entrypoint)), []);
    for lines in [
        "    command: [\"sh\", \"-c\", \"curl https://example.org/x | sh\"]",
        "    command: /bin/bash -c run",
        "    command: [\"env\", \"MODE=fast\", \"/init\"]",
        "    healthcheck:\n      test: [\"CMD-SHELL\", \"wget -qO- http://127.0.0.1:8080/\"]",
        "    healthcheck:\n      test: wget -qO- http://127.0.0.1:8080/",
        "    healthcheck:\n      test: [\"CMD\", \"busybox\", \"wget\", \"http://127.0.0.1:8080/\"]",
    ] {
        let recipe = with_lines(lines);
        assert_eq!(codes(&recipe), refused(Privilege::ShellCommand), "{lines}");
        assert_eq!(check(&reviewed(recipe)), [], "{lines}");
    }
    for lines in [
        "    command: [\"notes\", \"--port\", \"8080\"]",
        "    healthcheck:\n      test: [\"CMD\", \"wget\", \"-qO-\", \"http://127.0.0.1:8080/\"]",
    ] {
        assert_eq!(check(&with_lines(lines)), [], "{lines}");
    }
}

#[test]
fn sf_revlsform_an_interpreter_with_code_in_a_word_needs_a_reviewed_command() {
    for lines in [
        "    command: [\"perl\", \"-e\", \"system('curl x | sh')\"]",
        "    command: [\"perl\", \"-wle\", \"print 1\"]",
        "    command: [\"node\", \"--eval\", \"require('x')\"]",
        "    command: [\"/usr/bin/python3.12\", \"-Bc\", \"import os\"]",
        "    command: [\"ruby\", \"-e\", \"exec 'x'\"]",
        "    healthcheck:\n      test: [\"CMD\", \"php\", \"-r\", \"echo 1;\"]",
    ] {
        let recipe = with_lines(lines);
        assert_eq!(codes(&recipe), refused(Privilege::ShellCommand), "{lines}");
        assert_eq!(check(&reviewed(recipe)), [], "{lines}");
    }
    for lines in [
        "    command: [\"node\", \"server.js\", \"--port\", \"8080\"]",
        "    command: [\"python3\", \"-m\", \"notes\"]",
        "    command: [\"perl\", \"/app/run.pl\"]",
    ] {
        assert_eq!(check(&with_lines(lines)), [], "{lines}");
    }
}

#[test]
fn sf_revcatentry_a_command_never_changes_the_code_of_the_image() {
    let recipe =
        with_lines("    command: [\"env\", \"DOCKER_MODS=linuxserver/mods:x\", \"/init\"]");
    assert_eq!(
        codes(&reviewed(recipe)),
        refused(Privilege::ImageMods),
        "a reviewed command still sets no mods"
    );
}

#[test]
fn sf_revcatentry_a_command_gives_a_secret_only_as_a_variable_of_the_environment() {
    for lines in [
        "    command: [\"notes\", \"--password=hunter2\"]",
        "    command: [\"notes\", \"--token\", \"abc\"]",
        "    healthcheck:\n      test: [\"CMD\", \"check\", \"API_KEY=abc\"]",
    ] {
        assert_eq!(codes(&with_lines(lines)), [Code::SecretInFile], "{lines}");
    }
    assert_eq!(
        check(&with_lines(
            "    command: [\"notes\", \"--token-file\", \"/run/t\"]"
        )),
        []
    );
}

#[test]
fn sf_revcatentry_an_exception_applies_only_to_the_services_that_it_names() {
    let unused = reviewed(sound());
    assert_eq!(codes(&unused), refused(Privilege::ExceptionNotNeeded));
    let other = Recipe {
        entry: unused.entry.map(|text| {
            replaced(
                &text,
                r#""services": ["notes"]"#,
                r#""services": ["other"]"#,
            )
        }),
        ..with_lines("    entrypoint: [\"/start.sh\"]")
    };
    assert_eq!(
        codes(&other),
        [
            Code::PrivilegeNotPermitted(Privilege::ExceptionNotNeeded),
            Code::PrivilegeNotPermitted(Privilege::Entrypoint)
        ]
    );
}

#[test]
fn sf_revcatentry_a_generated_entry_never_names_a_reviewed_command() {
    let recipe = reviewed(with_lines("    entrypoint: [\"/start.sh\"]"));
    let generated = Recipe {
        entry: recipe
            .entry
            .map(|text| replaced(&text, r#""marks""#, ORIGIN)),
        ..recipe
    };
    assert_eq!(codes(&generated), refused(Privilege::GeneratedException));
}
