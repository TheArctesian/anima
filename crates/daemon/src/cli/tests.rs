//! The command line of the daemon: what each command takes and refuses.

use super::*;

const CODE: &str = "anima_enrol_AAAAAAAAAAAAAAAAAAAAAA";

fn args(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_owned).collect()
}

#[test]
fn hmd_055_version_takes_no_argument_and_prints_the_name_and_the_release_version() {
    assert!(matches!(parse(args("--version")), Ok(Command::Version)));
    assert!(matches!(parse(args("--version now")), Err(CliError::Usage)));
    assert!(matches!(parse(args("stop")), Ok(Command::Stop)));
    assert!(matches!(parse(args("stop now")), Err(CliError::Usage)));
    assert_eq!(
        version_line(),
        format!("anima-daemon {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn r_147_key_read_takes_one_key_name_and_forget_none() {
    let Ok(Command::KeyRead { name }) = parse(args("key read cloudflare.dns")) else {
        panic!("key read with a name reads");
    };
    assert_eq!(name, "cloudflare.dns");
    assert!(matches!(parse(args("key forget")), Ok(Command::KeyForget)));
    assert!(matches!(
        parse(args("key read Bad/Name")),
        Err(CliError::NotAKeyName)
    ));
    for line in [
        "key",
        "key read",
        "key read a b",
        "key forget now",
        "key list",
    ] {
        assert!(matches!(parse(args(line)), Err(CliError::Usage)), "{line}");
    }
}

#[test]
fn met_019_run_takes_no_other_argument() {
    assert!(matches!(
        parse(args("run")),
        Ok(Command::Run { hosted_hub: None })
    ));
    assert!(matches!(parse(args("threads")), Ok(Command::Threads)));
    assert!(matches!(parse(args("run now")), Err(CliError::Usage)));
}

#[test]
fn hmd_071_run_takes_the_absolute_data_folder_of_a_hosted_hub() {
    let hosted = parse(args("run --hosted-hub /var/lib/anima"));
    assert!(
        matches!(hosted, Ok(Command::Run { hosted_hub: Some(folder) }) if folder == std::path::Path::new("/var/lib/anima"))
    );
    assert!(matches!(
        parse(args("run --hosted-hub")),
        Err(CliError::Usage)
    ));
    assert!(matches!(
        parse(args("run --hosted-hub data")),
        Err(CliError::Usage)
    ));
    assert!(matches!(
        parse(args("run --hosted-hub /data more")),
        Err(CliError::Usage)
    ));
    assert!(matches!(parse(args("")), Err(CliError::Usage)));
}

#[test]
fn hmd_056_uninstall_takes_no_arguments() {
    assert!(matches!(parse(args("uninstall")), Ok(Command::Uninstall)));
    for line in ["uninstall --system", "uninstall now"] {
        assert!(matches!(parse(args(line)), Err(CliError::Usage)), "{line}");
    }
}

#[test]
fn met_019_the_enrol_hint_never_puts_the_code_on_a_command_line() {
    assert!(!ARGV_WARNING.contains("<code>"), "{ARGV_WARNING}");
    assert!(!ARGV_WARNING.contains("printf"), "{ARGV_WARNING}");
    assert!(
        ARGV_WARNING.contains("`anima-daemon enrol -` and paste the code at the hidden prompt")
    );
}

#[test]
fn met_019_install_is_for_the_user_unless_system_is_given() {
    assert!(matches!(
        parse(args("install")),
        Ok(Command::Install {
            target: Target::User
        })
    ));
    assert!(matches!(
        parse(args("install --system")),
        Ok(Command::Install {
            target: Target::System
        })
    ));
    for line in ["install --user", "install --system now", "install system"] {
        assert!(matches!(parse(args(line)), Err(CliError::Usage)), "{line}");
    }
}

#[test]
fn met_012_enrol_defaults_the_hub_and_reads_a_code_or_stdin() {
    let Ok(Command::Enrol { code, hub }) = parse(args(&format!("enrol {CODE}"))) else {
        panic!("enrol with a code reads");
    };
    assert!(matches!(code, CodeSource::Given(_)));
    assert_eq!(hub, Origin::default_hub());
    assert!(matches!(
        parse(args("enrol -")),
        Ok(Command::Enrol {
            code: CodeSource::Stdin,
            ..
        })
    ));
}

#[test]
fn met_020_enrol_takes_an_https_hub_in_either_spelling_and_refuses_http() {
    for line in [
        "enrol - --hub https://hub.example",
        "enrol --hub=https://hub.example -",
    ] {
        let Ok(Command::Enrol { hub, .. }) = parse(args(line)) else {
            panic!("{line}");
        };
        assert_eq!(hub.as_text(), "https://hub.example");
    }
    assert!(matches!(
        parse(args("enrol - --hub http://hub.example")),
        Err(CliError::Hub(OriginError::Scheme))
    ));
    let origin = if cfg!(windows) {
        "unix:C:/Users/me/Anima/link.sock"
    } else {
        "unix:/home/me/.config/anima-daemon/link.sock"
    };
    let line = format!("enrol - --hub {origin}");
    let Ok(Command::Enrol { hub, .. }) = parse(args(&line)) else {
        panic!("the socket of anima-link is a hub origin");
    };
    assert_eq!(hub.as_text(), origin);
    assert!(matches!(
        parse(args("enrol - --hub http://127.0.0.1:8443")),
        Err(CliError::Hub(OriginError::LinkPort))
    ));
}

#[test]
fn met_012_a_bad_code_is_refused_without_echoing_it() {
    let Err(error) = parse(args("enrol anima_enrol_short")) else {
        panic!("a short code is refused");
    };
    assert_eq!(error, CliError::NotACode);
    assert!(!error.to_string().contains("short"));
}

fn read(input: &[u8]) -> Result<EnrolmentCode, CliError> {
    read_code(input)
}

#[test]
fn met_012_a_code_on_standard_input_ends_at_its_line_feed_or_carriage_return_and_line_feed() {
    for input in [
        format!("{CODE}\n"),
        format!("{CODE}\r\n"),
        CODE.to_owned(),
        format!("{CODE}\nmore"),
    ] {
        assert!(read(input.as_bytes()).is_ok(), "{input:?}");
    }
}

#[test]
fn met_012_an_empty_a_split_or_a_padded_line_is_not_a_code() {
    let (head, tail) = CODE.split_at(10);
    for input in [
        String::new(),
        "\n".to_owned(),
        "\r\n".to_owned(),
        format!("{head}\n{tail}\n"),
        format!(" {CODE}\n"),
        format!("{CODE}\r\r\n"),
    ] {
        assert_eq!(
            read(input.as_bytes()).err(),
            Some(CliError::NotACode),
            "{input:?}"
        );
    }
}

#[test]
fn met_012_at_most_256_bytes_are_read_and_300_without_a_newline_are_not_a_code() {
    let long = vec![b'A'; 300];
    let mut input = std::io::Cursor::new(long);
    assert_eq!(read_code(&mut input).err(), Some(CliError::NotACode));
    assert_eq!(input.position(), 256, "the rest was never read");
}
