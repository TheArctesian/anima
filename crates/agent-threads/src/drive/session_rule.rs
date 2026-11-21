//! The rule that "Allow for this thread" sends to Claude Code (THR-030). Claude Code suggests
//! allow rules for a settings file; the runner offers the same rules for the session only, built
//! again from their tool names and contents, so that no other field goes to the program.

use serde_json::{Value, json};

/// The longest rule, in words, that is offered: a rule is offered whole or not at all, so that
/// the person never allows a part that the card does not show.
const MAX_WORDS: usize = 1024;

/// The tools that change files: a rule for one of them must name the files, and the files must
/// not be the settings of the program.
const FILE_TOOLS: [&str; 4] = ["Edit", "Write", "MultiEdit", "NotebookEdit"];

/// A rule that the runner may offer for the session, and its words (`Bash(npm test)`).
#[must_use]
pub(super) fn offered(suggestion: &Value) -> Option<(Value, String)> {
    if suggestion["type"] != "addRules" || suggestion["behavior"] != "allow" {
        return None;
    }
    let rules: Vec<(&str, &str)> = suggestion["rules"]
        .as_array()?
        .iter()
        .map(|rule| Some((rule["toolName"].as_str()?, rule["ruleContent"].as_str()?)))
        .collect::<Option<_>>()?;
    if rules.is_empty() || !rules.iter().all(|&(tool, content)| safe(tool, content)) {
        return None;
    }
    let words = super::request::visible(
        &rules
            .iter()
            .map(|(tool, content)| format!("{tool}({content})"))
            .collect::<Vec<_>>()
            .join(", "),
    );
    if words.len() > MAX_WORDS {
        return None;
    }
    let sent: Vec<Value> = rules
        .iter()
        .map(|(tool, content)| json!({"toolName": tool, "ruleContent": content}))
        .collect();
    let rule =
        json!({"type": "addRules", "behavior": "allow", "destination": "session", "rules": sent});
    Some((rule, words))
}

/// Whether one rule allows less than every use of its tool, and a file rule keeps away from the
/// settings of the program.
fn safe(tool: &str, content: &str) -> bool {
    let content = content.trim();
    if content.is_empty() || content == "*" {
        return false;
    }
    if !FILE_TOOLS.contains(&tool) {
        return true;
    }
    let everywhere = content.starts_with('*')
        || content.starts_with("/**")
        || content.starts_with("./**")
        || content.starts_with('~');
    !everywhere && !content.contains(".claude") && !content.contains(".mcp.json")
}
