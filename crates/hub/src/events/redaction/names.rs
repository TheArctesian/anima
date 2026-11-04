//! Whether a JSON key (or label value) names a secret (LOG-007, D-3).

use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

/// A normalised name that contains one of these names a secret.
const SECRET_PARTS: [&str; 19] = [
    "password",
    "passwd",
    "passphrase",
    "passcode",
    "secret",
    "token",
    "jwt",
    "authorization",
    "cookie",
    "bearer",
    "privatekey",
    "privkey",
    "authkey",
    "keymaterial",
    "totp",
    "otpauth",
    "recoverycode",
    "backupcode",
    "mfacode",
];

/// A name with one of these as a whole word names a secret (`otpCode`, `sshKeyPem`, `sshPass`),
/// while `notPaused`, `footprint` and `hotplug` do not.
const SECRET_WORDS: [&str; 7] = ["otp", "pem", "auth", "pin", "pass", "pwd", "pw"];

/// With one of these in the name, `pin` means certificate pinning, not a PIN (`certPin`).
const PINNING: [&str; 3] = ["cert", "tls", "spki"];

/// Names written as one lower-case word that end in "key". A name whose last word is `key`
/// (`apiKey`, `ssh_key`, `APIKeys`) is a secret too; `monkey`, `keyboard` and `keyId` are not.
const KEY_COMPOUNDS: [&str; 5] = [
    "apikey",
    "sshkey",
    "accesskey",
    "signingkey",
    "encryptionkey",
];

/// Names ending in these are public keys, never secret, and kept as audit evidence (SEC-006,
/// WEB-009): `hostKey`, `newHostKey`, `publicKey`, `authorizedKeys`.
const PUBLIC_KEYS: [&str; 4] = ["hostkey", "publickey", "nodekey", "authorizedkey"];

/// What a name says about its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Secrecy {
    /// Not a secret.
    Plain,
    /// Holds credentials (`credential`, `sshCredential`): a scalar value is a secret, but an
    /// object is searched like any other, so its `kind` and `username` stay.
    Holder,
    /// The value is a secret.
    Secret,
}

pub(super) fn secrecy(name: &str) -> Secrecy {
    let words = words(name);
    let joined = words.concat();
    let has_word = |wanted: &str| words.iter().any(|word| word == wanted);
    let pinning = PINNING.iter().any(|part| joined.contains(part));
    if PUBLIC_KEYS.iter().any(|public| joined.ends_with(public)) {
        Secrecy::Plain
    } else if SECRET_PARTS.iter().any(|part| joined.contains(part))
        || SECRET_WORDS
            .iter()
            .any(|word| has_word(word) && !(*word == "pin" && pinning))
        || words.last().is_some_and(|word| word == "key")
        || KEY_COMPOUNDS
            .iter()
            .any(|compound| joined.ends_with(compound))
    {
        Secrecy::Secret
    } else if joined.contains("credential") && words.last().is_none_or(|word| word != "id") {
        Secrecy::Holder
    } else {
        Secrecy::Plain
    }
}

/// The name in lower-case ASCII letters and digits only.
pub(super) fn normalise(name: &str) -> String {
    words(name).concat()
}

/// Splits a name into lower-case ASCII words. NFKD with combining marks dropped turns `pässword`
/// into `password` and full-width letters into ASCII. Any other character (separators,
/// zero-width spaces, other scripts) ends a word, and so does a case change: `apiKey`, `api_key`
/// and `APIKey` all give `["api", "key"]`. A plural `s` on the last word is dropped (`apiKeys`),
/// but not a double `ss` (`pass`).
fn words(name: &str) -> Vec<String> {
    let chars: Vec<char> = name.nfkd().filter(|c| !is_combining_mark(*c)).collect();
    let mut words = Vec::new();
    let mut word = String::new();
    let mut after_lower = false;
    for (index, &c) in chars.iter().enumerate() {
        let before_lower = chars.get(index + 1).is_some_and(|next| next.is_lowercase());
        if c.is_uppercase() && (after_lower || before_lower) && !word.is_empty() {
            words.push(std::mem::take(&mut word));
        }
        let folded: String = c
            .to_lowercase()
            .filter(char::is_ascii_alphanumeric)
            .collect();
        if folded.is_empty() && !word.is_empty() {
            words.push(std::mem::take(&mut word));
        }
        word.push_str(&folded);
        after_lower = !folded.is_empty() && !c.is_uppercase();
    }
    if !word.is_empty() {
        words.push(word);
    }
    if let Some(last) = words
        .last_mut()
        .filter(|last| last.ends_with('s') && !last.ends_with("ss"))
    {
        last.pop();
    }
    words
}
