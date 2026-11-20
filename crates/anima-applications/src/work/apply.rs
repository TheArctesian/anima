//! `application_apply` (contract K10 Section 6.4.6): installs or updates one application that
//! Anima installed. The check has three parts, because the program gets the two files and
//! pulls the images between them: [`accepted`] before the files, [`prepared`] with the files,
//! and [`planned`] with the digests of the pulls.

use std::fmt::Write;

use compose_rules::{Checked, Node, Rule};
use sha2::{Digest, Sha256};

use super::{Facts, free, managed};
use crate::{
    Apply, Command, FinalForm, Folders, Frame, Plan, Pulled, Refusal, Step,
    final_form::{self, image_by_digest},
    forms::project_of,
    inventory::project_is_protected,
    json::{self, Json},
    plan::Project,
};

/// The two files of an item, as the daemon gave them. `{:?}` shows only their lengths.
#[derive(Clone, Copy)]
pub struct Files<'a> {
    pub compose: &'a [u8],
    /// `None` when `environmentSha256` of the item is null.
    pub environment: Option<&'a [u8]>,
}

impl std::fmt::Debug for Files<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Files")
            .field("compose_bytes", &self.compose.len())
            .field("environment_bytes", &self.environment.map(<[u8]>::len))
            .finish()
    }
}

/// The pull of the image of one service, and the read of the digest that the pull gave.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pull {
    pub service: String,
    pub image: String,
    pub pull: Command,
    /// It prints the names by which the registry gave the image, each with its digest.
    pub inspect: Command,
}

/// One version of an application whose files passed each rule: what [`planned`] starts.
/// `{:?}` shows no byte of the two files and no value of the checked file.
#[derive(Clone, PartialEq, Eq)]
pub struct Prepared {
    pub(crate) application: String,
    pub(crate) keep: bool,
    pub(crate) checked: Checked,
    pub(crate) compose: Vec<u8>,
    pub(crate) environment: Vec<u8>,
    /// The digests that the program has before a pull: those of the last apply when `pull`
    /// is false, those of the kept version for `application_back`.
    pub(crate) known: Vec<Pulled>,
    /// The pulls that the caller does before [`planned`]; none when `pull` is false.
    pub pulls: Vec<Pull>,
}

impl std::fmt::Debug for Prepared {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Prepared")
            .field("application", &self.application)
            .field("keep", &self.keep)
            .field("compose_bytes", &self.compose.len())
            .field("environment_bytes", &self.environment.len())
            .field("known", &self.known)
            .field("pulls", &self.pulls)
            .finish_non_exhaustive()
    }
}

/// The plan of one version, and what the result and the log of the machine say of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Planned {
    pub plan: Plan,
    /// The SHA-256 of the final form, for the log of the machine (Section 9).
    pub final_form_sha256: String,
    /// True when the project has a volume or a bind mount.
    pub holds_volumes: bool,
}

