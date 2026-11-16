//! A changed area of `IronRDP`'s picture into the gateway's screen copy, which finds what
//! is new in it.

use ironrdp_pdu::geometry::InclusiveRectangle;
use ironrdp_session::image::DecodedImage;

use super::device::Malformed;
use crate::screen::{Area, BYTES_PER_PIXEL, Screen};

/// Copies `area` of `image` into `screen`. The part of `area` outside the picture is left
/// out.
pub(super) fn draw(
    image: &DecodedImage,
    area: &InclusiveRectangle,
    screen: &mut Screen,
) -> Result<(), Malformed> {
    let (width, height) = (usize::from(image.width()), usize::from(image.height()));
    let (left, top) = (usize::from(area.left), usize::from(area.top));
    let right = (usize::from(area.right) + 1).min(width);
    let bottom = (usize::from(area.bottom) + 1).min(height);
    if left >= right || top >= bottom {
        return Ok(());
    }
    let (pixel, stride) = (image.bytes_per_pixel(), image.stride());
    let mut rgb = Vec::with_capacity((right - left) * (bottom - top) * BYTES_PER_PIXEL);
    for row in image.data().chunks_exact(stride).take(bottom).skip(top) {
        for rgba in row[left * pixel..right * pixel].chunks_exact(pixel) {
            rgb.extend_from_slice(&rgba[..BYTES_PER_PIXEL]);
        }
    }
    let area = Area {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    };
    screen.write(area, &rgb).map_err(|_| Malformed)
}
