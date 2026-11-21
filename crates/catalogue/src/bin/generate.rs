//! The generator of the recipes of LinuxServer.io (task H39). It reads a saved fleet list and a
//! file of digests that were read from the registry, writes the recipe of each image that has a
//! digest, and prints the report of the whole fleet. It reaches no network.
//!
//! `generate <fleet.json> <pins.json> <entries folder>`
//!
//! It writes nothing when one recipe that exists would change in more than its digest or its
//! version, or was written by a person (SF-NEW-lsgenerator), and ends with a failure.

use std::{collections::BTreeMap, env, fmt::Write as _, fs, io, path::Path, process::ExitCode};

use catalogue::{
    Recipe,
    linuxserver::{self, Assessment, Pin, Unfit, assess, list, recipe, update},
    read,
};
use serde::Deserialize;

/// A digest that no registry gives: the report uses it for an image with no pin.
const NO_DIGEST: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// One line of the file of digests.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Read {
    tag: String,
    digest: String,
    resolved: String,
}

#[derive(Debug, thiserror::Error)]
enum GenerateError {
    #[error("usage: generate <fleet.json> <pins.json> <entries folder>")]
    Usage,
    #[error("a file cannot be read or written")]
    File(#[from] io::Error),
    #[error(transparent)]
    Fleet(#[from] linuxserver::FleetError),
    #[error("the file of digests cannot be read")]
    Pins(#[from] serde_json::Error),
    #[error("the folder of the catalogue cannot be read")]
    Folder(#[from] catalogue::ReadError),
    #[error("{0} recipes were not written; nothing was written")]
    Refused(usize),
}

fn write(root: &Path, made: &Recipe) -> io::Result<()> {
    let folder = root.join(&made.folder);
    fs::create_dir_all(&folder)?;
    for (name, text) in [
        ("entry.json", &made.entry),
        ("compose.yaml", &made.compose),
        ("environment.example", &made.environment),
    ] {
        fs::write(folder.join(name), text.as_deref().unwrap_or_default())?;
    }
    Ok(())
}

/// The report of the fleet in Markdown: the count of each place, and the images of each reason.
fn report(places: &[(String, Assessment)]) -> String {
    let mut groups: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for (name, place) in places {
        let group = match place {
            Assessment::Fits => "fits".to_owned(),
            Assessment::FitsWithPortMoved => "fits with a port moved".to_owned(),
            Assessment::FitsWithPartOff => "fits with an optional part off".to_owned(),
            Assessment::Unfit(Unfit::Capability(capability)) => format!("capability {capability}"),
            Assessment::Unfit(reason) => reason.code().to_owned(),
        };
        groups.entry(group).or_default().push(name);
    }
    let mut text = format!(
        "{} images\n\n| Place | Count | Images |\n| --- | --- | --- |\n",
        places.len()
    );
    for (group, names) in groups {
        writeln!(text, "| {group} | {} | {} |", names.len(), names.join(", "))
            .expect("a String takes text");
    }
    text
}

fn run(arguments: &[String]) -> Result<String, GenerateError> {
    let [_, fleet, pins, root] = arguments else {
        return Err(GenerateError::Usage);
    };
    let fleet = list(&fs::read_to_string(fleet)?)?;
    let pins: BTreeMap<String, Read> = serde_json::from_str(&fs::read_to_string(pins)?)?;
    let root = Path::new(root);
    let existing = read(root)?;
    let mut writes = Vec::new();
    let mut refused = 0;
    let mut places = Vec::new();
    for image in fleet.images.iter().filter(|image| !image.deprecated) {
        let pinned = pins.get(&image.name);
        let pin = Pin {
            tag: pinned.map_or(image.version.clone(), |read| read.tag.clone()),
            digest: pinned.map_or(NO_DIGEST.to_owned(), |read| read.digest.clone()),
            resolved: pinned.map_or(fleet.read.clone(), |read| read.resolved.clone()),
            read: fleet.read.clone(),
        };
        places.push((image.name.clone(), assess(image, &pin)));
        if pinned.is_none() {
            continue;
        }
        match recipe(image, &pin) {
            Err(reason) => eprintln!("{}: not written ({reason})", image.name),
            Ok(made) => match existing.iter().find(|recipe| recipe.folder == made.folder) {
                None => writes.push(made),
                Some(old) => match update(old, &made) {
                    Ok(()) => writes.push(made),
                    Err(change) => {
                        eprintln!("{}: not written ({change})", image.name);
                        refused += 1;
                    }
                },
            },
        }
    }
    if refused > 0 {
        return Err(GenerateError::Refused(refused));
    }
    for made in &writes {
        write(root, made)?;
    }
    Ok(report(&places))
}

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().collect();
    match run(&arguments) {
        Ok(report) => {
            print!("{report}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("generate: {error}");
            ExitCode::FAILURE
        }
    }
}