/// The SHA-256 of `bytes` in 64 lower-case hexadecimal characters.
pub(crate) fn sha256(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        // A write to a `String` does not fail.
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// The check of `application_apply` before the program has the two files.
///
/// # Errors
/// `applications_off`; `protected` when the list "Protected" has the project or one of its
/// containers; `name_taken` when a project of the name exists with a different folder
/// (SF-028); `busy`.
pub fn accepted(apply: &Apply, facts: &Facts<'_>) -> Result<(), Refusal> {
    project_checked(&apply.application, facts)
}

/// The checks of the project of `application` that an apply and a back do before the files
/// and again before the plan, with the facts of that moment.
///
/// # Errors
/// As [`accepted`].
pub(crate) fn project_checked(application: &str, facts: &Facts<'_>) -> Result<(), Refusal> {
    managed(facts)?;
    let project = project_of(application);
    if project_is_protected(&project, facts.settings, facts.inventory) {
        return Err(Refusal::Protected);
    }
    let folder = facts.paths.application(application).files();
    let of_the_project = |container: &&crate::Container| container.is_of(&project);
    if facts
        .inventory
        .containers
        .iter()
        .filter(of_the_project)
        .any(|container| container.folder.as_deref() != Some(folder.as_str()))
    {
        return Err(Refusal::NameTaken);
    }
    free(facts)
}

/// The names and the images of the services of a checked file, in the order of the file.
fn images(checked: &Checked) -> Vec<(&str, &str)> {
    let services = checked.document.get("services").and_then(Node::entries);
    services
        .unwrap_or(&[])
        .iter()
        .filter_map(|(name, node)| Some((name.as_str(), node.get("image")?.text()?)))
        .collect()
}

fn pulls(checked: &Checked, facts: &Facts<'_>) -> Vec<Pull> {
    let format = "{{json .RepoDigests}}";
    images(checked)
        .into_iter()
        .map(|(service, image)| Pull {
            service: service.to_owned(),
            image: image.to_owned(),
            pull: facts.paths.docker(&["image", "pull"], &[image]),
            inspect: facts
                .paths
                .docker(&["image", "inspect", "--format", format], &[image]),
        })
        .collect()
}

/// The final form of `prepared` with `digests`.
pub(crate) fn final_form_of(
    prepared: &Prepared,
    digests: &[Pulled],
    facts: &Facts<'_>,
) -> Result<FinalForm, Refusal> {
    let project = project_of(&prepared.application);
    let binds = facts.paths.application(&prepared.application).binds();
    let own = facts.settings.limits.own(&facts.machine);
    let frame = Frame {
        project: &project,
        binds: &binds,
        own: &own,
        open_to_network: facts.settings.open_to_network,
        digests,
    };
    final_form::formed(&prepared.checked, &frame)
}

/// The check of the two files of `application_apply` (Sections 6.4.4 and 6.4.6).
///
/// # Errors
/// Each refusal of [`accepted`]; `program_mismatch` for a file whose SHA-256 is different
/// from the one of the item, and for an environment file that the item does not name;
/// `file_not_permitted` and `not_at_machine` for a file that breaks a rule of Section 6.4.4;
/// `limit_disk` when a pull is asked and the free disk is below the limit of the machine;
/// `no_digest` when `pull` is false and a service has no digest of the last apply.
pub fn prepared(apply: &Apply, files: &Files<'_>, facts: &Facts<'_>) -> Result<Prepared, Refusal> {
    accepted(apply, facts)?;
    let named = apply.environment_sha256.as_deref();
    let environment_sound = match (files.environment, named) {
        (None, None) => true,
        (Some(bytes), Some(hash)) => sha256(bytes) == hash,
        _ => false,
    };
    if sha256(files.compose) != apply.file_sha256 || !environment_sound {
        return Err(Refusal::ProgramMismatch);
    }
    let environment = files.environment.unwrap_or_default();
    let compose =
        std::str::from_utf8(files.compose).map_err(|_| Refusal::file(Rule::YamlNotPermitted))?;
    let variables =
        std::str::from_utf8(environment).map_err(|_| Refusal::file(Rule::VariableNotPermitted))?;
    let checked = compose_rules::check(compose, variables, &facts.settings.of_a_file())?;
    if apply.pull && facts.machine.free_disk_bytes < facts.settings.limits.free_disk_bytes {
        return Err(Refusal::LimitDisk);
    }
    let installed = facts.records.installed(&apply.application);
    let mut prepared = Prepared {
        application: apply.application.clone(),
        keep: apply.keep,
        pulls: Vec::new(),
        known: Vec::new(),
        compose: files.compose.to_vec(),
        environment: environment.to_vec(),
        checked,
    };
    if apply.pull {
        prepared.pulls = pulls(&prepared.checked, facts);
    } else {
        prepared.known = installed.map_or_default(|installed| installed.current.clone());
        final_form_of(&prepared, &prepared.known, facts)?;
    }
    Ok(prepared)
}

fn digests_written(digests: &[Pulled]) -> Vec<u8> {
    let one = |pulled: &Pulled| {
        Json::Record(vec![
            ("service".to_owned(), Json::text(&pulled.service)),
            ("image".to_owned(), Json::text(&pulled.image)),
            ("digest".to_owned(), Json::text(&pulled.digest)),
        ])
    };
    json::written(&Json::List(digests.iter().map(one).collect())).into_bytes()
}

/// The steps that write one version and start it: the Compose file, the environment file and
/// the digests under their staged names, the final form, the check that `files/` holds no
/// `.env`, the check of each folder of `binds/` directly before the start (SF-016), and the
/// start from the final form only (SF-017). The records of the version are moved in place
/// only after the start, by [`in_place_steps`].
///
/// The final form is written in place before the start, because it is the file that Compose
/// reads. After a failed start it is the final form of the new version, and the records are
/// those of the version before it.
pub(crate) fn version_steps(
    prepared: &Prepared,
    digests: &[Pulled],
    facts: &Facts<'_>,
) -> Result<(Vec<Step>, FinalForm), Refusal> {
    let form = final_form_of(prepared, digests, facts)?;
    let folders = facts.paths.application(&prepared.application);
    let write = |path: String, content: &[u8]| Step::Write {
        path,
        content: content.to_vec(),
    };
    let [compose, environment, digests_file] = folders.staged();
    let mut steps = vec![
        write(compose, &prepared.compose),
        write(environment, &prepared.environment),
        write(folders.final_form(), form.text.as_bytes()),
        write(digests_file, &digests_written(digests)),
        Step::Absent {
            path: folders.dot_env(),
        },
        Step::RealFolder {
            path: folders.binds(),
            below: folders.application().to_owned(),
        },
    ];
    steps.extend(form.binds.iter().map(|name| Step::RealFolder {
        path: format!("{}/{name}", folders.binds()),
        below: folders.binds(),
    }));
    let project = project_of(&prepared.application);
    let files = [folders.final_form()];
    let named = Project {
        name: &project,
        folder: &folders.files(),
        files: &files,
    };
    let up = [
        "up",
        "-d",
        "--remove-orphans",
        "--pull",
        "never",
        "--no-build",
    ];
    steps.push(Step::Run(facts.paths.compose(&named, &up)));
    Ok((steps, form))
}

/// The steps after a start that did not fail: the staged records of the new version are moved
/// in place of the records of the current version.
pub(crate) fn in_place_steps(folders: &Folders) -> Vec<Step> {
    folders
        .staged()
        .into_iter()
        .zip(folders.current())
        .map(|(from, to)| Step::Move { from, to })
        .collect()
}

/// The folders of an application and of each bind mount, which the program makes itself.
pub(crate) fn folder_steps(folders: &Folders, binds: &[String]) -> Vec<Step> {
    let mut paths = vec![
        folders.application().to_owned(),
        folders.files(),
        folders.binds(),
    ];
    paths.extend(
        binds
            .iter()
            .map(|name| format!("{}/{name}", folders.binds())),
    );
    paths
        .into_iter()
        .map(|path| Step::Folder { path })
        .collect()
}

/// Each image that the program pulled for the application, with those of `digests`.
fn pulled_written(prepared: &Prepared, digests: &[Pulled], facts: &Facts<'_>) -> Vec<u8> {
    let installed = facts.records.installed(&prepared.application);
    let mut pulled = installed.map_or_default(|installed| installed.pulled.clone());
    for one in digests {
        let image = image_by_digest(&one.image, &one.digest);
        if !pulled.contains(&image) {
            pulled.push(image);
        }
    }
    json::written(&Json::List(
        pulled.iter().map(|image| Json::text(image)).collect(),
    ))
    .into_bytes()
}

/// The plan of `application_apply`, with `pulled`, the digests of the pulls of `prepared`.
/// `facts` are those of the moment after the pulls: the checks of [`accepted`] are done
/// again with them.
///
/// # Errors
/// Each refusal of [`accepted`]; `no_digest` for a service that has no digest;
/// `file_not_permitted` for a value that the final form cannot hold.
pub fn planned(
    prepared: &Prepared,
    pulled: &[Pulled],
    facts: &Facts<'_>,
) -> Result<Planned, Refusal> {
    project_checked(&prepared.application, facts)?;
    let digests = if prepared.pulls.is_empty() {
        &prepared.known
    } else {
        pulled
    };
    let (version, form) = version_steps(prepared, digests, facts)?;
    let folders = facts.paths.application(&prepared.application);
    let mut steps = folder_steps(&folders, &form.binds);
    // The images are on the machine after the pulls, whether the start fails or not.
    steps.push(Step::Write {
        path: folders.pulled(),
        content: pulled_written(prepared, digests, facts),
    });
    steps.extend(version);
    let has_files = facts
        .records
        .installed(&prepared.application)
        .is_some_and(|installed| installed.has_files);
    // After the start only: a failed start never replaces the last kept version.
    for (from, to) in folders.current().into_iter().zip(folders.kept()) {
        // `keep` 0 keeps no version, so an `application_back` after it has nothing.
        steps.push(match (prepared.keep, has_files) {
            (true, true) => Step::Move { from, to },
            _ => Step::Delete { path: to },
        });
    }
    steps.extend(in_place_steps(&folders));
    Ok(Planned {
        plan: Plan { steps },
        final_form_sha256: sha256(form.text.as_bytes()),
        holds_volumes: form.holds_volumes,
    })
}
