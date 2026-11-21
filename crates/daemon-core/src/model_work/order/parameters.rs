//! The `parameters` object of each model work type (Section 2): exactly the members of its row.
//! Identifiers are UUIDs; a runtime, a backend, a format or a probe is a word of a fixed list.

use serde_json::{Map, Value};
use uuid::Uuid;

use super::ModelMachine;
use crate::model_work::cause::Cause;
use crate::model_work::files::{self, MAX_FILES};
use crate::model_work::names::{
    Backend, Format, Probe, Runtime, Word, is_lower_hex, is_source_name,
};
use crate::model_work::releases::Release;
use crate::model_work::settings::{self, ServerSettings};
use crate::wire::WorkType;

type Object = Map<String, Value>;

/// The source of a pull: an organisation, a repository and a commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullSource {
    pub organisation: String,
    pub repository: String,
    pub revision: String,
}

/// One file of a pull.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullFile {
    pub name: String,
    pub size_bytes: u64,
    pub sha256: String,
}

/// The typed `parameters` of each type.
#[derive(Clone, Debug, PartialEq)]
pub enum Parameters {
    Report,
    Install(Release),
    Pull {
        copy: Uuid,
        variant: Uuid,
        source: PullSource,
        format: Format,
        files: Vec<PullFile>,
        gated: bool,
    },
    CopyDelete {
        copy: Uuid,
    },
    Start {
        server: Uuid,
        release: Release,
        settings: ServerSettings,
    },
    Stop {
        server: Uuid,
    },
    Load {
        server: Uuid,
        copy: Uuid,
        settings: ServerSettings,
    },
    Unload {
        server: Uuid,
        copy: Uuid,
    },
    Measure {
        server: Uuid,
        copy: Uuid,
        probe: Probe,
    },
    Drain {
        seconds: u32,
    },
}

fn exactly(object: &Object, names: &[&str]) -> Result<(), Cause> {
    let holds = object.len() == names.len() && names.iter().all(|name| object.contains_key(*name));
    holds.then_some(()).ok_or(Cause::MemberUnknown)
}

fn uuid(object: &Object, name: &str) -> Result<Uuid, Cause> {
    object
        .get(name)
        .and_then(Value::as_str)
        .and_then(|text| Uuid::parse_str(text).ok())
        .ok_or(Cause::MemberUnknown)
}

fn word<T: Word>(object: &Object, name: &str) -> Result<T, Cause> {
    object
        .get(name)
        .and_then(Value::as_str)
        .and_then(T::parse)
        .ok_or(Cause::MemberUnknown)
}

fn release(object: &Object) -> Result<Release, Cause> {
    let runtime: Runtime = word(object, "runtime")?;
    let version = object
        .get("version")
        .and_then(Value::as_str)
        .filter(|version| Release::number(runtime, version).is_some())
        .ok_or(Cause::MemberUnknown)?;
    Ok(Release {
        runtime,
        version: version.to_owned(),
        backend: word::<Backend>(object, "backend")?,
    })
}

fn settings_of(
    object: &Object,
    runtime: Option<Runtime>,
    machine: &ModelMachine<'_>,
) -> Result<ServerSettings, Cause> {
    let settings = object
        .get("settings")
        .and_then(Value::as_object)
        .ok_or(Cause::MemberUnknown)?;
    settings::read(settings, runtime, machine.limits)
}

fn source(object: &Object, machine: &ModelMachine<'_>) -> Result<PullSource, Cause> {
    let source = object
        .get("source")
        .and_then(Value::as_object)
        .ok_or(Cause::MemberUnknown)?;
    exactly(source, &["organisation", "repository", "revision"])?;
    let text = |name| {
        source
            .get(name)
            .and_then(Value::as_str)
            .ok_or(Cause::MemberUnknown)
    };
    let (organisation, repository) = (text("organisation")?, text("repository")?);
    let revision = text("revision")?;
    if !is_source_name(organisation) || !is_source_name(repository) || !is_lower_hex(revision, 40) {
        return Err(Cause::MemberUnknown);
    }
    if !machine.settings.organisation_permitted(organisation) {
        return Err(Cause::SourceNotPermitted);
    }
    Ok(PullSource {
        organisation: organisation.to_owned(),
        repository: repository.to_owned(),
        revision: revision.to_owned(),
    })
}

