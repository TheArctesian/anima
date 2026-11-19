//! `anima location import <file>`: sends the samples of a file to the hub. The file is a JSON
//! list or JSON lines, each a sample in the Anima form (K1 `LocationSampleInput`) or an
//! OwnTracks message; an OwnTracks message that is no location is passed over. The hub stores
//! a sample one time, so the same file again adds nothing.

use std::path::PathBuf;

use chrono::{DateTime, SecondsFormat};
use serde_json::Value;

use super::documents::{LocationRecord, location_record};
use crate::failure::Failure;
use crate::invocation::Invocation;

type SampleInput = location_record::LocationSampleInput;

/// The most samples the hub takes in one call.
const BATCH: usize = 500;
/// Kilometres in an hour for each metre in a second.
const KMH_PER_MPS: f64 = 3.6;

#[derive(Debug, clap::Args)]
pub struct ImportArgs {
    /// The file of samples.
    file: PathBuf,
}

/// Why a file of samples cannot be read. Names the entry, never its text.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum FileError {
    #[error("entry {0} is not JSON")]
    NotJson(usize),
    #[error("entry {0} is not a sample: give key and, when known, observedAt and the position")]
    NotSample(usize),
    #[error("entry {0} is an OwnTracks location with no tst, lat or lon")]
    OwnTracks(usize),
}

fn entries(text: &str) -> Result<Vec<Value>, FileError> {
    if text.trim_start().starts_with('[') {
        return serde_json::from_str(text).map_err(|_| FileError::NotJson(1));
    }
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
        .map(|(index, line)| serde_json::from_str(line).map_err(|_| FileError::NotJson(index + 1)))
        .collect()
}

/// The sample of an OwnTracks location, with the key `<tid>:<tst>` as the hub makes it.
fn owntracks(number: usize, message: &Value) -> Result<SampleInput, FileError> {
    let (Some(tst), Some(latitude), Some(longitude)) = (
        message["tst"].as_i64(),
        message["lat"].as_f64(),
        message["lon"].as_f64(),
    ) else {
        return Err(FileError::OwnTracks(number));
    };
    let observed = DateTime::from_timestamp(tst, 0).ok_or(FileError::OwnTracks(number))?;
    Ok(SampleInput {
        key: format!("{}:{tst}", message["tid"].as_str().unwrap_or_default()),
        observed_at: Some(observed.to_rfc3339_opts(SecondsFormat::Secs, true)),
        latitude: Some(latitude),
        longitude: Some(longitude),
        accuracy_m: message["acc"].as_f64(),
        altitude_m: message["alt"].as_f64(),
        speed_mps: message["vel"].as_f64().map(|kmh| kmh / KMH_PER_MPS),
    })
}

/// The samples of the text of a file, in its order.
///
/// # Errors
/// [`FileError`] for the first entry that is not usable.
pub fn samples(text: &str) -> Result<Vec<SampleInput>, FileError> {
    let mut found = Vec::new();
    for (index, entry) in entries(text)?.into_iter().enumerate() {
        let number = index + 1;
        match entry.get("_type").and_then(Value::as_str) {
            Some("location") => found.push(owntracks(number, &entry)?),
            Some(_) => {}
            None => {
                found
                    .push(serde_json::from_value(entry).map_err(|_| FileError::NotSample(number))?);
            }
        }
    }
    Ok(found)
}

impl ImportArgs {
    /// Sends the samples in calls of at most 500 and says how many were new.
    ///
    /// # Errors
    /// [`Failure::Input`] for a file that cannot be read or holds an unusable entry; the hub's
    /// refusal, after which the calls before it stay stored.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let unreadable = |reason: String| Failure::Input {
            what: self.file.display().to_string(),
            reason,
        };
        let text =
            std::fs::read_to_string(&self.file).map_err(|error| unreadable(error.to_string()))?;
        let all = samples(&text).map_err(|error| unreadable(error.to_string()))?;
        let hub = invocation.hub()?;
        let mut added = 0;
        for batch in all.chunks(BATCH) {
            let variables = location_record::Variables {
                samples: batch.to_vec(),
            };
            added += hub
                .run::<LocationRecord>(variables)
                .await?
                .record_location_samples;
        }
        if invocation.output.json() {
            return invocation
                .output
                .raw(&serde_json::json!({ "read": all.len(), "added": added }));
        }
        invocation.output.line(&format!(
            "{} sample(s) read, {added} new. The hub already had the others.",
            all.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r_145_owntracks_lines_give_the_key_the_time_and_a_speed_in_metres() {
        let text = "{\"_type\":\"location\",\"tst\":1794830400,\"lat\":52.1,\"lon\":4.2,\
                    \"vel\":36,\"tid\":\"ph\"}\n\n{\"_type\":\"transition\",\"tst\":5}\n";
        let found = samples(text).unwrap();
        assert_eq!(
            found.len(),
            1,
            "a message that is no location is passed over"
        );
        assert_eq!(found[0].key, "ph:1794830400");
        assert_eq!(
            found[0].observed_at.as_deref(),
            Some("2026-11-16T12:00:00Z")
        );
        assert_eq!(found[0].speed_mps, Some(10.0));
        assert_eq!(found[0].accuracy_m, None, "no accuracy stays unknown");
    }

    #[test]
    fn r_145_a_list_in_the_anima_form_is_read_as_it_is() {
        let text = r#"[{"key":"a","observedAt":"2026-11-16T12:00:00Z","latitude":1.5,
                       "longitude":2.5,"accuracyM":8},{"key":"b"}]"#;
        let found = samples(text).unwrap();
        assert_eq!(
            (found[0].key.as_str(), found[0].accuracy_m),
            ("a", Some(8.0))
        );
        assert_eq!((found[1].key.as_str(), found[1].latitude), ("b", None));
    }

    #[test]
    fn r_145_an_unusable_entry_is_named_by_its_number() {
        assert_eq!(
            samples("{\"key\":\"a\"}\nnot json"),
            Err(FileError::NotJson(2))
        );
        assert_eq!(samples("{\"name\":\"a\"}"), Err(FileError::NotSample(1)));
        let no_time = "{\"_type\":\"location\",\"lat\":1.0,\"lon\":2.0}";
        assert_eq!(samples(no_time), Err(FileError::OwnTracks(1)));
    }
}
