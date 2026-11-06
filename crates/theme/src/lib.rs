//! The Nord palette and the terminal colour scheme the hub hands to guacd (RA-010).
//!
//! The palette is generated from `design/tokens` (TOK-001); this file holds the colour type and
//! the guacd `color-scheme` text built from it.

mod generated;

pub use generated::{NORD, TERMINAL_ANSI, TERMINAL_BACKGROUND, TERMINAL_FOREGROUND};

/// An 8-bit-per-channel sRGB colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// The X11 colour spec `rgb:RR/GG/BB` in lower-case hex.
    fn x11(self) -> String {
        format!("rgb:{:02x}/{:02x}/{:02x}", self.r, self.g, self.b)
    }
}

/// The Nord terminal palette as guacd's `color-scheme` connection parameter (RA-010).
///
/// Format: `name: rgb:RR/GG/BB` entries separated by `;`, for `background`, `foreground` and
/// `color0` to `color15`. Source: the Guacamole manual, "Configuring Guacamole", SSH/telnet
/// terminal display parameter `color-scheme`, which takes X11 `rgb:` colour specs.
#[must_use]
pub fn guac_color_scheme() -> String {
    let named = [
        ("background".to_owned(), TERMINAL_BACKGROUND),
        ("foreground".to_owned(), TERMINAL_FOREGROUND),
    ];
    let ansi = TERMINAL_ANSI
        .iter()
        .enumerate()
        .map(|(index, colour)| (format!("color{index}"), *colour));
    named
        .into_iter()
        .chain(ansi)
        .map(|(name, colour)| format!("{name}: {}", colour.x11()))
        .collect::<Vec<_>>()
        .join(";")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ra_010_nord_8_is_88c0d0() {
        assert_eq!(NORD[8], Rgb::new(0x88, 0xC0, 0xD0));
    }

    #[test]
    fn ra_010_nord_0_is_2e3440() {
        assert_eq!(NORD[0], Rgb::new(0x2E, 0x34, 0x40));
    }

    #[test]
    fn ra_010_color_scheme_is_the_exact_guacd_text() {
        let expected = "background: rgb:2e/34/40;foreground: rgb:d8/de/e9;\
            color0: rgb:3b/42/52;color1: rgb:bf/61/6a;color2: rgb:a3/be/8c;color3: rgb:eb/cb/8b;\
            color4: rgb:81/a1/c1;color5: rgb:b4/8e/ad;color6: rgb:88/c0/d0;color7: rgb:e5/e9/f0;\
            color8: rgb:4c/56/6a;color9: rgb:bf/61/6a;color10: rgb:a3/be/8c;color11: rgb:eb/cb/8b;\
            color12: rgb:81/a1/c1;color13: rgb:b4/8e/ad;color14: rgb:8f/bc/bb;color15: rgb:ec/ef/f4";
        assert_eq!(guac_color_scheme(), expected);
    }

    #[test]
    fn ra_010_color_scheme_has_background_foreground_and_sixteen_colours() {
        let scheme = guac_color_scheme();
        assert_eq!(scheme.split(';').count(), 18);
    }

    #[test]
    fn ra_010_ansi_red_is_nord_11() {
        assert_eq!(TERMINAL_ANSI[1], NORD[11]);
        assert!(guac_color_scheme().contains("color1: rgb:bf/61/6a"));
    }
}
