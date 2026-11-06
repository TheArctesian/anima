//! What a device web view trusts (WEB-008, D-4, D-5): the fingerprint form, the verdict on a
//! server-trust challenge and the leaf-certificate probe for the system-browser mode (D-9).

mod certificate;
mod fingerprint;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "WP-31 T4's trust hook decides challenges with the verdict"
    )
)]
mod pin;

pub use certificate::{CertificateError, leaf_fingerprint};
pub use fingerprint::{Fingerprint, FingerprintError};
#[cfg(test)]
pub(in crate::web_view) use pin::{Challenge, Host, Verdict, verdict};
pub use pin::{OriginError, Pin, PinnedOrigin};
