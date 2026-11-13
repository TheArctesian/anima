//! The distribution in a device's os-release: quoted and bare values, the last `ID=` line,
//! and junk from a hostile device left out (DEV-016).

use proptest::prelude::*;

use crate::fleet::metrics::collector::os_release::distribution;

fn id(os_release: &[u8]) -> Option<String> {
    distribution(os_release).map(|distribution| distribution.as_str().to_owned())
}

#[test]
fn dev_016_quoted_and_bare_ids_are_read() {
    let pop = b"NAME=\"Pop!_OS\"\nVERSION=\"22.04 LTS\"\nID=pop\nID_LIKE=\"ubuntu debian\"\n";
    assert_eq!(id(pop).as_deref(), Some("pop"));
    assert_eq!(id(b"ID=\"fedora\"\n").as_deref(), Some("fedora"));
    assert_eq!(
        id(b"ID='nixos'").as_deref(),
        Some("nixos"),
        "no final newline"
    );
    assert_eq!(
        id(b"  ID=arch\r\n").as_deref(),
        Some("arch"),
        "spaces and CRLF"
    );
    assert_eq!(
        id(b"ID=debian\nID=linuxmint\n").as_deref(),
        Some("linuxmint"),
        "the last one"
    );
}

#[test]
fn dev_016_other_keys_are_not_the_id() {
    assert_eq!(
        id(b"ID_LIKE=debian\nVERSION_ID=\"12\"\n# ID=debian\n"),
        None
    );
    assert_eq!(id(b""), None);
}

#[test]
fn dev_016_junk_is_not_stored() {
    for junk in [
        &b"ID=Ubuntu\n"[..],
        b"ID=\"pop\n",
        b"ID=\"pop'\n",
        b"ID=\"\"\n",
        b"ID=pop os\n",
        b"ID=$(reboot)\n",
        b"ID=\xff\xfe\n",
        b"ID=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
        b"\x00\x01\x02 binary",
    ] {
        assert_eq!(id(junk), None, "{:?}", String::from_utf8_lossy(junk));
    }
}

/// Whether `text` keeps the token rule: 1 to 32 of `a-z`, `0-9`, `.`, `_` and `-`.
fn is_token(text: &str) -> bool {
    let allowed =
        |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte);
    (1..=32).contains(&text.len()) && text.bytes().all(allowed)
}

fn has_id_line(bytes: &[u8]) -> bool {
    bytes
        .split(|byte| *byte == b'\n')
        .any(|line| line.trim_ascii().starts_with(b"ID="))
}

proptest! {
    /// Whatever value a hostile device puts after `ID=`, the result keeps the token rule or
    /// is nothing.
    #[test]
    fn dev_016_any_value_gives_a_token_or_nothing(
        value in proptest::collection::vec(any::<u8>(), 0..64),
    ) {
        let os_release = [&b"ID="[..], &value].concat();
        if let Some(found) = id(&os_release) {
            prop_assert!(is_token(&found), "{found:?}");
        }
    }

    /// A valid `ID=` line is read whatever bytes come before it and whatever other lines come
    /// after it.
    #[test]
    fn dev_016_a_valid_id_line_is_found_among_any_bytes(
        before in proptest::collection::vec(any::<u8>(), 0..256),
        after in proptest::collection::vec(any::<u8>(), 0..256),
        token in "[a-z0-9._-]{1,32}",
    ) {
        prop_assume!(!has_id_line(&after));
        let os_release = [&before[..], b"\nID=\"", token.as_bytes(), b"\"\n", &after].concat();
        prop_assert_eq!(id(&os_release), Some(token));
    }
}
