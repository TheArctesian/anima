//! Fills the binary's path into the unit texts (MET-019): the path is checked first, so a
//! hostile or odd path never reaches a file that systemd or launchd reads.

use std::path::Path;

use super::{LAUNCH_AGENT, SYSTEMD_SYSTEM, SYSTEMD_USER};

/// Which unit file to make.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Unit {
    SystemdUser,
    SystemdSystem,
    LaunchAgent,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum RenderError {
    #[error(
        "the agent's path must be absolute and use only letters, digits and / . _ + - (systemd and launchd read other characters as syntax)"
    )]
    Path,
    #[error("the unit text no longer holds the binary's path exactly once: fix the template")]
    Template,
}

/// A path to the binary that is safe to write into a unit: absolute, and made only of
/// characters that systemd (`%`, `$`, quotes, spaces, backslash) and XML never treat as syntax.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Program<'a>(&'a str);

impl<'a> Program<'a> {
    /// # Errors
    /// [`RenderError::Path`] for a path that is relative, not UTF-8, empty of a name, has a
    /// `..` part, or holds a character outside `A-Za-z0-9/._+-` (a newline included).
    pub(super) fn parse(path: &'a Path) -> Result<Self, RenderError> {
        let text = path.to_str().ok_or(RenderError::Path)?;
        let plain = text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._+-".contains(&b));
        let clean = path.is_absolute()
            && plain
            && !text.ends_with('/')
            && text.split('/').all(|part| part != "..");
        if clean {
            Ok(Self(text))
        } else {
            Err(RenderError::Path)
        }
    }

    #[must_use]
    pub(super) const fn as_str(&self) -> &str {
        self.0
    }
}

/// What the template names the binary as, and the text around it that must stay.
struct Slot {
    template: &'static str,
    before: &'static str,
    placeholder: &'static str,
    after: &'static str,
}

const fn slot(unit: Unit) -> Slot {
    match unit {
        Unit::SystemdUser => Slot {
            template: SYSTEMD_USER,
            before: "ExecStart=",
            placeholder: "%h/.local/bin/anima-agent",
            after: " run\n",
        },
        Unit::SystemdSystem => Slot {
            template: SYSTEMD_SYSTEM,
            before: "ExecStart=",
            placeholder: "/usr/local/bin/anima-agent",
            after: " run\n",
        },
        Unit::LaunchAgent => Slot {
            template: LAUNCH_AGENT,
            before: "<string>",
            placeholder: "/usr/local/bin/anima-agent",
            after: "</string>",
        },
    }
}

/// The unit text with `program` in place of the template's path.
///
/// # Errors
/// [`RenderError::Template`] when the template does not hold the placeholder exactly once.
pub(super) fn render(unit: Unit, program: &Program<'_>) -> Result<String, RenderError> {
    fill(&slot(unit), program)
}

fn fill(slot: &Slot, program: &Program<'_>) -> Result<String, RenderError> {
    let marked = format!("{}{}{}", slot.before, slot.placeholder, slot.after);
    if slot.template.matches(&marked).count() != 1 {
        return Err(RenderError::Template);
    }
    let filled = format!("{}{}{}", slot.before, program.as_str(), slot.after);
    Ok(slot.template.replacen(&marked, &filled, 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(text: &str) -> Result<Program<'_>, RenderError> {
        Program::parse(Path::new(text))
    }

    #[test]
    fn met_019_each_unit_holds_the_path_once_where_the_binary_runs() {
        let path = program("/opt/anima/bin/anima-agent").expect("a path");
        for unit in [Unit::SystemdUser, Unit::SystemdSystem] {
            let text = render(unit, &path).expect("renders");
            assert_eq!(text.matches("/opt/anima/bin/anima-agent").count(), 1);
            assert!(text.contains("\nExecStart=/opt/anima/bin/anima-agent run\n"));
            assert!(!text.contains("/usr/local/bin") && !text.contains("%h/.local"));
        }
        let text = render(Unit::LaunchAgent, &path).expect("renders");
        let list = plist::Value::from_reader_xml(text.as_bytes()).expect("a property list");
        let arguments: Vec<&str> = list
            .as_dictionary()
            .and_then(|dict| dict.get("ProgramArguments"))
            .and_then(plist::Value::as_array)
            .expect("arguments")
            .iter()
            .filter_map(plist::Value::as_string)
            .collect();
        assert_eq!(arguments, ["/opt/anima/bin/anima-agent", "run"]);
    }

    #[test]
    fn met_019_the_rest_of_each_unit_is_left_as_it_was() {
        let path = program("/usr/local/bin/anima-agent").expect("a path");
        for (unit, template) in [
            (Unit::SystemdSystem, SYSTEMD_SYSTEM),
            (Unit::LaunchAgent, LAUNCH_AGENT),
        ] {
            assert_eq!(render(unit, &path).as_deref(), Ok(template));
        }
    }

    #[test]
    fn met_019_a_hostile_or_odd_path_is_refused() {
        for text in [
            "anima-agent",
            "./anima-agent",
            "",
            "/",
            "/opt/anima-agent/",
            "/opt/../bin/anima-agent",
            "/opt/a\nExecStartPre=/bin/sh/anima-agent",
            "/opt/a b/anima-agent",
            "/opt/%h/anima-agent",
            "/opt/$HOME/anima-agent",
            "/opt/\"x\"/anima-agent",
            "/opt/a\\b/anima-agent",
            "/opt/<x>/anima-agent",
            "/opt/a&b/anima-agent",
            "/opt/anima-agent (deleted)",
        ] {
            assert_eq!(program(text), Err(RenderError::Path), "{text:?}");
        }
        assert!(Program::parse(Path::new("/home/me/.local/bin/anima-agent")).is_ok());
    }

    #[test]
    fn met_019_a_template_without_the_path_once_is_an_error_not_a_silent_copy() {
        let path = program("/opt/anima-agent").expect("a path");
        for template in [
            "[Service]\nExecStart=/somewhere/else run\n",
            "ExecStart=/usr/local/bin/anima-agent run\nExecStart=/usr/local/bin/anima-agent run\n",
        ] {
            let slot = Slot {
                template,
                ..slot(Unit::SystemdSystem)
            };
            assert_eq!(fill(&slot, &path), Err(RenderError::Template));
        }
    }
}
