//! Access warnings come from the saved settings, in K1 order (RA-020, D-5).

use crate::fleet::devices::{
    remote::{RdpSecurity, RdpSettings, VncSettings},
    warnings::{AccessWarning, access_warnings},
};

fn rdp(enabled: bool, ignore_certificate: bool, security: RdpSecurity) -> RdpSettings {
    RdpSettings {
        enabled,
        security,
        ignore_certificate,
        ..RdpSettings::default()
    }
}

fn vnc(enabled: bool) -> VncSettings {
    VncSettings {
        enabled,
        ..VncSettings::default()
    }
}

#[test]
fn ra_020_default_settings_earn_no_warning() {
    assert_eq!(
        access_warnings(&RdpSettings::default(), &VncSettings::default()),
        []
    );
}

#[test]
fn ra_020_every_weak_setting_earns_its_warning_in_k1_order() {
    assert_eq!(
        access_warnings(&rdp(true, true, RdpSecurity::Rdp), &vnc(true)),
        [
            AccessWarning::IgnoredCertificate,
            AccessWarning::RdpSecurity,
            AccessWarning::UnencryptedVnc,
        ]
    );
}

#[test]
fn ra_020_each_warning_stands_alone() {
    let off = vnc(false);
    assert_eq!(
        access_warnings(&rdp(true, true, RdpSecurity::Nla), &off),
        [AccessWarning::IgnoredCertificate]
    );
    assert_eq!(
        access_warnings(&rdp(true, false, RdpSecurity::Rdp), &off),
        [AccessWarning::RdpSecurity]
    );
    assert_eq!(
        access_warnings(&RdpSettings::default(), &vnc(true)),
        [AccessWarning::UnencryptedVnc]
    );
}

#[test]
fn ra_020_only_rdp_security_rdp_warns_of_the_security_layer() {
    for security in [
        RdpSecurity::Any,
        RdpSecurity::Nla,
        RdpSecurity::NlaExt,
        RdpSecurity::Tls,
        RdpSecurity::Vmconnect,
    ] {
        assert_eq!(
            access_warnings(&rdp(true, false, security), &vnc(false)),
            [],
            "{security:?}"
        );
    }
}

#[test]
fn ra_020_a_protocol_that_is_off_earns_none() {
    assert_eq!(
        access_warnings(&rdp(false, true, RdpSecurity::Rdp), &vnc(false)),
        []
    );
}
