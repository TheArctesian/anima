//! The remote access inputs (K1 `RdpSettingsInput`, `VncSettingsInput`, `TransferPolicyInput`).
//! Each converts to the form types the pure rules check.

use async_graphql::InputObject;

use crate::devices::{
    FieldError, RdpSecurity, RdpSettings, ResizeMethod, TransferPolicy, VncSettings,
    remote::{RdpInput, VncInput},
    rules::{rdp_settings, vnc_settings},
};

#[derive(InputObject)]
#[graphql(name = "RdpSettingsInput")]
pub struct RdpSettingsInput {
    enabled: bool,
    #[graphql(default_with = "Some(3389)")]
    port: Option<i32>,
    username: String,
    domain: Option<String>,
    #[graphql(default_with = "Some(RdpSecurity::Nla)")]
    security: Option<RdpSecurity>,
    #[graphql(default_with = "Some(false)")]
    ignore_certificate: Option<bool>,
    #[graphql(default_with = "Some(ResizeMethod::DisplayUpdate)")]
    resize_method: Option<ResizeMethod>,
    #[graphql(default_with = "Some(true)")]
    audio_out: Option<bool>,
    #[graphql(default_with = "Some(false)")]
    microphone: Option<bool>,
    #[graphql(default_with = "Some(false)")]
    shared_drive: Option<bool>,
}

impl RdpSettingsInput {
    /// The RDP port as sent, K1's default 3389 when left out, unchecked.
    #[must_use]
    pub const fn port(&self) -> Option<i32> {
        self.port
    }

    /// The settings, checked by the rules that guard `createDevice` and `updateDevice`.
    ///
    /// # Errors
    /// Every invalid field (`rdp.port`, `rdp.username`, `rdp.domain`), all at once.
    pub fn validated(self) -> Result<RdpSettings, Vec<FieldError>> {
        rdp_settings(&RdpInput::from(self))
    }
}

impl From<RdpSettingsInput> for RdpInput {
    fn from(input: RdpSettingsInput) -> Self {
        let default = RdpSettings::default();
        Self {
            enabled: input.enabled,
            port: input.port,
            username: input.username,
            domain: input.domain,
            security: input.security.unwrap_or(default.security),
            ignore_certificate: input
                .ignore_certificate
                .unwrap_or(default.ignore_certificate),
            resize: input.resize_method.unwrap_or(default.resize),
            audio_out: input.audio_out.unwrap_or(default.audio_out),
            microphone: input.microphone.unwrap_or(default.microphone),
            shared_drive: input.shared_drive.unwrap_or(default.shared_drive),
        }
    }
}

#[derive(InputObject)]
#[graphql(name = "VncSettingsInput")]
pub struct VncSettingsInput {
    enabled: bool,
    #[graphql(default_with = "Some(5900)")]
    port: Option<i32>,
    username: Option<String>,
}

impl VncSettingsInput {
    /// The VNC port as sent, K1's default 5900 when left out, unchecked.
    #[must_use]
    pub const fn port(&self) -> Option<i32> {
        self.port
    }

    /// The settings, checked by the rules that guard `createDevice` and `updateDevice`.
    ///
    /// # Errors
    /// Every invalid field (`vnc.port`, `vnc.username`), all at once.
    pub fn validated(self) -> Result<VncSettings, Vec<FieldError>> {
        vnc_settings(&VncInput::from(self))
    }
}

impl From<VncSettingsInput> for VncInput {
    fn from(input: VncSettingsInput) -> Self {
        Self {
            enabled: input.enabled,
            port: input.port,
            username: input.username,
        }
    }
}

#[derive(InputObject)]
#[graphql(name = "TransferPolicyInput")]
pub struct TransferPolicyInput {
    #[graphql(default_with = "Some(true)")]
    upload: Option<bool>,
    #[graphql(default_with = "Some(true)")]
    download: Option<bool>,
    #[graphql(default_with = "Some(true)")]
    copy: Option<bool>,
    #[graphql(default_with = "Some(true)")]
    paste: Option<bool>,
}

impl From<TransferPolicyInput> for TransferPolicy {
    fn from(input: TransferPolicyInput) -> Self {
        let default = Self::default();
        Self {
            upload: input.upload.unwrap_or(default.upload),
            download: input.download.unwrap_or(default.download),
            copy: input.copy.unwrap_or(default.copy),
            paste: input.paste.unwrap_or(default.paste),
        }
    }
}
