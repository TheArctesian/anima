//! Text from GitHub, cut to the lengths K1 states. Other people write it: it is passed on as a
//! plain string, never parsed, rendered or logged. Pure.

/// The most characters of a title.
pub const TITLE: usize = 300;
/// The most characters of a body or a comment (Markdown source).
pub const BODY: usize = 20_000;
/// The most labels of an issue or a pull request.
pub const LABELS: usize = 20;

/// The first `most` characters of `text`, cut on a character boundary, and whether it was cut.
#[must_use]
pub fn cut(text: String, most: usize) -> (String, bool) {
    match text.char_indices().nth(most) {
        Some((end, _)) => (text[..end].to_owned(), true),
        None => (text, false),
    }
}

/// [`cut`] without the mark, for fields K1 caps without one.
#[must_use]
pub fn capped(text: String, most: usize) -> String {
    cut(text, most).0
}

/// A body cut to [`BODY`], and whether it was cut; no body is not cut.
#[must_use]
pub fn body(text: Option<String>) -> (Option<String>, bool) {
    match text.map(|text| cut(text, BODY)) {
        Some((body, was_cut)) => (Some(body), was_cut),
        None => (None, false),
    }
}

/// The first `most` of `names`.
#[must_use]
pub fn first(mut names: Vec<String>, most: usize) -> Vec<String> {
    names.truncate(most);
    names
}
