//! The device settings that `device create` and `device update` take as flags (CMD-020). Each K1
//! input field is a flag with its K1 name in kebab-case. The flags become the K1 input as JSON,
//! and the generated input type of the document reads it back, so K1 still decides what is valid.
//!
//! A group (`--ssh-*`, `--rdp-*`, `--vnc-*`, `--transfer-*`) is sent whole, because the hub
//! replaces the group. A new device has no current values, so its SSH and RDP groups need
//! `--ssh-username` or `--rdp-username`. `device update` reads the current values first (never a
//! credential) and lays the flags over them.

use clap::ValueEnum;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use crate::failure::{Failure, Hint};

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

/// The values of a device's groups now, under their K1 input names; empty for a new device.
#[derive(Debug, Default)]
pub struct Current {
    ssh: Map<String, Value>,
    rdp: Map<String, Value>,
    vnc: Map<String, Value>,
    transfer: Map<String, Value>,
}

impl Current {
    /// The groups of `device`, an answer that selects only input fields of each group.
    #[must_use]
    pub fn of<T: Serialize>(device: &T) -> Self {
        let value = serde_json::to_value(device).unwrap_or(Value::Null);
        let group = |key: &str| match value.get(key) {
            Some(Value::Object(map)) => map.clone(),
            _ => Map::new(),
        };
        Self {
            ssh: group("ssh"),
            rdp: group("rdp"),
            vnc: group("vnc"),
            transfer: group("transfer"),
        }
    }
}

impl SettingsFlags {
    /// Whether any flag changes a group, so that the current values are needed.
    #[must_use]
    pub fn touches_groups(&self) -> bool {
        [
            self.ssh_enabled.is_some(),
            self.ssh_port.is_some(),
            self.ssh_username.is_some(),
            self.ssh_auth.is_some(),
            self.rdp_enabled.is_some(),
            self.rdp_port.is_some(),
            self.rdp_username.is_some(),
            self.rdp_domain.is_some(),
            self.rdp_security.is_some(),
            self.rdp_ignore_certificate.is_some(),
            self.rdp_resize_method.is_some(),
            self.rdp_audio_out.is_some(),
            self.rdp_microphone.is_some(),
            self.rdp_shared_drive.is_some(),
            self.vnc_enabled.is_some(),
            self.vnc_port.is_some(),
            self.vnc_username.is_some(),
            self.transfer_upload.is_some(),
            self.transfer_download.is_some(),
            self.transfer_copy.is_some(),
            self.transfer_paste.is_some(),
        ]
        .contains(&true)
    }

    /// The K1 input fields these flags set, with each changed group laid over its `current`
    /// values; a field with no flag is left out.
    ///
    /// # Errors
    /// [`Failure::Usage`] when a group lacks the user name K1 requires.
    pub fn fields(&self, current: &Current) -> Result<Map<String, Value>, Failure> {
        let mut input = Map::new();
        put(&mut input, "kind", self.kind.as_ref());
        put(&mut input, "os", self.os.as_ref());
        put(&mut input, "macAddress", self.mac_address.as_ref());
        put(&mut input, "metricsSource", self.metrics_source.as_ref());
        put(&mut input, "sessionLimit", self.session_limit.as_ref());
        if !self.tags.is_empty() {
            input.insert("tags".to_owned(), json!(self.tags));
        }
        put(&mut input, "ssh", self.ssh(&current.ssh)?.as_ref());
        put(&mut input, "rdp", self.rdp(&current.rdp)?.as_ref());
        put(&mut input, "vnc", self.vnc(&current.vnc)?.as_ref());
        put(
            &mut input,
            "transfer",
            self.transfer(&current.transfer).as_ref(),
        );
        Ok(input)
    }

    fn ssh(&self, current: &Map<String, Value>) -> Result<Option<Value>, Failure> {
        let mut group = Map::new();
        put(&mut group, "port", self.ssh_port.as_ref());
        put(&mut group, "username", self.ssh_username.as_ref());
        put(&mut group, "auth", self.ssh_auth.as_ref());
        let group = complete(
            group,
            self.ssh_enabled,
            current,
            Some(("username", "--ssh-username")),
        )?;
        Ok(group.map(|mut group| {
            group
                .entry("auth")
                .or_insert_with(|| json!(SshAuthArg::Password));
            Value::Object(group)
        }))
    }

    fn rdp(&self, current: &Map<String, Value>) -> Result<Option<Value>, Failure> {
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
        let needs = Some(("username", "--rdp-username"));
        Ok(complete(group, self.rdp_enabled, current, needs)?.map(Value::Object))
    }

    fn vnc(&self, current: &Map<String, Value>) -> Result<Option<Value>, Failure> {
        let mut group = Map::new();
        put(&mut group, "port", self.vnc_port.as_ref());
        put(&mut group, "username", self.vnc_username.as_ref());
        Ok(complete(group, self.vnc_enabled, current, None)?.map(Value::Object))
    }

    fn transfer(&self, current: &Map<String, Value>) -> Option<Value> {
        let mut group = Map::new();
        put(&mut group, "upload", self.transfer_upload.as_ref());
        put(&mut group, "download", self.transfer_download.as_ref());
        put(&mut group, "copy", self.transfer_copy.as_ref());
        put(&mut group, "paste", self.transfer_paste.as_ref());
        (!group.is_empty()).then(|| {
            let mut merged = current.clone();
            merged.extend(group);
            Value::Object(merged)
        })
    }
}

/// `input` as the K1 input type `T` of a document.
///
/// # Errors
/// [`Failure::Usage`] when K1 would not accept it.
pub fn typed<T: DeserializeOwned>(input: Map<String, Value>) -> Result<T, Failure> {
    serde_json::from_value(Value::Object(input)).map_err(|error| Failure::Usage {
        message: format!("The device settings are not complete: {error}."),
        hint: Some(Hint::run("anima device create --help")),
    })
}

fn put<T: Serialize>(map: &mut Map<String, Value>, key: &str, value: Option<&T>) {
    if let Some(value) = value {
        map.insert(key.to_owned(), json!(value));
    }
}

/// The group the flags change, laid over `current`, with `enabled` (kept, else on); none when
/// no flag of it was given. `needs` is a field K1 requires and the flag that sets it.
fn complete(
    flags: Map<String, Value>,
    enabled: Option<bool>,
    current: &Map<String, Value>,
    needs: Option<(&str, &str)>,
) -> Result<Option<Map<String, Value>>, Failure> {
    if flags.is_empty() && enabled.is_none() {
        return Ok(None);
    }
    let mut group = current.clone();
    group.extend(flags);
    if let Some((field, flag)) = needs.filter(|(field, _)| !group.contains_key(*field)) {
        return Err(Failure::Usage {
            message: format!(
                "{flag} is needed: the hub replaces the whole group, so {field} is always sent."
            ),
            hint: None,
        });
    }
    if let Some(enabled) = enabled {
        group.insert("enabled".to_owned(), json!(enabled));
    } else {
        group.entry("enabled").or_insert_with(|| json!(true));
    }
    Ok(Some(group))
}
