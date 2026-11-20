//! Native roaming settings: the documented Windows locations for Claude Desktop and Zed.
use std::path::{Path, PathBuf};

pub(super) fn file(agent: &str, roaming: Option<&Path>) -> Option<PathBuf> {
    let relative = match agent {
        "claude-desktop" => "Claude/claude_desktop_config.json",
        "zed" => "Zed/settings.json",
        _ => return None,
    };
    Some(roaming?.join(relative))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_settings_follow_the_roaming_folder_without_guessing_a_unix_path() {
        let roaming = Path::new(r"D:\Profiles\User\Roaming");
        assert_eq!(
            file("zed", Some(roaming)),
            Some(roaming.join("Zed/settings.json"))
        );
        assert_eq!(
            file("claude-desktop", Some(roaming)),
            Some(roaming.join("Claude/claude_desktop_config.json"))
        );
        assert_eq!(file("zed", None), None);
    }
}
