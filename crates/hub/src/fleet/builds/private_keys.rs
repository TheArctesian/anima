//! Private keys in a builder's log (BLD-049, LOG-007): where a key block opens, which lines
//! belong to it, where it closes, and the updater's minisign key file written as one base64
//! word, the secret WP-36 most has to keep.
//!
//! Pure.

use base64::{Engine, engine::general_purpose::STANDARD};

/// The most lines a PEM, OpenSSH, PGP, SSH2 or `PuTTY` block may take after its opening line.
/// A 4096-bit RSA key in PEM is about 50; a PGP key with subkeys may be about 120.
const ARMOURED_LINES: u16 = 200;

/// The lines a minisign secret key takes after its `untrusted comment:` line: one base64 line.
const MINISIGN_LINES: u16 = 1;

/// The headers that may sit inside an armoured key block (PEM, PGP, SSH2, `PuTTY`).
const KEY_HEADERS: [&str; 14] = [
    "Proc-Type:",
    "DEK-Info:",
    "Comment:",
    "Version:",
    "Hash:",
    "Charset:",
    "Key-Derivation:",
    "Encryption:",
    "Public-Lines:",
    "Private-Lines:",
    "Argon2-Memory:",
    "Argon2-Passes:",
    "Argon2-Parallelism:",
    "Argon2-Salt:",
];

/// `untrusted comment:` in base64: how a minisign key file starts when written as one word
/// (the Tauri updater's `TAURI_SIGNING_PRIVATE_KEY`).
const MINISIGN_WORD: &str = "dW50cnVzdGVkIGNvbW1lbnQ6";

/// How a minisign secret key's base64 line starts, without its comment line: `Ed`, the KDF
/// (`Sc` for scrypt, two zero bytes for none) and `B2`, encoded (`EdScB2`, `Ed\0\0B2`). The
/// client's `redaction.ts` matches the same words. A public key (`Ed` and its key ID) starts with
/// `RWQ` too, but only a key ID that begins `00 00 42 32` would match.
const MINISIGN_SECRET_WORDS: [&str; 2] = ["RWRTY0Iy", "RWQAAEIy"];

/// The base64 characters read to find a minisign key file's comment line: 120 decode to 90
/// bytes, more than the comment line takes.
const MINISIGN_COMMENT_CHARS: usize = 120;

/// How many lines after `line` belong to the private key block it opens, or `None` when it
/// opens none: PEM, OpenSSH and PGP (`-----BEGIN` with `PRIVATE KEY` or `SECRET KEY`), SSH2
/// (`---- BEGIN SSH2` with `PRIVATE KEY`), a `PuTTY` key file's private lines
/// (`Private-Lines:`), and a minisign secret key's comment line.
pub(super) fn opens(line: &str) -> Option<u16> {
    let pem = (line.contains("-----BEGIN") || line.contains("---- BEGIN SSH2"))
        && (line.contains("PRIVATE KEY") || line.contains("SECRET KEY"));
    if pem || line.contains("Private-Lines:") {
        Some(ARMOURED_LINES)
    } else if line.contains("untrusted comment:") && line.contains("secret key") {
        Some(MINISIGN_LINES)
    } else {
        None
    }
}

/// Whether `line` closes a private key block: `-----END`, `---- END`, or a `PuTTY` key file's
/// `Private-MAC:`.
pub(super) fn closes(line: &str) -> bool {
    line.contains("-----END") || line.contains("---- END") || line.contains("Private-MAC:")
}

/// Whether `line` can be part of a key block: blank, an armour header, or base64 and armour
/// characters only. A compiler error or any other sentence is not, so it ends the block.
pub(super) fn is_material(line: &str) -> bool {
    let line = line.trim();
    line.is_empty()
        || KEY_HEADERS.iter().any(|header| line.starts_with(header))
        || line
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'=' | b'-' | b'_'))
}

/// Whether `text` is secret as a whole: it opens a private key block (a one-line key holds
/// its body on that line), holds a minisign secret key file as one word, or holds a minisign
/// secret key's base64 line on its own.
pub(super) fn is_secret_text(text: &str) -> bool {
    opens(text).is_some() || holds_minisign_secret(text) || holds_minisign_secret_line(text)
}

/// In a minisign public key's comment line (`untrusted comment: minisign public key: 1E1D…`),
/// the byte of the colon before the key id, which the event log's rule would read as the
/// separator of a secret `key: value`. The key id is public.
pub(super) fn public_key_id_colon(text: &str) -> Option<usize> {
    if !text.contains("untrusted comment:") {
        return None;
    }
    let colon = text.find("public key:")? + "public key".len();
    let id = text[colon + 1..].trim_start_matches(' ');
    let hex = id.bytes().take_while(u8::is_ascii_hexdigit).count();
    let ended = id[hex..].chars().next().is_none_or(char::is_whitespace);
    (hex > 0 && ended).then_some(colon)
}

/// A byte of standard base64.
fn is_base64(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=')
}

/// Where the base64 words of `text` that start with `prefix` start.
fn word_starts<'a>(text: &'a str, prefix: &'a str) -> impl Iterator<Item = usize> + 'a {
    text.match_indices(prefix)
        .map(|(start, _)| start)
        .filter(|&start| start == 0 || !is_base64(text.as_bytes()[start - 1]))
}

/// Whether `text` holds a word that starts like a minisign secret key's base64 line.
fn holds_minisign_secret_line(text: &str) -> bool {
    MINISIGN_SECRET_WORDS
        .iter()
        .any(|word| word_starts(text, word).next().is_some())
}

/// Whether `text` holds a minisign key file written as one base64 word whose comment line does
/// not name a public key. A word that does not decode counts as secret.
fn holds_minisign_secret(text: &str) -> bool {
    let bytes = text.as_bytes();
    word_starts(text, MINISIGN_WORD).any(|start| {
        let len = bytes[start..].iter().take_while(|b| is_base64(**b)).count();
        let read = if len > MINISIGN_COMMENT_CHARS {
            MINISIGN_COMMENT_CHARS
        } else {
            len
        };
        let word = &text[start..start + read - read % 4];
        STANDARD.decode(word).map_or(true, |decoded| {
            let comment = String::from_utf8_lossy(&decoded);
            !comment.lines().next().unwrap_or("").contains("public key")
        })
    })
}
