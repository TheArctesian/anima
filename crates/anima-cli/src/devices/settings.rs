//! The device settings that `device create` and `device update` take as flags (CMD-020). Each K1
//! input field is a flag with its K1 name in kebab-case. The flags become the K1 input as JSON,
//! and the generated input type of the document reads it back, so K1 still decides what is valid.
//!
//! A group (`--ssh-*`, `--rdp-*`, `--vnc-*`) is sent whole: the hub replaces the group, so
//! the user name is needed whenever any flag of an SSH or RDP group is given.

use clap::ValueEnum;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use crate::failure::Failure;

#[derive(Clone, Copy, Debug, ValueEnum, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum OsArg {
    Linux,
    Macos,
    Windows,
    Other,
}

#[derive(Clone, Copy, Debug, ValueEnum, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum MetricsSourceArg {
    None,
    Ssh,
}

#[derive(Clone, Copy, Debug, ValueEnum, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum SshAuthArg {
    Password,
    PrivateKey,
}

#[derive(Clone, Copy, Debug, ValueEnum, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum RdpSecurityArg {
    Any,
    Nla,
    NlaExt,
    Tls,
    Vmconnect,
    Rdp,
}

#[derive(Clone, Copy, Debug, ValueEnum, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum ResizeMethodArg {
    DisplayUpdate,
    Reconnect,
}

#[derive(Debug, clap::Args)]
pub struct SettingsFlags {
    /// The kind of device, such as server or router.
    #[arg(long)]
    kind: Option<String>,
    /// The operating system.
    #[arg(long, value_enum)]
    os: Option<OsArg>,
    /// A tag; repeat for more than one.
    #[arg(long = "tag")]
    tags: Vec<String>,
    /// The MAC address for Wake-on-LAN, such as aa:bb:cc:dd:ee:ff.
    #[arg(long)]
    mac_address: Option<String>,
    /// Where metrics come from (an agent is set by `anima agent enrol`).
    #[arg(long, value_enum)]
    metrics_source: Option<MetricsSourceArg>,
    /// The most live sessions at once.
    #[arg(long)]
    session_limit: Option<i64>,
    /// Whether SSH is on (`--ssh-*` flags send the whole SSH group).
    #[arg(long)]
    ssh_enabled: Option<bool>,
    #[arg(long)]
    ssh_port: Option<u16>,
    #[arg(long)]
    ssh_username: Option<String>,
    #[arg(long, value_enum)]
    ssh_auth: Option<SshAuthArg>,
    /// Whether RDP is on (`--rdp-*` flags send the whole RDP group).
    #[arg(long)]
    rdp_enabled: Option<bool>,
    #[arg(long)]
    rdp_port: Option<u16>,
    #[arg(long)]
    rdp_username: Option<String>,
    #[arg(long)]
    rdp_domain: Option<String>,
    #[arg(long, value_enum)]
    rdp_security: Option<RdpSecurityArg>,
    #[arg(long)]
    rdp_ignore_certificate: Option<bool>,
    #[arg(long, value_enum)]
    rdp_resize_method: Option<ResizeMethodArg>,
    #[arg(long)]
    rdp_audio_out: Option<bool>,
    #[arg(long)]
    rdp_microphone: Option<bool>,
    #[arg(long)]
    rdp_shared_drive: Option<bool>,
    /// Whether VNC is on (`--vnc-*` flags send the whole VNC group).
    #[arg(long)]
    vnc_enabled: Option<bool>,
    #[arg(long)]
    vnc_port: Option<u16>,
    #[arg(long)]
    vnc_username: Option<String>,
    /// Whether files may be uploaded in a session.
    #[arg(long)]
    transfer_upload: Option<bool>,
    #[arg(long)]
    transfer_download: Option<bool>,
    #[arg(long)]
    transfer_copy: Option<bool>,
    #[arg(long)]
    transfer_paste: Option<bool>,
}

