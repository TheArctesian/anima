//! What the command line reads from its process: environment variables and whether each
//! standard stream is a terminal. `main` reads it once; everything else takes it as a
//! parameter, so tests set it by running the program with another environment.

use std::ffi::OsString;
use std::io::IsTerminal as _;
use std::path::PathBuf;

use zeroize::Zeroizing;

/// The process's view of its surroundings. `Debug` is not derived: `token` is a secret.
pub struct Environment {
    /// `ANIMA_HUB` (CMD-013).
    pub hub: Option<String>,
    /// `ANIMA_TOKEN`, used before the stored token (CMD-013).
    pub token: Option<Zeroizing<String>>,
    /// `ANIMA_TOKEN_STORE=file`: keep the token in the 0600 file, not the OS key store.
    pub token_file: bool,
    /// `$XDG_CONFIG_HOME/anima`, or `~/.config/anima` (CMD-015).
    pub config_dir: Option<PathBuf>,
    /// `NO_COLOR` is set and not empty (CMD-027).
    pub no_colour: bool,
    pub terminals: Terminals,
}

/// Which standard streams are terminals (CMD-023, CMD-027).
#[derive(Clone, Copy, Debug, Default)]
pub struct Terminals {
    pub stdin: bool,
    pub stdout: bool,
}

impl Environment {
    /// Reads this process's environment and streams.
    #[must_use]
    pub fn from_process() -> Self {
        let value = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
        Self {
            hub: value("ANIMA_HUB").and_then(|value| value.into_string().ok()),
            token: value("ANIMA_TOKEN")
                .and_then(|value| value.into_string().ok())
                .map(Zeroizing::new),
            token_file: value("ANIMA_TOKEN_STORE").is_some_and(|value| value == "file"),
            config_dir: config_dir(value("XDG_CONFIG_HOME"), value("HOME")),
            no_colour: value("NO_COLOR").is_some(),
            terminals: Terminals {
                stdin: std::io::stdin().is_terminal(),
                stdout: std::io::stdout().is_terminal(),
            },
        }
    }
}

/// `$XDG_CONFIG_HOME/anima` when that is an absolute path (the XDG rule), else
/// `$HOME/.config/anima`.
fn config_dir(xdg: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let xdg = xdg.map(PathBuf::from).filter(|path| path.is_absolute());
    let base = xdg.or_else(|| home.map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("anima"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_015_the_config_folder_follows_xdg_then_home() {
        let dir = config_dir(Some("/x/cfg".into()), Some("/home/op".into()));
        assert_eq!(dir, Some(PathBuf::from("/x/cfg/anima")));
        let dir = config_dir(Some("relative".into()), Some("/home/op".into()));
        assert_eq!(dir, Some(PathBuf::from("/home/op/.config/anima")));
        assert_eq!(config_dir(None, None), None);
    }
}
