//! The commands of a service (SF-251): its `entrypoint`, its `command` and the test
//! of its `healthcheck`, read as words. A shell in one of them can run any text, also a download
//! or a setting that changes the code of the image, so it needs a person who read it.

use compose_rules::Node;

/// The programs that run a text as a command.
const SHELLS: [&str; 7] = ["sh", "bash", "ash", "dash", "zsh", "busybox", "env"];
/// The interpreters that run code given in a word, each with the letters of its short options
/// and its long options that take that code (SF-252).
const INLINE: [(&str, &str, &[&str]); 9] = [
    ("perl", "eE", &[]),
    ("node", "ep", &["--eval", "--print"]),
    ("nodejs", "ep", &["--eval", "--print"]),
    ("bun", "ep", &["--eval", "--print"]),
    ("python", "c", &[]),
    ("pypy", "c", &[]),
    ("ruby", "e", &[]),
    ("php", "r", &[]),
    ("lua", "e", &[]),
];
/// The interpreters whose subcommand `eval` runs code given in a word.
const EVAL: [&str; 1] = ["deno"];
/// The versions of awk: a program that calls `system`, reads with `getline` or writes to or
/// reads from a pipe `|` runs a command.
const AWK: [&str; 4] = ["awk", "gawk", "mawk", "nawk"];
/// The parts of an awk program that run a command.
const AWK_COMMAND: [&str; 3] = ["system", "getline", "|"];
/// The variables of the init of `LinuxServer` that get or install code at the start.
const MODS: [&str; 3] = ["DOCKER_MODS", "INSTALL_PACKAGES", "INSTALL_PIP_PACKAGES"];

/// The words of a key that is one text or a list of texts. Each text is cut at white space.
fn split(node: Option<&Node>) -> Vec<&str> {
    match node {
        Some(Node::Text(text)) => text.split_whitespace().collect(),
        Some(Node::List(items)) => items
            .iter()
            .filter_map(Node::text)
            .flat_map(str::split_whitespace)
            .collect(),
        _ => Vec::new(),
    }
}

/// The words of `entrypoint`.
pub(crate) fn entrypoint(service: &Node) -> Vec<&str> {
    split(service.get("entrypoint"))
}

/// The words of the test of the health check, and true when a shell runs them: the form of
/// one text, and the form `CMD-SHELL`.
fn health(service: &Node) -> (Vec<&str>, bool) {
    let test = service
        .get("healthcheck")
        .and_then(|check| check.get("test"));
    match test {
        Some(Node::Text(_)) => (split(test), true),
        Some(Node::List(items)) => {
            let words = items
                .iter()
                .skip(1)
                .filter_map(Node::text)
                .flat_map(str::split_whitespace)
                .collect();
            let shell = items.first().and_then(Node::text) == Some("CMD-SHELL");
            (words, shell)
        }
        _ => (Vec::new(), false),
    }
}

/// Each word of the entrypoint, the command and the test of the health check.
pub(crate) fn words(service: &Node) -> Vec<&str> {
    let mut found = entrypoint(service);
    found.extend(split(service.get("command")));
    found.extend(health(service).0);
    found
}

/// True when a word runs a text as a command: a shell, `env`, or the option `-c`.
fn is_shell(word: &str) -> bool {
    let program = word.rsplit('/').next().unwrap_or(word);
    SHELLS.contains(&program) || word == "-c"
}

/// The short and long options that take inline code for the program of `word`, if it is an
/// interpreter: `python3.12` is `python`.
fn inline_options(word: &str) -> Option<(&'static str, &'static [&'static str])> {
    let bare = program(word)
        .trim_end_matches(|character: char| character.is_ascii_digit() || character == '.');
    INLINE
        .iter()
        .find(|(name, _, _)| *name == bare)
        .map(|(_, short, long)| (*short, *long))
}

/// True when `option` is one of the inline-code options: a short cluster such as `-le` that
/// holds one of the letters `short`, or one of `long`, also with `=value`.
fn is_inline_option(option: &str, short: &str, long: &[&str]) -> bool {
    match option.strip_prefix("--") {
        Some(_) => long
            .iter()
            .any(|name| option == *name || option.starts_with(&format!("{name}="))),
        None => option
            .strip_prefix('-')
            .is_some_and(|letters| letters.chars().any(|letter| short.contains(letter))),
    }
}

/// The name of the program of `word`, with no folder: `/usr/bin/gawk` is `gawk`.
fn program(word: &str) -> &str {
    word.rsplit('/').next().unwrap_or(word)
}

/// True when an interpreter in `words` is followed by an option that takes code in a word, as
/// `perl -e`, `node --eval` or `deno eval`, or awk by a program that runs a command: it runs any
/// text, as a shell does.
fn runs_inline_code(words: &[&str]) -> bool {
    words.iter().enumerate().any(|(at, word)| {
        let after = &words[at + 1..];
        let inline = inline_options(word).is_some_and(|(short, long)| {
            after
                .iter()
                .any(|option| is_inline_option(option, short, long))
        });
        let eval = EVAL.contains(&program(word)) && after.first() == Some(&"eval");
        let awk = AWK.contains(&program(word))
            && (after.iter()).any(|text| AWK_COMMAND.iter().any(|call| text.contains(call)));
        inline || eval || awk
    })
}

/// True when the command or the test of the health check runs a shell, or an interpreter with
/// code in a word.
pub(crate) fn runs_shell(service: &Node) -> bool {
    let (test, shell) = health(service);
    let found: Vec<&str> = split(service.get("command"))
        .into_iter()
        .chain(test)
        .collect();
    shell || found.iter().copied().any(is_shell) || runs_inline_code(&found)
}

/// True when a word names a variable that gets or installs code at the start.
pub(crate) fn sets_mods(service: &Node) -> bool {
    words(service)
        .into_iter()
        .any(|word| MODS.iter().any(|name| word.contains(name)))
}

/// True when a word gives a value to a name that marks a secret: `NAME=value`,
/// `--name=value`, or `--name value`. A name that ends with `FILE` names the path of a file
/// that holds the secret, not the secret.
pub(crate) fn secret_literal(service: &Node, is_secret_name: fn(&str) -> bool) -> bool {
    let is_secret =
        |name: &str| is_secret_name(name) && !name.to_ascii_uppercase().ends_with("FILE");
    let found = words(service);
    found.iter().enumerate().any(|(at, word)| {
        let bare = word.trim_start_matches('-');
        match bare.split_once('=') {
            Some((name, value)) => is_secret(name) && !value.is_empty(),
            None => {
                word.starts_with('-')
                    && is_secret(bare)
                    && found.get(at + 1).is_some_and(|next| !next.starts_with('-'))
            }
        }
    })
}
