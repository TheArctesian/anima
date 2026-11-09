//! Step output split into lines (BLD-048): never trusting a line's length.

use crate::builder::lines::{Lines, MAX_LINE_BYTES};

/// The lines of `input` when it arrives in the given chunks.
fn split(input: &[u8], cuts: &[usize]) -> Vec<String> {
    let mut lines = Lines::default();
    let mut out = Vec::new();
    let mut from = 0;
    for &cut in cuts.iter().chain([&input.len()]) {
        out.extend(lines.push(&input[from..cut]));
        from = cut;
    }
    out.extend(lines.finish());
    out
}

#[test]
fn bld_048_lines_are_the_same_however_the_output_is_chunked() {
    let long = "x".repeat(MAX_LINE_BYTES + 10);
    let input = format!("one\r\ntwo\n\n{long}\nthree é\nlast").into_bytes();
    let whole = split(&input, &[]);
    assert_eq!(whole.len(), 6);
    assert_eq!(whole[..3], ["one", "two", ""]);
    assert_eq!(whole[3].len(), MAX_LINE_BYTES + " [line cut]".len());
    assert_eq!(whole[4..], ["three é", "last"]);
    for a in (0..=input.len()).step_by(13) {
        for b in (a..=input.len()).step_by(97) {
            assert_eq!(split(&input, &[a, b]), whole, "cuts at {a} and {b}");
        }
    }
}

#[test]
fn bld_048_an_endless_line_is_cut_and_never_held_whole() {
    let mut lines = Lines::default();
    let chunk = vec![b'y'; 1 << 16];
    let mut emitted = 0;
    for _ in 0..64 {
        emitted += lines.push(&chunk).len();
    }
    assert_eq!(emitted, 1);
    assert_eq!(lines.finish(), None);
    assert_eq!(lines.push(b"after\n"), ["after"]);
}
