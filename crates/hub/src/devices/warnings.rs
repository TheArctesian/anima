//! Access warnings: the weak settings the client marks with a badge (RA-020).

#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;

use super::remote::{RdpSecurity, RdpSettings, VncSettings};

/// A saved setting that weakens the security of a session. K1 lists them in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub(in crate::devices) enum AccessWarning {
    /// RDP is on and the server certificate is not checked.
    IgnoredCertificate,
    /// RDP is on with the legacy RDP security layer.
    RdpSecurity,
    /// VNC is on; VNC in Phase 2 has no TLS option, so it is never encrypted.
    UnencryptedVnc,
}

/// The warnings the saved RDP and VNC settings earn, in K1 order. A protocol that is off earns
/// none.
pub(in crate::devices) fn access_warnings(
    rdp: &RdpSettings,
    vnc: &VncSettings,
) -> Vec<AccessWarning> {
    [
        (
            rdp.enabled && rdp.ignore_certificate,
            AccessWarning::IgnoredCertificate,
        ),
        (
            rdp.enabled && rdp.security == RdpSecurity::Rdp,
            AccessWarning::RdpSecurity,
        ),
        (vnc.enabled, AccessWarning::UnencryptedVnc),
    ]
    .into_iter()
    .filter_map(|(earned, warning)| earned.then_some(warning))
    .collect()
}