fn pull_file(value: &Value, format: Format) -> Result<PullFile, Cause> {
    let file = value.as_object().ok_or(Cause::MemberUnknown)?;
    exactly(file, &["name", "sizeBytes", "sha256"])?;
    let name = file
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| files::is_file_path(name))
        .ok_or(Cause::MemberUnknown)?;
    let size_bytes = file
        .get("sizeBytes")
        .and_then(Value::as_u64)
        .filter(|size| *size > 0)
        .ok_or(Cause::MemberUnknown)?;
    let sha256 = file
        .get("sha256")
        .and_then(Value::as_str)
        .filter(|sha| is_lower_hex(sha, 64))
        .ok_or(Cause::MemberUnknown)?;
    if !files::permitted(format, name) {
        return Err(Cause::FileType);
    }
    Ok(PullFile {
        name: name.to_owned(),
        size_bytes,
        sha256: sha256.to_owned(),
    })
}

fn pull(object: &Object, machine: &ModelMachine<'_>) -> Result<Parameters, Cause> {
    exactly(
        object,
        &["copy", "variant", "source", "format", "files", "gated"],
    )?;
    let format: Format = word(object, "format")?;
    let list = object
        .get("files")
        .and_then(Value::as_array)
        .filter(|list| (1..=MAX_FILES).contains(&list.len()))
        .ok_or(Cause::MemberUnknown)?;
    let files = list
        .iter()
        .map(|file| pull_file(file, format))
        .collect::<Result<Vec<_>, _>>()?;
    let mut names: Vec<String> = files
        .iter()
        .map(|file| file.name.to_ascii_lowercase())
        .collect();
    names.sort_unstable();
    names.dedup();
    if names.len() != files.len() {
        return Err(Cause::MemberUnknown);
    }
    Ok(Parameters::Pull {
        copy: uuid(object, "copy")?,
        variant: uuid(object, "variant")?,
        source: source(object, machine)?,
        format,
        files,
        gated: object
            .get("gated")
            .and_then(Value::as_bool)
            .ok_or(Cause::MemberUnknown)?,
    })
}

fn server_start(object: &Object, machine: &ModelMachine<'_>) -> Result<Parameters, Cause> {
    exactly(
        object,
        &["server", "runtime", "version", "backend", "settings"],
    )?;
    let release = release(object)?;
    let settings = settings_of(object, Some(release.runtime), machine)?;
    Ok(Parameters::Start {
        server: uuid(object, "server")?,
        release,
        settings,
    })
}

/// Reads the `parameters` of `work_type`.
///
/// # Errors
/// The cause of the first member that does not hold.
pub fn read(
    work_type: WorkType,
    object: &Object,
    machine: &ModelMachine<'_>,
) -> Result<Parameters, Cause> {
    let pair = |a, b| -> Result<(Uuid, Uuid), Cause> {
        exactly(object, &[a, b])?;
        Ok((uuid(object, a)?, uuid(object, b)?))
    };
    match work_type {
        WorkType::ModelReport => exactly(object, &[]).map(|()| Parameters::Report),
        WorkType::RuntimeInstall => {
            exactly(object, &["runtime", "version", "backend"])?;
            release(object).map(Parameters::Install)
        }
        WorkType::ModelPull => pull(object, machine),
        WorkType::CopyDelete => {
            exactly(object, &["copy"])?;
            Ok(Parameters::CopyDelete {
                copy: uuid(object, "copy")?,
            })
        }
        WorkType::ServerStart => server_start(object, machine),
        WorkType::ServerStop => {
            exactly(object, &["server"])?;
            Ok(Parameters::Stop {
                server: uuid(object, "server")?,
            })
        }
        WorkType::ModelLoad => {
            exactly(object, &["server", "copy", "settings"])?;
            let settings = settings_of(object, None, machine)?;
            Ok(Parameters::Load {
                server: uuid(object, "server")?,
                copy: uuid(object, "copy")?,
                settings,
            })
        }
        WorkType::ModelUnload => {
            pair("server", "copy").map(|(server, copy)| Parameters::Unload { server, copy })
        }
        WorkType::ModelMeasure => {
            exactly(object, &["server", "copy", "probe"])?;
            let probe = word::<Probe>(object, "probe")?;
            Ok(Parameters::Measure {
                server: uuid(object, "server")?,
                copy: uuid(object, "copy")?,
                probe,
            })
        }
        WorkType::ModelDrain => {
            exactly(object, &["seconds"])?;
            let seconds = object
                .get("seconds")
                .and_then(Value::as_u64)
                .filter(|s| (1..=60).contains(s));
            let seconds = seconds
                .and_then(|s| u32::try_from(s).ok())
                .ok_or(Cause::MemberUnknown)?;
            Ok(Parameters::Drain { seconds })
        }
        WorkType::Build | WorkType::Runner | WorkType::Unknown => Err(Cause::TypeUnknown),
    }
}
