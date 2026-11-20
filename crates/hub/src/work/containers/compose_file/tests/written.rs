//! The writer: each text that it writes reads back as the same text, and as a text.

use super::super::{
    writer::{Value, write},
    yaml::{Node, Shape},
};
use super::{text, tree};

/// Texts that plain YAML reads as a different type, or as more than one node.
const HARD: [&str; 40] = [
    "",
    " ",
    " leading and trailing ",
    "yes",
    "no",
    "on",
    "off",
    "true",
    "False",
    "null",
    "~",
    "123",
    "0x1f",
    "1.5",
    "1e3",
    "012",
    "12:30:00",
    "2026-11-20",
    "a: b",
    "a #b",
    "#comment",
    "- item",
    "[a, b]",
    "{a: b}",
    "&anchor",
    "*alias",
    "!tag",
    "|",
    ">",
    "%directive",
    "@at",
    "`tick`",
    "it's \"quoted\"",
    "back\\slash \\n \\x41",
    "two\nlines\n",
    "tab\there",
    "carriage\rreturn",
    "$$ ${NAME} $NAME",
    "caf\u{e9} \u{65e5}\u{672c} \u{1f600}",
    "next\u{85}line \u{2028} \u{2029} \u{feff} \u{7f}",
];

/// What the reader must give for `value`: a truth value and a number are plain scalars.
fn read_back(value: &Value) -> Shape {
    let node = |shape| Node {
        place: super::super::Place { line: 0, column: 0 },
        shape,
    };
    match value {
        Value::Text(text) => Shape::Text(text.clone()),
        Value::Truth(truth) => Shape::Text(truth.to_string()),
        Value::Number(number) => Shape::Text(number.to_string()),
        Value::List(items) => Shape::List(items.iter().map(|item| node(read_back(item))).collect()),
        Value::Map(entries) => Shape::Map(
            entries
                .iter()
                .map(|(key, value)| (node(Shape::Text(key.clone())), node(read_back(value))))
                .collect(),
        ),
    }
}

/// Whether two trees are the same but for the positions.
fn same(read: &Node, expected: &Shape) -> bool {
    match (&read.shape, expected) {
        (Shape::List(read), Shape::List(expected)) => {
            read.len() == expected.len()
                && read.iter().zip(expected).all(|(a, b)| same(a, &b.shape))
        }
        (Shape::Map(read), Shape::Map(expected)) => {
            read.len() == expected.len()
                && read
                    .iter()
                    .zip(expected)
                    .all(|((ka, va), (kb, vb))| same(ka, &kb.shape) && same(va, &vb.shape))
        }
        (read, expected) => read == expected,
    }
}

#[test]
fn host_022_each_text_reads_back_as_the_same_text() {
    let texts = || {
        HARD.iter()
            .map(|text| Value::text(text))
            .collect::<Vec<_>>()
    };
    let mut inner = Value::map();
    for (index, text) in HARD.iter().enumerate() {
        // Each hard text also as a key, with the hard text as its value.
        inner.set(&format!("{text}{index}"), Value::text(text));
        inner.set(text, Value::text(text));
    }
    let mut file = Value::map();
    file.set("texts", Value::List(texts()));
    file.set("keys", inner);
    file.set(
        "nested",
        Value::List(vec![Value::List(texts()), Value::map()]),
    );
    file.child("scalars").set("truth", Value::Truth(false));
    file.child("scalars").set("number", Value::Number(-7));
    file.child("scalars")
        .set("empty_list", Value::List(Vec::new()));
    file.child("scalars").set("empty_map", Value::map());
    let written = write(&file);
    let read = tree(&written);
    let Shape::Map(mut expected) = read_back(&file) else {
        unreachable!()
    };
    // An empty list and an empty map read back as they are.
    let scalars = &mut expected[3].1.shape;
    if let Shape::Map(entries) = scalars {
        entries[2].1.shape = Shape::List(Vec::new());
        entries[3].1.shape = Shape::Map(Vec::new());
    }
    assert!(same(&read, &Shape::Map(expected)), "{written}");
    assert!(written.is_ascii() || written.contains('\u{e9}'));
    assert!(
        !written.chars().any(|c| c.is_control() && c != '\n'),
        "{written:?}"
    );
}

#[test]
fn host_022_a_fixed_key_is_plain_and_a_name_is_in_quotes() {
    let mut file = Value::map();
    file.child("services")
        .child("no")
        .set("image", Value::text("a"));
    file.child("services")
        .child("web")
        .set("read_only", Value::Truth(true));
    file.child("services")
        .child("web")
        .push("ports", Value::text("80:80"));
    let mut mount = Value::map();
    mount.set("type", Value::text("bind"));
    file.child("services").child("web").push("volumes", mount);
    let written = write(&file);
    assert_eq!(
        written,
        "services:\n  \"no\":\n    image: \"a\"\n  web:\n    read_only: true\n    ports:\n      \
         - \"80:80\"\n    volumes:\n      - type: \"bind\"\n"
    );
    assert_eq!(
        text(
            &tree(&written),
            &["services", "web", "volumes", "0", "type"]
        ),
        "bind"
    );
}

#[test]
fn host_022_each_character_that_yaml_does_not_print_is_an_escape() {
    // Not in the printable set of YAML 1.2, or a line break or a mark of the byte order.
    let odd = [
        '\u{7f}', '\u{85}', '\u{9f}', '\u{2028}', '\u{2029}', '\u{feff}', '\u{fffe}', '\u{ffff}',
    ];
    let text: String = odd.iter().flat_map(|c| [*c, 'x']).collect();
    let mut file = Value::map();
    file.set("odd", Value::text(&text));
    let written = write(&file);
    assert!(!written.chars().any(|c| odd.contains(&c)), "{written:?}");
    assert!(
        written.contains("\\ufffe") && written.contains("\\u0085"),
        "{written}"
    );
    assert_eq!(super::text(&tree(&written), &["odd"]), text);
    // A character that YAML prints stays as it is.
    let mut file = Value::map();
    file.set("kept", Value::text("é中😀"));
    assert!(write(&file).contains("é中😀"));
}
