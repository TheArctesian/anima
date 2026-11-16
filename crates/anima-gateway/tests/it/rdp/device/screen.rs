//! What the test device sends about its screen: its capabilities, painted rectangles as
//! fast-path bitmap updates, and the Deactivate All that starts a resize.

use std::borrow::Cow;

use anima_gateway::screen::Area;
use ironrdp_acceptor::{AcceptorResult, DesktopSize};
use ironrdp_core::{Encode, encode_vec};
use ironrdp_pdu::bitmap::{BitmapData, BitmapUpdateData, Compression};
use ironrdp_pdu::fast_path::{
    EncryptionFlags, FastPathHeader, FastPathUpdatePdu, Fragmentation, UpdateCode,
};
use ironrdp_pdu::geometry::InclusiveRectangle;
use ironrdp_pdu::mcs::SendDataIndication;
use ironrdp_pdu::rdp::capability_sets::{
    Bitmap, BitmapDrawingFlags, CapabilitySet, General, GeneralExtraFlags, Input, InputFlags,
};
use ironrdp_pdu::rdp::headers::{ServerDeactivateAll, ShareControlHeader, ShareControlPdu};
use ironrdp_pdu::x224::X224;

/// The bytes of a 32-bit pixel.
const PIXEL: usize = 4;
/// The most bitmap bytes in one fast-path update: its length field and the fast-path
/// PDU's own (at most 0x7fff bytes) leave room for this and the headers.
const MOST_BYTES: usize = 0x7000;

/// What the device can do: fast-path output, uncompressed 32-bit bitmaps of its size, and
/// fast-path input of scancodes, Unicode and the pointer.
pub(super) fn capabilities(size: DesktopSize) -> Vec<CapabilitySet> {
    let general = General {
        extra_flags: GeneralExtraFlags::FASTPATH_OUTPUT_SUPPORTED
            | GeneralExtraFlags::NO_BITMAP_COMPRESSION_HDR,
        ..General::default()
    };
    let bitmap = Bitmap {
        pref_bits_per_pix: 32,
        desktop_width: size.width,
        desktop_height: size.height,
        desktop_resize_flag: true,
        drawing_flags: BitmapDrawingFlags::empty(),
    };
    let input = Input {
        input_flags: InputFlags::SCANCODES
            | InputFlags::MOUSEX
            | InputFlags::UNICODE
            | InputFlags::FASTPATH_INPUT
            | InputFlags::FASTPATH_INPUT_2
            | InputFlags::TS_MOUSE_HWHEEL,
        keyboard_layout: 0,
        keyboard_type: None,
        keyboard_subtype: 0,
        keyboard_function_key: 0,
        keyboard_ime_filename: String::new(),
    };
    vec![
        CapabilitySet::General(general),
        CapabilitySet::Bitmap(bitmap),
        CapabilitySet::Input(input),
    ]
}

/// `area` in one colour (red, green, blue), as fast-path PDUs of uncompressed 32-bit
/// bitmaps: one for each band of rows that fits in an update.
pub(super) fn fill(area: Area, [red, green, blue]: [u8; 3]) -> Vec<u8> {
    let row = area.width * PIXEL;
    assert!(row > 0 && row <= MOST_BYTES, "a row fits in one update");
    let band = MOST_BYTES / row;
    let mut pdus = Vec::new();
    for top in (area.y..area.y + area.height).step_by(band) {
        let rows = band.min(area.y + area.height - top);
        // RDP bitmaps are bottom-up with rows padded to four bytes; 32-bit rows need no
        // padding and the rows of one colour are all the same.
        let pixels = [blue, green, red, 0].repeat(area.width * rows);
        let rectangle = InclusiveRectangle {
            left: wire(area.x),
            top: wire(top),
            right: wire(area.x + area.width - 1),
            bottom: wire(top + rows - 1),
        };
        let bitmap = BitmapData {
            rectangle,
            width: wire(area.width),
            height: wire(rows),
            bits_per_pixel: 32,
            compression_flags: Compression::empty(),
            compressed_data_header: None,
            bitmap_data: &pixels,
        };
        let update = BitmapUpdateData {
            rectangles: vec![bitmap],
        };
        pdus.extend(fast_path(&encode_vec(&update).expect("a bitmap update")));
    }
    pdus
}

/// One fast-path PDU with one bitmap update in one piece.
fn fast_path(update: &[u8]) -> Vec<u8> {
    let update = FastPathUpdatePdu {
        fragmentation: Fragmentation::Single,
        update_code: UpdateCode::Bitmap,
        compression_flags: None,
        compression_type: None,
        data: update,
    };
    let header = FastPathHeader::new(EncryptionFlags::empty(), update.size());
    let mut pdu = encode_vec(&header).expect("a fast-path header");
    pdu.extend(encode_vec(&update).expect("a fast-path update"));
    pdu
}

/// The Deactivate All PDU on the session's I/O channel.
pub(super) fn deactivate_all(session: &AcceptorResult) -> Vec<u8> {
    let deactivate = ShareControlHeader {
        share_id: 0,
        pdu_source: session.io_channel_id,
        share_control_pdu: ShareControlPdu::ServerDeactivateAll(ServerDeactivateAll),
    };
    let indication = SendDataIndication {
        initiator_id: session.user_channel_id,
        channel_id: session.io_channel_id,
        user_data: Cow::Owned(encode_vec(&deactivate).expect("a Deactivate All")),
    };
    encode_vec(&X224(indication)).expect("an X.224 PDU")
}

/// A coordinate or length on the wire.
fn wire(value: usize) -> u16 {
    u16::try_from(value).expect("a screen coordinate")
}
