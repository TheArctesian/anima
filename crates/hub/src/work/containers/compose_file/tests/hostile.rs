//! Text that is made to break the check: it must stop at a limit and never fail.

use std::fmt::Write as _;

use super::super::{LARGEST_FILE, ProblemCode, check, syntax::MOST_PROBLEMS, yaml};
use super::codes;

#[test]
fn host_022_a_file_of_more_than_256_kib_is_not_read() {
    let line = "# a line of a comment that is fifty bytes long.....\n";
    let large = format!("services:\n  web:\n    image: a\n{}", line.repeat(6_000));
    assert!(large.len() > 300 * 1024);
    assert_eq!(codes(&large), [ProblemCode::TooLarge]);
    let fits = &large[..LARGEST_FILE - 1];
    assert_eq!(codes(fits), []);
}

#[test]
fn host_022_deep_nesting_stops_at_the_limit() {
    let lists = |depth: usize| format!("{}{}", "[".repeat(depth), "]".repeat(depth));
    assert_eq!(codes(&lists(70)), [ProblemCode::TooDeep]);
    // The scanner of the parser reads ahead at most 255 open lists, and refuses more.
    let open = codes(&"[".repeat(100_000));
    assert!(matches!(
        open[..],
        [ProblemCode::TooDeep | ProblemCode::NotYaml]
    ));
    let maps = |depth: usize| format!("{}x{}", "{a: ".repeat(depth), "}".repeat(depth));
    assert_eq!(codes(&maps(70)), [ProblemCode::TooDeep]);
    // The parser reads ahead in a flow mapping, and refuses one that is very deep itself.
    let deepest = codes(&maps(50_000));
    assert!(
        matches!(deepest[..], [ProblemCode::TooDeep | ProblemCode::NotYaml]),
        "{deepest:?}"
    );
    let mut block = String::new();
    for depth in 0..200 {
        block.push_str(&" ".repeat(depth));
        block.push_str("a:\n");
    }
    assert_eq!(codes(&block), [ProblemCode::TooDeep]);
    // A file at the limit is read.
    let at_limit = format!(
        "x-deep: {}{}\nservices: {{web: {{image: a}}}}\n",
        "[".repeat(yaml::DEEPEST - 1),
        "]".repeat(yaml::DEEPEST - 1)
    );
    assert_eq!(codes(&at_limit), []);
}

#[test]
fn host_022_a_chain_of_aliases_is_not_expanded() {
    // Each anchor names the one before it ten times: expanded, the last is 10^30 nodes.
    let mut file = String::from("x-0: &a0 [x, x, x, x, x, x, x, x, x, x]\n");
    for step in 1..=30 {
        let before = step - 1;
        let aliases = vec![format!("*a{before}"); 10].join(", ");
        let _ = writeln!(file, "x-{step}: &a{step} [{aliases}]");
    }
    file.push_str("services:\n  web:\n    image: a\n    command: *a30\n");
    assert_eq!(codes(&file), []);
}

#[test]
fn host_022_a_file_with_too_many_nodes_stops_at_the_limit() {
    let file = format!("x-list: [{}]\nservices: {{}}\n", "a,".repeat(110_000));
    assert!(file.len() < LARGEST_FILE);
    assert_eq!(codes(&file), [ProblemCode::TooManyNodes]);
}

#[test]
fn host_022_the_problems_of_one_file_stop_at_100() {
    let mut file = String::from("services:\n  web:\n    image: a\n");
    for key in 0..500 {
        let _ = writeln!(file, "    unknown{key}: 1");
    }
    let found = check(&file);
    assert_eq!(found.problems.len(), MOST_PROBLEMS);
    assert!(found.cut && found.syntax_valid());
    // Many keys with one name: the check does not compare each key with each other key.
    let same = format!(
        "services:\n  web:\n    image: a\n{}",
        "    a: 1\n".repeat(25_000)
    );
    assert!(check(&same).cut);
}

#[test]
fn host_022_control_characters_and_broken_text_never_stop_the_check() {
    let texts = [
        "services:\n  web:\n    image: \u{0}\u{7}\u{1b}[31m\n",
        "\u{0}",
        "\u{feff}services:\n  web:\n    image: a\n",
        "services:\r\n  web:\r\n    image: a\r\n",
        "\tservices: x",
        "%YAML 9.9\n---\na: b",
        "? [a, b]\n: c\n",
        "*unknown",
        "&a *a",
        "a: !!binary |\n  AAAA\n",
        "- - - - -",
        "{",
        "}",
        "'",
        "\"\\x",
        "a: |\n",
        "a: >-\n\n\n",
        "--- ---",
        "...",
        "? ? ? ?",
        "a: b: c: d",
        "!!map {a: b}",
        "\u{2028}\u{2029}",
    ];
    for text in texts {
        for problem in check(text).problems {
            let shown = problem.key.unwrap_or_default();
            assert!(shown.chars().all(|c| c.is_ascii_graphic()), "{text:?}");
        }
    }
    // Each start of a good file, cut at each byte: a file that someone is writing.
    let good = "services:\n  \"web\":\n    image: 'a'\n    ports: [\"80:80\", {target: 1}]\n";
    for end in 0..good.len() {
        let _ = check(&good[..end]);
    }
}
