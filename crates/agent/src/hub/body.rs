//! The most of an answer the agent reads, kept in a buffer that is wiped on every path.

use zeroize::Zeroizing;

/// The most of an answer the agent reads: far above any K8 answer.
pub const LIMIT: usize = 64 * 1024;

/// What has come of an answer so far. Room for [`LIMIT`] bytes is made at the start and the
/// buffer never grows past it, so it never moves and its wipe on drop covers every byte.
/// (hyper's and rustls's own buffers are not wiped: a known remainder.)
pub struct Body(Zeroizing<Vec<u8>>);

impl Body {
    #[must_use]
    pub fn new() -> Self {
        Self(Zeroizing::new(Vec::with_capacity(LIMIT)))
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        self.0.len() >= LIMIT
    }

    /// Keeps as much of `chunk` as fits under [`LIMIT`].
    pub fn add(&mut self, chunk: &[u8]) {
        let room = LIMIT - self.0.len();
        self.0.extend_from_slice(&chunk[..chunk.len().min(room)]);
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Default for Body {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn met_020_the_buffer_never_grows_past_its_room_so_it_never_moves() {
        let mut body = Body::new();
        let (room, place) = (body.0.capacity(), body.0.as_ptr());
        let chunk = vec![7_u8; 40 * 1024];
        for _ in 0..3 {
            body.add(&chunk);
        }
        assert!(body.is_full());
        assert_eq!(body.bytes().len(), LIMIT, "the third chunk is cut to fit");
        assert_eq!(body.0.capacity(), room);
        assert_eq!(body.0.as_ptr(), place, "no reallocation, so no unwiped copy");
    }

    #[test]
    fn met_020_a_buffer_below_the_limit_is_not_full() {
        let mut body = Body::new();
        body.add(&[1; LIMIT - 1]);
        assert!(!body.is_full());
        body.add(&[1]);
        assert!(body.is_full());
    }
}
