//! The reader of the settings file: strict, so that a member that this program does not know
//! is never ignored (contract K10 Section 6.4.2).

use super::{Adopted, EngineKind, Limits, Managed, Settings};
use crate::{
    forms::{is_engine_name, is_path, is_project_name},
    strict::{self, Members, Value},
};

/// The most bytes of a settings file.
pub const LARGEST_SETTINGS: usize = 1 << 20;
/// The most entries of one list of the settings, and the most files of one adopted project.
const MOST_ENTRIES: usize = 256;
const MOST_FILES: usize = 16;
/// The one version of the form of the file.
const VERSION: u64 = 1;

/// Why a settings file is not read. No variant holds text of the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SettingsFault {
    #[error("the settings file is too large")]
    TooLarge,
    #[error("the settings file is not strict JSON")]
    NotJson,
    #[error("the settings file has a member that this program does not know")]
    MemberUnknown,
    #[error("the settings file lacks a member or has a value that is not permitted")]
    ValueNotPermitted,
}

const WRONG: SettingsFault = SettingsFault::ValueNotPermitted;

fn finished(members: &Members) -> Result<(), SettingsFault> {
    if members.is_empty() {
        Ok(())
    } else {
        Err(SettingsFault::MemberUnknown)
    }
}

/// A whole number above zero, or `null` where the value of the machine is permitted.
fn limit(members: &mut Members, name: &str, machine: bool) -> Result<Option<u64>, SettingsFault> {
    match members.take(name) {
        Some(Value::Whole(whole)) if whole > 0 => Ok(Some(whole)),
        Some(Value::Null) if machine => Ok(None),
        _ => Err(WRONG),
    }
}

fn limits(value: Value) -> Result<Limits, SettingsFault> {
    let mut members = Members::of(value).ok_or(WRONG)?;
    let limits = Limits {
        memory_bytes: limit(&mut members, "memoryBytes", true)?,
        processes: limit(&mut members, "processes", false)?.ok_or(WRONG)?,
        processor_billionths: limit(&mut members, "processorBillionths", true)?,
        stop_seconds: limit(&mut members, "stopSeconds", false)?.ok_or(WRONG)?,
        free_disk_bytes: limit(&mut members, "freeDiskBytes", false)?.ok_or(WRONG)?,
    };
    finished(&members)?;
    Ok(limits)
}

fn path(members: &mut Members, name: &str) -> Result<String, SettingsFault> {
    members.text(name).filter(|path| is_path(path)).ok_or(WRONG)
}

fn managed(value: Value) -> Result<Option<Managed>, SettingsFault> {
    if value == Value::Null {
        return Ok(None);
    }
    let mut members = Members::of(value).ok_or(WRONG)?;
    let engine = match members.text("engine").as_deref() {
        Some("system") => EngineKind::System,
        Some("user") => EngineKind::User,
        _ => return Err(WRONG),
    };
    let managed = Managed {
        engine,
        socket: path(&mut members, "socket")?,
        docker: path(&mut members, "docker")?,
        compose: path(&mut members, "compose")?,
        accepted_user: members
            .whole("acceptedUser")
            .and_then(|user| u32::try_from(user).ok())
            .ok_or(WRONG)?,
        set_on_at: members.whole("setOnAt").ok_or(WRONG)?,
    };
    finished(&members)?;
    Ok(Some(managed))
}

/// One adopted project. A project whose name starts with `anima-` cannot be adopted
/// (Section 6.4.3).
fn adopted(value: Value) -> Result<Adopted, SettingsFault> {
    let mut members = Members::of(value).ok_or(WRONG)?;
    let name = members
        .text("name")
        .filter(|name| is_project_name(name) && !name.starts_with("anima-"))
        .ok_or(WRONG)?;
    let folder = path(&mut members, "folder")?;
    let files = members.texts("files").ok_or(WRONG)?;
    if files.is_empty() || files.len() > MOST_FILES || !files.iter().all(|file| is_path(file)) {
        return Err(WRONG);
    }
    finished(&members)?;
    Ok(Adopted {
        name,
        folder,
        files,
    })
}

/// A list of texts of one form, each there one time.
fn list(
    members: &mut Members,
    name: &str,
    sound: fn(&str) -> bool,
) -> Result<Vec<String>, SettingsFault> {
    let texts = members.texts(name).ok_or(WRONG)?;
    let repeated = texts
        .iter()
        .enumerate()
        .any(|(at, text)| texts[..at].contains(text));
    if repeated || texts.len() > MOST_ENTRIES || !texts.iter().all(|text| sound(text)) {
        return Err(WRONG);
    }
    Ok(texts)
}

/// A registry: a host in lower case, with or without a port.
fn is_registry(text: &str) -> bool {
    let sound =
        |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-:".contains(&byte);
    (1..=255).contains(&text.len())
        && text.bytes().all(sound)
        && text
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
}

/// A prefix of labels: it is never a prefix of the labels of Compose itself (SF-043).
fn is_label_prefix(text: &str) -> bool {
    (1..=255).contains(&text.len())
        && text
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b'=')
        && !text.starts_with("com.docker.")
        && !"com.docker.".starts_with(text)
}

fn truth(members: &mut Members, name: &str) -> Result<bool, SettingsFault> {
    members.truth(name).ok_or(WRONG)
}

/// Reads the text of a settings file.
///
/// # Errors
/// The [`SettingsFault`], when the text is too large or not strict JSON, has a member that
/// this program does not know, lacks a member, or has a value that is not permitted. The
/// caller then treats "Manage applications" as off.
pub fn read(text: &str) -> Result<Settings, SettingsFault> {
    if text.len() > LARGEST_SETTINGS {
        return Err(SettingsFault::TooLarge);
    }
    let value = strict::read(text).ok_or(SettingsFault::NotJson)?;
    let mut members = Members::of(value).ok_or(WRONG)?;
    if members.whole("version") != Some(VERSION) {
        return Err(WRONG);
    }
    let Some(Value::List(projects)) = members.take("adoptedProjects") else {
        return Err(WRONG);
    };
    let projects: Vec<Adopted> = projects
        .into_iter()
        .map(adopted)
        .collect::<Result<_, _>>()?;
    let repeated = projects.iter().enumerate().any(|(at, project)| {
        projects[..at]
            .iter()
            .any(|other| other.name == project.name)
    });
    if repeated || projects.len() > MOST_ENTRIES {
        return Err(WRONG);
    }
    let settings = Settings {
        managed: managed(members.take("manageApplications").ok_or(WRONG)?)?,
        all_containers: truth(&mut members, "allContainers")?,
        open_to_network: truth(&mut members, "openToNetwork")?,
        hub_may_delete_volumes: truth(&mut members, "hubMayDeleteVolumes")?,
        adopted: projects,
        protected: list(&mut members, "protected", is_engine_name)?,
        registries: list(&mut members, "registriesWithLogin", is_registry)?,
        limits: limits(members.take("limits").ok_or(WRONG)?)?,
        label_prefixes: list(&mut members, "labelPrefixes", is_label_prefix)?,
    };
    finished(&members)?;
    Ok(settings)
}
