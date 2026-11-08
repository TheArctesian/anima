//! The CSP hashes of the inline scripts in `index.html` (A-2): `SvelteKit` starts the app from
//! one inline `<script>`, and `script-src` allows exactly that text and no other.
//!
//! The page is the hub's own build output, not operator input. A misread can only drop a
//! hash, which blocks a script, or add the hash of text the page holds; it never widens
//! `script-src` to any other source.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use sha2::{Digest as _, Sha256};

const OPEN: &str = "<script";
const CLOSE: &str = "</script";

/// The CSP source `'sha256-<base64>'` of each inline script in `html`, in page order. A
/// script with a `src` attribute is not inline and gets none; an unclosed script ends the
/// search.
#[must_use]
pub fn inline_script_hashes(html: &str) -> Vec<String> {
    // ASCII lowercasing keeps every byte offset, so offsets in `lower` slice `html`.
    let lower = html.to_ascii_lowercase();
    let mut hashes = Vec::new();
    let mut from = 0;
    while let Some(found) = lower[from..].find(OPEN) {
        let tag = from + found + OPEN.len();
        let Some(&next) = lower.as_bytes().get(tag) else {
            break;
        };
        if !(next.is_ascii_whitespace() || next == b'>' || next == b'/') {
            from = tag; // `<scripts`, `<script-x`: another element.
            continue;
        }
        let Some(tag_len) = tag_end(&lower[tag..]) else {
            break;
        };
        let body = tag + tag_len + 1;
        let Some(body_len) = lower[body..].find(CLOSE) else {
            break;
        };
        if !attribute_names(&lower[tag..tag + tag_len]).contains(&"src") {
            hashes.push(source_hash(&html[body..body + body_len]));
        }
        from = body + body_len + CLOSE.len();
    }
    hashes
}

/// `'sha256-<base64>'` of `script`'s UTF-8 bytes, as a browser hashes an inline script.
fn source_hash(script: &str) -> String {
    format!(
        "'sha256-{}'",
        STANDARD.encode(Sha256::digest(script.as_bytes()))
    )
}

/// The offset of the `>` that ends a start tag whose attributes begin `tag`; a `>` inside a
/// quoted value does not end it.
fn tag_end(tag: &str) -> Option<usize> {
    let mut quote = None;
    for (offset, byte) in tag.bytes().enumerate() {
        match (quote, byte) {
            (Some(open), _) if byte == open => quote = None,
            (None, b'"' | b'\'') => quote = Some(byte),
            (None, b'>') => return Some(offset),
            _ => {}
        }
    }
    None
}

/// The names of the attributes in a start tag's attribute text, values skipped.
fn attribute_names(attributes: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut rest = attributes;
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace() || c == '/');
        if rest.is_empty() {
            return names;
        }
        let end = rest
            .find(|c: char| c.is_ascii_whitespace() || c == '=' || c == '/')
            .unwrap_or(rest.len());
        names.push(&rest[..end]);
        rest = rest[end..].trim_start_matches(|c: char| c.is_ascii_whitespace());
        if let Some(value) = rest.strip_prefix('=') {
            rest = after_value(value.trim_start_matches(|c: char| c.is_ascii_whitespace()));
        }
    }
}

/// The text after one attribute value at the start of `value`: quoted, or up to whitespace.
fn after_value(value: &str) -> &str {
    match value.chars().next() {
        Some(quote @ ('"' | '\'')) => value[1..]
            .find(quote)
            .map_or("", |close| &value[close + 2..]),
        _ => value
            .find(|c: char| c.is_ascii_whitespace())
            .map_or("", |end| &value[end..]),
    }
}
