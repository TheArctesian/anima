//! The words of a shell command line: quotes, the backslash, the line continuation, and what
//! stays of a `$`.

use super::super::words::{Split, split};

fn words(command: &str) -> Vec<String> {
    let split = split(command).unwrap_or_else(|refusal| panic!("{command}: {refusal:?}"));
    split.words.into_iter().map(|word| word.text).collect()
}

#[test]
fn host_022_quotes_and_backslashes_are_read_as_a_shell_reads_them() {
    for (command, expected) in [
        ("a  b\tc", vec!["a", "b", "c"]),
        ("  a b  \n", vec!["a", "b"]),
        ("'a b' \"c d\" e\\ f", vec!["a b", "c d", "e f"]),
        ("a'b'\"c\"d", vec!["abcd"]),
        ("'' \"\" x", vec!["", "", "x"]),
        ("'it'\\''s'", vec!["it's"]),
        ("\"a \\\"b\\\" \\\\ \\n\"", vec!["a \"b\" \\ \\n"]),
        ("'a \\ b'", vec!["a \\ b"]),
        ("a \\\n  b\\\nc", vec!["a", "bc"]),
        ("a \\\r\n b", vec!["a", "b"]),
        ("\"a\\\nb\"", vec!["ab"]),
        ("'two\nlines'", vec!["two\nlines"]),
        (
            // A shell expands `~`, `*` and `{a,b}` out of quotes, and keeps each in quotes.
            "-e COLOR=#fff '~/x' \"a*b\" \\{a,b} a=b",
            vec!["-e", "COLOR=#fff", "~/x", "a*b", "{a,b}", "a=b"],
        ),
        (
            "'|' \";\" '&&' \">\" '`' '$(x)' '#'",
            vec!["|", ";", "&&", ">", "`", "$$(x)", "#"],
        ),
        ("a\\|b a\\;b", vec!["a|b", "a;b"]),
    ] {
        assert_eq!(words(command), expected, "{command}");
    }
}

#[test]
fn host_022_a_variable_stays_for_compose_and_a_dollar_character_is_doubled() {
    for (command, expected, variables, dollars) in [
        ("$HOME/x", "$HOME/x", true, false),
        ("\"${DATA_DIR}/x\"", "${DATA_DIR}/x", true, false),
        ("${TAG:-latest}", "${TAG:-latest}", true, false),
        ("${TAG-latest}", "${TAG-latest}", true, false),
        ("${KEY:?it is needed}", "${KEY:?it is needed}", true, false),
        ("a$B_2.c", "a$B_2.c", true, false),
        ("'$HOME'", "$$HOME", false, true),
        ("\\$HOME", "$$HOME", false, true),
        ("\"\\$HOME\"", "$$HOME", false, true),
        ("\"costs 5$\"", "costs 5$$", false, true),
        ("5$ a", "5$$", false, true),
        ("'a$$b'", "a$$$$b", false, true),
    ] {
        let Split {
            words,
            variables: has_variables,
            dollars: has_dollars,
        } = split(command).unwrap_or_else(|refusal| panic!("{command}: {refusal:?}"));
        assert_eq!(words[0].text, expected, "{command}");
        assert_eq!(
            (has_variables, has_dollars),
            (variables, dollars),
            "{command}"
        );
    }
}

#[test]
fn host_022_each_word_has_the_position_of_its_first_character() {
    let split = split("docker run 'a b' c").expect("words");
    let places: Vec<u32> = split.words.iter().map(|word| word.at).collect();
    assert_eq!(places, [1, 8, 12, 18]);
}
