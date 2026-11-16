//! A limit on what a device may send on a virtual channel (the clipboard, the channel for
//! the screen size). `IronRDP` joins the parts of a channel message with no limit of its
//! own, so a hostile device could fill the worker's memory before the gateway sees the
//! message. The guard counts the parts first.
//!
//! Pure: no I/O.

use ironrdp_pdu::mcs::decode_send_data_indication;
use ironrdp_pdu::rdp::vc::ChannelControlFlags;

use super::device::Malformed;

/// The most bytes of one channel message: the longest clipboard text in UTF-16, and room
/// for its headers.
const MOST_MESSAGE_BYTES: usize = (2 << 20) + 4096;
/// The channel header: the length of the whole message, then the flags.
const HEADER_BYTES: usize = 8;

/// The channel messages that are arriving in parts.
pub(super) struct ChannelGuard {
    /// The virtual channels of the session, each with the bytes of its message so far.
    channels: Vec<(u16, usize)>,
}

impl ChannelGuard {
    pub(super) fn new(channels: impl IntoIterator<Item = u16>) -> Self {
        Self {
            channels: channels.into_iter().map(|channel| (channel, 0)).collect(),
        }
    }

    /// Checks one X.224 message of the device before `IronRDP` gets it.
    ///
    /// # Errors
    /// [`Malformed`] for a channel message that is packed, though no packing was agreed,
    /// or longer than the limit.
    pub(super) fn check(&mut self, message: &[u8]) -> Result<(), Malformed> {
        let Ok(data) = decode_send_data_indication(message) else {
            return Ok(());
        };
        let Some((_, so_far)) = self
            .channels
            .iter_mut()
            .find(|(channel, _)| *channel == data.channel_id)
        else {
            return Ok(());
        };
        let Some((header, part)) = data.user_data.split_first_chunk::<HEADER_BYTES>() else {
            return Err(Malformed);
        };
        let [l0, l1, l2, l3, f0, f1, f2, f3] = *header;
        let whole = u32::from_le_bytes([l0, l1, l2, l3]);
        let flags = ChannelControlFlags::from_bits_retain(u32::from_le_bytes([f0, f1, f2, f3]));
        if flags.contains(ChannelControlFlags::FLAG_FIRST) {
            *so_far = 0;
        }
        *so_far += part.len();
        let too_long = usize::try_from(whole).map_or(true, |whole| whole > MOST_MESSAGE_BYTES)
            || *so_far > MOST_MESSAGE_BYTES;
        if too_long || flags.contains(ChannelControlFlags::PACKET_COMPRESSED) {
            return Err(Malformed);
        }
        if flags.contains(ChannelControlFlags::FLAG_LAST) {
            *so_far = 0;
        }
        Ok(())
    }
}
