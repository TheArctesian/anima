//! The commands of a service (SF-NEW-revcatentry): its `entrypoint`, its `command` and the test
//! of its `healthcheck`, read as words. A shell in one of them can run any text, also a download
//! or a setting that changes the code of the image, so it needs a person who read it.

use compose_rules::Node;

/// The programs that run a text as a command.
const SHELLS: [&str; 7] = ["sh", "bash", "ash", "dash", "zsh", "busybox", "env"];
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

/// True when the command or the test of the health check runs a shell.
pub(crate) fn runs_shell(service: &Node) -> bool {
    let (test, shell) = health(service);
    shell
        || split(service.get("command"))
            .into_iter()
            .chain(test)
            .any(is_shell)
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
