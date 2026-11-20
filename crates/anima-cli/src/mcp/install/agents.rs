//! The coding agents that `anima mcp install` knows, and the way to set each up (plan Section
//! 12.2). Each row is from the documentation of the product; nothing here touches a file.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};

/// The name of the entry in each agent: the one member this program ever changes (MCP-030).
pub const NAME: &str = "anima";

/// How the entry of one agent is written in its file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// `{"command": ..., "args": ["mcp"]}`.
    Plain,
    /// The same with `"type": "stdio"`.
    Stdio,
    /// opencode: `{"type": "local", "command": [..., "mcp"]}`.
    Local,
    /// Zed: with an empty `env`.
    WithEnv,
}

/// How an agent tells whether it holds a server, through its own program (MCP-035).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    /// The arguments end without failure exactly when the server is there.
    Get(&'static [&'static str]),
    /// The arguments list the servers; the server is there when the list names the path.
    List(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug)]
pub enum Way {
    /// A file below the home folder and the keys of its servers object (MCP-030).
    File {
        place: Place,
        keys: &'static [&'static str],
        shape: Shape,
    },
    /// The agent's own program; `{anima}` in an argument is the path of MCP-033 (MCP-035).
    Own {
        program: &'static str,
        add: &'static [&'static str],
        remove: &'static [&'static str],
        check: Check,
    },
}

/// Where the file of an agent is, below the home folder.
#[derive(Clone, Copy, Debug)]
pub enum Place {
    Fixed(&'static str),
    /// A different folder on macOS and on Linux.
    BySystem {
        macos: &'static str,
        linux: &'static str,
    },
    /// The first when it exists, else the second.
    FirstThatExists(&'static str, &'static str),
}

#[derive(Clone, Copy, Debug)]
pub struct Agent {
    pub id: &'static str,
    pub name: &'static str,
    /// The agent is on this computer when one of these is on `PATH`...
    pub programs: &'static [&'static str],
    /// ...or one of these folders exists below the home folder (MCP-027).
    pub folders: &'static [&'static str],
    pub way: Way,
}

const SERVERS: &[&str] = &["mcpServers"];

pub const AGENTS: &[Agent] = &[
    Agent {
        id: "claude-code",
        name: "Claude Code",
        programs: &["claude"],
        folders: &[],
        way: Way::Own {
            program: "claude",
            add: &[
                "mcp",
                "add",
                "--transport",
                "stdio",
                "--scope",
                "user",
                NAME,
                "--",
                "{anima}",
                "mcp",
            ],
            remove: &["mcp", "remove", NAME],
            check: Check::Get(&["mcp", "get", NAME]),
        },
    },
    Agent {
        id: "claude-desktop",
        name: "Claude Desktop",
        programs: &[],
        folders: &["Library/Application Support/Claude", ".config/Claude"],
        way: Way::File {
            place: Place::BySystem {
                macos: "Library/Application Support/Claude/claude_desktop_config.json",
                linux: ".config/Claude/claude_desktop_config.json",
            },
            keys: SERVERS,
            shape: Shape::Plain,
        },
    },
    Agent {
        id: "codex",
        name: "Codex CLI",
        programs: &["codex"],
        folders: &[],
        way: Way::Own {
            program: "codex",
            add: &["mcp", "add", NAME, "--", "{anima}", "mcp"],
            remove: &["mcp", "remove", NAME],
            check: Check::Get(&["mcp", "get", NAME, "--json"]),
        },
    },
    Agent {
        id: "cursor",
        name: "Cursor",
        programs: &[],
        folders: &[".cursor"],
        way: Way::File {
            place: Place::Fixed(".cursor/mcp.json"),
            keys: SERVERS,
            shape: Shape::Stdio,
        },
    },
    Agent {
        id: "vscode",
        name: "VS Code",
        programs: &["code"],
        folders: &[".copilot"],
        way: Way::File {
            place: Place::Fixed(".copilot/mcp-config.json"),
            keys: SERVERS,
            shape: Shape::Stdio,
        },
    },
    Agent {
        id: "gemini",
        name: "Gemini CLI",
        programs: &["gemini"],
        folders: &[],
        way: Way::Own {
            program: "gemini",
            add: &["mcp", "add", "-s", "user", NAME, "{anima}", "mcp"],
            remove: &["mcp", "remove", "-s", "user", NAME],
            check: Check::List(&["mcp", "list"]),
        },
    },
    Agent {
        id: "opencode",
        name: "opencode",
        programs: &["opencode"],
        folders: &[".config/opencode"],
        way: Way::File {
            place: Place::FirstThatExists(
                ".config/opencode/opencode.jsonc",
                ".config/opencode/opencode.json",
            ),
            keys: &["mcp", "servers"],
            shape: Shape::Local,
        },
    },
    Agent {
        id: "zed",
        name: "Zed",
        programs: &[],
        folders: &[".config/zed"],
        way: Way::File {
            place: Place::Fixed(".config/zed/settings.json"),
            keys: &["context_servers"],
            shape: Shape::WithEnv,
        },
    },
];

/// The agent with this id.
#[must_use]
pub fn agent(id: &str) -> Option<&'static Agent> {
    AGENTS.iter().find(|agent| agent.id == id)
}

/// What an agent's entry is to this program (MCP-027).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    NotInstalled,
    NotSetUp,
    SetUp,
    Different,
    Unreadable,
}

impl State {
    /// The state in words for a person.
    #[must_use]
    pub const fn words(self) -> &'static str {
        match self {
            Self::NotInstalled => "not on this computer",
            Self::NotSetUp => "not set up",
            Self::SetUp => "set up",
            Self::Different => "set up, starts a different anima",
            Self::Unreadable => "its file cannot be read",
        }
    }
}

/// The entry of `shape` that starts `anima` with the one argument `mcp` (MCP-033): no
/// environment, no token, no hub address.
#[must_use]
pub fn entry(shape: Shape, anima: &Path) -> Value {
    let anima = anima.to_string_lossy();
    match shape {
        Shape::Plain => json!({ "command": anima, "args": ["mcp"] }),
        Shape::Stdio => json!({ "type": "stdio", "command": anima, "args": ["mcp"] }),
        Shape::Local => json!({ "type": "local", "command": [anima, "mcp"] }),
        Shape::WithEnv => json!({ "command": anima, "args": ["mcp"], "env": {} }),
    }
}

/// The file of `place` below `home`; `exists` answers for [`Place::FirstThatExists`].
#[must_use]
pub fn file(place: Place, home: &Path, macos: bool, exists: impl Fn(&Path) -> bool) -> PathBuf {
    match place {
        Place::Fixed(path) => home.join(path),
        Place::BySystem { macos: path, .. } if macos => home.join(path),
        Place::BySystem { linux, .. } => home.join(linux),
        Place::FirstThatExists(first, second) => {
            let first = home.join(first);
            if exists(&first) {
                first
            } else {
                home.join(second)
            }
        }
    }
}

/// The arguments of an own command with the path of `anima` put in.
#[must_use]
pub fn arguments(template: &[&str], anima: &Path) -> Vec<String> {
    template
        .iter()
        .map(|part| {
            if *part == "{anima}" {
                anima.to_string_lossy().into_owned()
            } else {
                (*part).to_owned()
            }
        })
        .collect()
}
