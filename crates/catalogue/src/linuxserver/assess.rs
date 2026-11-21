//! The place of one image in the report of the fleet (Section 15.10 of the proposal).

use super::{Image, Pin, Unfit, plan::HARDWARE_DECODING, recipe};

/// How an image fits the rules of a file from the hub.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Assessment {
    /// The recipe has each part that the publisher marks as not optional.
    Fits,
    /// It fits, with a port of the machine below 1024 moved up by 8000.
    FitsWithPortMoved,
    /// It fits, with a device, a capability, a `security_opt` or a Compose key that the
    /// publisher marks as optional left off.
    FitsWithPartOff,
    Unfit(Unfit),
}

/// True when the image has a part that the form leaves off.
fn part_off(image: &Image) -> bool {
    let config = &image.config;
    HARDWARE_DECODING.contains(&image.name.as_str())
        || config.devices.iter().any(|device| device.optional)
        || config.caps.iter().any(|capability| capability.optional)
        || config.security_opt.iter().any(|option| option.optional)
        || config.custom.iter().any(|custom| custom.optional)
}

/// True when a port of the machine that is not optional is below 1024.
fn port_moved(image: &Image) -> bool {
    image
        .config
        .ports
        .iter()
        .filter(|port| !port.optional)
        .any(|port| {
            port.external
                .parse::<u16>()
                .is_ok_and(|external| external < 1024)
        })
}

/// The place of `image` in the report, with `pin` for its recipe.
#[must_use]
pub fn assess(image: &Image, pin: &Pin) -> Assessment {
    match recipe(image, pin) {
        Err(reason) => Assessment::Unfit(reason),
        Ok(_) if part_off(image) => Assessment::FitsWithPartOff,
        Ok(_) if port_moved(image) => Assessment::FitsWithPortMoved,
        Ok(_) => Assessment::Fits,
    }
}
