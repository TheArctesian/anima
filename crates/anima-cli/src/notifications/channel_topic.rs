//! `anima notify channel topic`: a random topic for an ntfy channel (REM-008). A person who
//! knows the topic of a channel on a public ntfy server reads its notifications, and the hub
//! sends no reminder to an ntfy channel whose topic is shorter than 16 characters.

use serde::Serialize;

use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::Table;

/// The characters of a topic: each is permitted in the topic of ntfy.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
/// The length of the topic: 22 characters of 6 bits each are 132 random bits.
pub const LENGTH: usize = 22;

#[derive(Debug, Serialize)]
struct Offered {
    topic: String,
}

/// The topic that `random` gives: one character for each byte, from its low 6 bits.
fn topic(random: [u8; LENGTH]) -> String {
    random
        .iter()
        .map(|byte| char::from(ALPHABET[usize::from(byte & 63)]))
        .collect()
}

/// Prints a new random topic of 22 characters. The hub is not asked and nothing is stored: put
/// the topic in the document of `anima notify channel create`.
///
/// # Errors
/// [`Failure::Input`] when the operating system gives no random bytes.
pub fn run(invocation: &Invocation) -> Result<(), Failure> {
    let mut random = [0_u8; LENGTH];
    getrandom::fill(&mut random).map_err(|error| Failure::Input {
        what: "random bytes of the operating system".to_owned(),
        reason: error.to_string(),
    })?;
    let offered = Offered {
        topic: topic(random),
    };
    invocation.output.data(&offered, |offered| {
        let mut table = Table::record();
        table.pair("Topic", offered.topic.as_str());
        table
    })
}

#[cfg(test)]
mod tests {
    use super::{LENGTH, topic};

    #[test]
    fn rem_008_a_topic_has_22_characters_that_ntfy_permits() {
        let low = topic([0; LENGTH]);
        let high = topic([255; LENGTH]);
        assert_eq!((low.as_str(), high.len()), ("AAAAAAAAAAAAAAAAAAAAAA", 22));
        assert_eq!(high, "______________________");
        let mixed = topic(std::array::from_fn(|index| {
            u8::try_from(index * 11).unwrap_or_default()
        }));
        assert!(
            mixed
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "{mixed}"
        );
    }
}
