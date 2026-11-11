//! The STREAM BE32 layout by a known answer (G-2): each part opens with a plain
//! `XChaCha20Poly1305` under the nonce prefix ‖ big-endian counter ‖ last flag and the header
//! as associated data; and the counter never wraps.

use chacha20poly1305::{KeyInit as _, XChaCha20Poly1305, XNonce, aead::Aead as _, aead::Payload};

use super::{KEY, PREFIX, seal};
use crate::sessions::recording::format::{
    Damage, Opener, SealError, Sealer,
    part::{self, Place},
};

/// Opens a framed part with the cipher alone and a nonce built here, by hand.
fn open_by_hand(header: &[u8], framed: &[u8], counter: u32, flag: u8) -> Option<Vec<u8>> {
    let length = u32::from_be_bytes(framed[..4].try_into().unwrap());
    assert_eq!(usize::try_from(length).unwrap(), framed.len() - 4);
    let mut nonce = PREFIX.to_vec();
    nonce.extend_from_slice(&counter.to_be_bytes());
    nonce.push(flag);
    let nonce = XNonce::try_from(nonce.as_slice()).unwrap();
    let payload = Payload {
        msg: &framed[4..],
        aad: header,
    };
    XChaCha20Poly1305::new(&KEY.into())
        .decrypt(&nonce, payload)
        .ok()
}

#[test]
fn k4_11_each_part_opens_under_prefix_then_be32_counter_then_last_flag() {
    let sealed = seal(&[b"part zero", b"part one"], b"part two");
    let h = &sealed.header;

    assert_eq!(
        open_by_hand(h, &sealed.parts[0], 0, 0),
        Some(b"part zero".to_vec())
    );
    assert_eq!(
        open_by_hand(h, &sealed.parts[1], 1, 0),
        Some(b"part one".to_vec())
    );
    assert_eq!(
        open_by_hand(h, &sealed.parts[2], 2, 1),
        Some(b"part two".to_vec())
    );
}

#[test]
fn k4_11_a_part_opens_under_no_other_counter_flag_or_associated_data() {
    let sealed = seal(&[b"part zero", b"part one"], b"part two");
    let h = &sealed.header;

    assert_eq!(open_by_hand(h, &sealed.parts[1], 1, 1), None);
    assert_eq!(open_by_hand(h, &sealed.parts[1], 0, 0), None);
    assert_eq!(open_by_hand(h, &sealed.parts[1], 1 << 24, 0), None);
    assert_eq!(open_by_hand(h, &sealed.parts[2], 2, 0), None);
    assert_eq!(
        open_by_hand(&h[..h.len() - 1], &sealed.parts[0], 0, 0),
        None
    );
}

#[test]
fn k4_11_the_counter_bytes_are_big_endian_in_nonce_bytes_19_to_22() {
    let nonce = part::nonce(
        &PREFIX,
        Place {
            counter: 0x0102_0304,
            last: true,
        },
    );

    assert_eq!(nonce[..19], PREFIX);
    assert_eq!(nonce[19..], [1, 2, 3, 4, 1]);
}

#[test]
fn k4_11_no_part_before_the_last_takes_the_counters_final_value() {
    let mut sealer = Sealer::new(super::key(), &super::header()).at_counter(u32::MAX - 1);

    let before_final = sealer.seal(b"fits").unwrap();
    let refused = sealer.seal(b"no room for the last part");
    let last = sealer.finish(b"the last part").unwrap();

    assert_eq!(refused, Err(SealError::TooManyParts));
    let h = super::header().encode();
    assert_eq!(
        open_by_hand(&h, &before_final, u32::MAX - 1, 0),
        Some(b"fits".to_vec())
    );
    assert_eq!(
        open_by_hand(&h, &last, u32::MAX, 1),
        Some(b"the last part".to_vec())
    );
}

#[test]
fn k4_11_the_opener_refuses_a_part_before_the_last_at_the_counters_final_value() {
    let header = super::header().encode();
    let place = |last| Place {
        counter: u32::MAX,
        last,
    };
    let not_last = part::seal(&super::key(), &header, place(false), b"x");
    let last = part::seal(&super::key(), &header, place(true), b"y");
    let opener = || {
        Opener::new(super::key(), &header, super::recording())
            .unwrap()
            .at_counter(u32::MAX)
    };

    let refused = opener().open(not_last[4..].to_vec()).err();
    let mut final_opener = opener();
    let last_part = final_opener.open(last[4..].to_vec()).unwrap();

    assert_eq!(refused, Some(Damage::TooManyParts));
    assert!(last_part.last);
    assert_eq!(*last_part.plaintext, b"y");
    assert!(final_opener.is_finished());
    assert_eq!(
        final_opener.part_length([0, 0, 0, 16]),
        Err(Damage::AfterLast)
    );
}
