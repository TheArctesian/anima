//! Failure messages lose control and invisible characters and stop at 512 characters (LOG-007).

#[test]
fn messages_lose_control_characters_and_stop_at_512_characters() {
    use crate::checks::probe::{MESSAGE_MAX, clean_message};
    assert_eq!(clean_message("a\r\nb\0c\u{1b}[31md\te"), "abc[31mde");
    let long = "x".repeat(MESSAGE_MAX + 100);
    assert_eq!(clean_message(&long).chars().count(), 512);
    assert_eq!(clean_message(&"y".repeat(512)).chars().count(), 512);
    assert_eq!(clean_message("short"), "short");
}

#[test]
fn log_007_messages_lose_bidi_zero_width_and_separator_characters() {
    use crate::checks::probe::clean_message;
    let text = "ok\u{202E}gnp.exe\u{2028}line\u{2029}para\u{200B}zw\u{FEFF}bom\u{061C}alm";
    assert_eq!(clean_message(text), "okgnp.exelineparazwbomalm");
    for c in [
        '\u{200B}',
        '\u{200F}',
        '\u{202A}',
        '\u{2060}',
        '\u{2069}',
        '\u{FEFF}',
        '\u{061C}',
        '\u{0600}',
        '\u{0890}',
        '\u{1D173}',
    ] {
        assert_eq!(clean_message(&format!("a{c}b")), "ab", "{c:?}");
    }
    assert_eq!(
        clean_message("plain \u{e4}\u{4e2d}"),
        "plain \u{e4}\u{4e2d}"
    );
}
