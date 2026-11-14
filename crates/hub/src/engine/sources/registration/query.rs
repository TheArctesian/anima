//! Reading one value from the query of a redirect back from GitHub or of the ticket link.
//! Pure. The text is hostile: nothing is decoded, and a name given twice gives nothing.
//!
//! Every value the flow reads (a ticket, a state, a code, an installation ID) is plain base64url
//! or digits, so a value that needs percent-decoding could never match and is never decoded.

/// The longest query read; longer text gives nothing.
const MAX_QUERY: usize = 4096;

/// The value of `name` in `raw`, when it is there exactly once and not empty.
#[must_use]
pub fn value<'q>(raw: Option<&'q str>, name: &str) -> Option<&'q str> {
    let raw = raw.filter(|raw| raw.len() <= MAX_QUERY)?;
    let mut found = None;
    for pair in raw.split('&') {
        let (key, text) = pair.split_once('=').unwrap_or((pair, ""));
        if key != name {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some(text);
    }
    found.filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::value;

    #[test]
    fn prj_041_one_value_is_read_and_a_repeated_name_gives_nothing() {
        let cases = [
            (Some("ticket=abc"), Some("abc")),
            (Some("a=1&ticket=abc&b=2"), Some("abc")),
            (Some("ticket=abc&ticket=abc"), None),
            (Some("ticket=abc&ticket="), None),
            (Some("ticket="), None),
            (Some("ticket"), None),
            (Some("tickets=abc"), None),
            (Some("Ticket=abc"), None),
            (Some(""), None),
            (None, None),
            (Some("ticket=a%2Fb"), Some("a%2Fb")),
            (Some("ticket=a=b"), Some("a=b")),
        ];
        for (raw, expected) in cases {
            assert_eq!(value(raw, "ticket"), expected, "{raw:?}");
        }
        let long = format!("ticket=abc&pad={}", "x".repeat(4096));
        assert_eq!(value(Some(&long), "ticket"), None);
    }

    proptest! {
        #[test]
        fn prj_041_any_query_reads_without_panic_and_a_value_is_a_substring(raw in ".{0,300}") {
            if let Some(found) = value(Some(&raw), "state") {
                prop_assert!(raw.contains(found));
                prop_assert!(!found.is_empty());
                prop_assert!(!found.contains('&'));
            }
        }
    }
}
