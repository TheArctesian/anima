// The parser of `trust-cases.txt`, pulled in with `include!` by the agent's test module
// (crates/agent/src/units/trust_cases.rs) and the desktop shell's
// (apps/desktop/src-tauri/src/agent/tests/trust.rs), so there is one copy. It is a file of
// items, not a module, and has no `use`: the including file brings in nothing it needs.

const TABLE: &str = include_str!("../trust-cases.txt");

// One row of the table of trust cases, with its account files resolved. `passwd` and
// `group_file` are `None` for a file the row says cannot be read.
#[expect(
    clippy::struct_excessive_bools,
    reason = "a row of the table: each flag is one of its columns"
)]
struct Case {
    name: String,
    mode: u32,
    owner: u32,
    group: u32,
    acl: bool,
    uid: u32,
    passwd: Option<String>,
    group_file: Option<String>,
    private: Option<u32>,
    agent: bool,
    shell: bool,
    differs: bool,
}

// One of exactly two words; any other value panics, so a typo cannot read as a verdict.
fn word(value: &str, yes: &str, no: &str, row: &str) -> bool {
    if value == yes {
        true
    } else if value == no {
        false
    } else {
        panic!("row {row}: `{value}` is neither `{yes}` nor `{no}`")
    }
}

// The rows of `trust-cases.txt`; a malformed row or a duplicate set panics, so the table
// cannot rot quietly.
fn cases() -> Vec<Case> {
    let mut sets = std::collections::HashMap::new();
    let mut rows = Vec::new();
    for line in TABLE
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let set = ["passwd", "group"]
            .into_iter()
            .find_map(|kind| Some((kind, line.strip_prefix(kind)?.strip_prefix(' ')?)));
        if let Some((kind, rest)) = set {
            let (name, text) = rest.split_once(' ').expect("set name and text");
            let text = (text != "none").then(|| text.replace(';', "\n") + "\n");
            let again = sets.insert((kind, name.to_owned()), text);
            assert!(again.is_none(), "duplicate {kind} set {name}");
        } else {
            rows.push(line);
        }
    }
    let set = |kind: &str, name: &str| {
        sets.get(&(kind, name.to_owned()))
            .unwrap_or_else(|| panic!("no {kind} set {name}"))
            .clone()
    };
    rows.into_iter()
        .map(|row| {
            let f: Vec<&str> = row.split('|').map(str::trim).collect();
            assert_eq!(f.len(), 12, "row {row}");
            Case {
                name: f[0].to_owned(),
                mode: u32::from_str_radix(f[1], 8).expect("octal"),
                owner: f[2].parse().expect("owner"),
                group: f[3].parse().expect("group"),
                acl: word(f[4], "1", "0", row),
                uid: f[5].parse().expect("uid"),
                passwd: set("passwd", f[6]),
                group_file: set("group", f[7]),
                private: (f[8] != "-").then(|| f[8].parse().expect("private")),
                agent: word(f[9], "trust", "refuse", row),
                shell: word(f[10], "trust", "refuse", row),
                differs: word(f[11], "DIFF", "-", row),
            }
        })
        .collect()
}
