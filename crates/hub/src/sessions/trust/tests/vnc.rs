//! The VNC version banner a connection test reads (DEV-013, RFC 6143 7.1.1).

use super::super::vnc::{BANNER, is_banner};

#[test]
fn dev_013_rfb_version_banners_are_recognised() {
    for banner in [b"RFB 003.003\n", b"RFB 003.008\n", b"RFB 003.889\n"] {
        assert!(is_banner(banner), "{banner:?}");
    }
}

#[test]
fn dev_013_other_first_bytes_are_not_a_banner() {
    let cases: [&[u8; BANNER]; 5] = [
        b"SSH-2.0-Open",
        b"RFB 003.008 ",
        b"RFB 0x3.008\n",
        b"rfb 003.008\n",
        b"RFB 003,008\n",
    ];
    for case in cases {
        assert!(!is_banner(case), "{case:?}");
    }
}
