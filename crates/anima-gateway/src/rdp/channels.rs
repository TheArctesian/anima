//! A limit on what a device may send in parts: a message on a virtual channel (the
//! clipboard, the channel for the screen size) and a drawing update. `IronRDP` joins the
//! parts with no limit of its own, so a hostile device could fill the worker's memory
//! before the gateway sees the whole. The guard counts the parts first.
//!
//! Pure: no I/O.

use ironrdp_pdu::mcs::decode_send_data_indication;
use ironrdp_pdu::rdp::vc::ChannelControlFlags;

use super::device::Malformed;

/// The most bytes of one channel message: the longest clipboard text in UTF-16, and room
/// for its headers.
const MOST_MESSAGE_BYTES: usize = (2 << 20) + 4096;
/// The most bytes of one drawing update: what the client tells the device it takes
/// (`IronRDP`'s multi-fragment update capability).
const MOST_UPDATE_BYTES: usize = 8 << 20;
/// The channel header: the length of the whole message, then the flags.
const HEADER_BYTES: usize = 8;

/// The channel messages that are arriving in parts.
pub(super) struct ChannelGuard {
    /// The virtual channels of the session, each with the bytes of its message so far.
    channels: Vec<(u16, usize)>,
    /// The bytes of the drawing update that is arriving in parts.
    update: usize,
}

impl ChannelGuard {
    pub(super) fn new(channels: impl IntoIterator<Item = u16>) -> Self {
        Self {
            channels: channels.into_iter().map(|channel| (channel, 0)).collect(),
            update: 0,
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
        let first = flags.contains(ChannelControlFlags::FLAG_FIRST);
        let last = flags.contains(ChannelControlFlags::FLAG_LAST);
        if first {
            *so_far = 0;
        }
        // A part with neither flag and no message open is a whole message for `IronRDP`.
        if first || last || *so_far > 0 {
            *so_far += part.len();
        }
        let too_long = usize::try_from(whole).map_or(true, |whole| whole > MOST_MESSAGE_BYTES)
            || *so_far > MOST_MESSAGE_BYTES;
        if too_long || flags.contains(ChannelControlFlags::PACKET_COMPRESSED) {
            return Err(Malformed);
        }
        if last {
            *so_far = 0;
        }
        Ok(())
    }

    /// Checks one fast-path message of the device, which holds drawing updates, before
    /// `IronRDP` gets it. A message this cannot read is left to `IronRDP` to refuse.
    ///
    /// # Errors
    /// [`Malformed`] for an update that is packed, though no packing was agreed, or whose
    /// parts are longer than the limit.
    pub(super) fn check_updates(&mut self, message: &[u8]) -> Result<(), Malformed> {
        // The header byte, then the length in one byte or, with the top bit, in two.
        let long = message.get(1).is_some_and(|length| length & 0x80 != 0);
        let mut updates = message.get(if long { 3 } else { 2 }..).unwrap_or_default();
        // Each update: its header, its size in two bytes, its data.
        while let Some((&[header, low, high], rest)) = updates.split_first_chunk::<3>() {
            if header & 0xc0 != 0 {
                return Err(Malformed);
            }
            let size = usize::from(u16::from_le_bytes([low, high]));
            // 0 is a whole update, 1 the last part, 2 the first and 3 one between.
            match (header >> 4) & 0x3 {
                2 => self.update = size,
                3 => self.update += size,
                _ => self.update = 0,
            }
            if self.update > MOST_UPDATE_BYTES {
                return Err(Malformed);
            }
            updates = rest.get(size..).unwrap_or_default();
        }
        Ok(())
    }
}
