//! What a device web view trusts (WEB-008, D-4, D-5): the fingerprint form, the verdict on a
//! server-trust challenge, the answer the trust hook gives it and the leaf-certificate probe
//! for the system-browser mode (D-9).

#[cfg_attr(
    not(any(target_os = "macos", test)),
    expect(
        dead_code,
        reason = "only macOS has a trust hook until the operator runs OD-7 on Linux"
    )
)]
mod answer;
mod certificate;
mod fingerprint;
mod pin;

#[cfg_attr(
    not(any(target_os = "macos", test)),
    expect(
        unused_imports,
        reason = "only macOS has a trust hook until the operator runs OD-7 on Linux"
    )
)]
pub use answer::{Answer, Presented, rule};
pub use certificate::{CertificateError, leaf_fingerprint};
pub use fingerprint::{Fingerprint, FingerprintError};
#[cfg(test)]
pub(super) use pin::verdict;
#[cfg(test)]
pub(super) use pin::{Challenge, Host};
pub use pin::{OriginError, Pin, PinnedOrigin, Verdict};