impl SettingsFlags {
    /// The K1 input fields these flags set; a field with no flag is left out.
    ///
    /// # Errors
    /// [`Failure::Usage`] when a group lacks the user name K1 requires.
    pub fn fields(&self) -> Result<Map<String, Value>, Failure> {
        let mut input = Map::new();
        put(&mut input, "kind", self.kind.as_ref());
        put(&mut input, "os", self.os.as_ref());
        put(&mut input, "macAddress", self.mac_address.as_ref());
        put(&mut input, "metricsSource", self.metrics_source.as_ref());
        put(&mut input, "sessionLimit", self.session_limit.as_ref());
        if !self.tags.is_empty() {
            input.insert("tags".to_owned(), json!(self.tags));
        }
        put(&mut input, "ssh", self.ssh()?.as_ref());
        put(&mut input, "rdp", self.rdp()?.as_ref());
        put(&mut input, "vnc", self.vnc()?.as_ref());
        put(&mut input, "transfer", self.transfer().as_ref());
        Ok(input)
    }

    fn ssh(&self) -> Result<Option<Value>, Failure> {
        let mut group = Map::new();
        put(&mut group, "port", self.ssh_port.as_ref());
        put(&mut group, "username", self.ssh_username.as_ref());
        put(&mut group, "auth", self.ssh_auth.as_ref());
        if !group.is_empty() {
            group
                .entry("auth")
                .or_insert_with(|| json!(SshAuthArg::Password));
        }
        complete(
            group,
            self.ssh_enabled,
            Some(("username", "--ssh-username")),
        )
    }

    fn rdp(&self) -> Result<Option<Value>, Failure> {
        let mut group = Map::new();
        put(&mut group, "port", self.rdp_port.as_ref());
        put(&mut group, "username", self.rdp_username.as_ref());
        put(&mut group, "domain", self.rdp_domain.as_ref());
        put(&mut group, "security", self.rdp_security.as_ref());
        put(
            &mut group,
            "ignoreCertificate",
            self.rdp_ignore_certificate.as_ref(),
        );
        put(&mut group, "resizeMethod", self.rdp_resize_method.as_ref());
        put(&mut group, "audioOut", self.rdp_audio_out.as_ref());
        put(&mut group, "microphone", self.rdp_microphone.as_ref());
        put(&mut group, "sharedDrive", self.rdp_shared_drive.as_ref());
        complete(
            group,
            self.rdp_enabled,
            Some(("username", "--rdp-username")),
        )
    }

    fn vnc(&self) -> Result<Option<Value>, Failure> {
        let mut group = Map::new();
        put(&mut group, "port", self.vnc_port.as_ref());
        put(&mut group, "username", self.vnc_username.as_ref());
        complete(group, self.vnc_enabled, None)
    }

    fn transfer(&self) -> Option<Value> {
        let mut group = Map::new();
        put(&mut group, "upload", self.transfer_upload.as_ref());
        put(&mut group, "download", self.transfer_download.as_ref());
        put(&mut group, "copy", self.transfer_copy.as_ref());
        put(&mut group, "paste", self.transfer_paste.as_ref());
        (!group.is_empty()).then_some(Value::Object(group))
    }
}

/// `input` as the K1 input type `T` of a document.
///
/// # Errors
/// [`Failure::Usage`] when K1 would not accept it.
pub fn typed<T: DeserializeOwned>(input: Map<String, Value>) -> Result<T, Failure> {
    serde_json::from_value(Value::Object(input)).map_err(|error| Failure::Usage {
        message: format!("The device settings are not complete: {error}."),
        hint: Some("anima device create --help".to_owned()),
    })
}

fn put<T: Serialize>(map: &mut Map<String, Value>, key: &str, value: Option<&T>) {
    if let Some(value) = value {
        map.insert(key.to_owned(), json!(value));
    }
}

/// The group with `enabled` (on unless said otherwise), or none when no flag of it was given.
/// `needs` is a field K1 requires and the flag that sets it.
fn complete(
    mut group: Map<String, Value>,
    enabled: Option<bool>,
    needs: Option<(&str, &str)>,
) -> Result<Option<Value>, Failure> {
    if group.is_empty() && enabled.is_none() {
        return Ok(None);
    }
    if let Some((field, flag)) = needs.filter(|(field, _)| !group.contains_key(*field)) {
        return Err(Failure::Usage {
            message: format!(
                "{flag} is needed: the hub replaces the whole group, so {field} is always sent."
            ),
            hint: None,
        });
    }
    group.insert("enabled".to_owned(), json!(enabled.unwrap_or(true)));
    Ok(Some(Value::Object(group)))
}
