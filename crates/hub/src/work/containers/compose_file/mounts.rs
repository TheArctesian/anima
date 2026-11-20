//! The value of `--mount` of `docker run` as a volume of a Compose file in its long form.

use super::writer::Value;

/// One `--mount` as Compose writes it.
pub(super) struct Mount {
    pub volume: Value,
    /// The name of the volume of Docker, for a mount of the type `volume` that names one.
    pub named: Option<String>,
}

/// Whether `name` is a name of a volume or of a network of Docker.
pub(super) fn is_name(name: &str) -> bool {
    let mut characters = name.chars();
    let first = characters.next().is_some_and(|c| c.is_ascii_alphanumeric());
    let rest = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-');
    first && name.len() > 1 && characters.all(rest)
}

/// A truth value as Docker reads it.
pub(super) fn truth(text: &str) -> Option<bool> {
    match text {
        "true" | "True" | "TRUE" | "t" | "T" | "1" => Some(true),
        "false" | "False" | "FALSE" | "f" | "F" | "0" => Some(false),
        _ => None,
    }
}

#[derive(Default)]
struct Fields {
    kind: Option<String>,
    source: Option<String>,
    target: Option<String>,
    read_only: bool,
    /// The keys of the part of the long form that is for one type: `bind`, `volume`, `tmpfs`.
    typed: Vec<(&'static str, &'static str, Value)>,
}

impl Fields {
    /// Reads one field; `None` for a field that the converter does not know.
    fn read(&mut self, key: &str, value: Option<&str>) -> Option<()> {
        match key {
            "type" => self.kind = Some(value?.to_owned()),
            "source" | "src" => self.source = Some(value?.to_owned()),
            "target" | "destination" | "dst" => self.target = Some(value?.to_owned()),
            "readonly" | "ro" => self.read_only = value.map_or(Some(true), truth)?,
            "volume-nocopy" => {
                let nocopy = Value::Truth(value.map_or(Some(true), truth)?);
                self.typed.push(("volume", "nocopy", nocopy));
            }
            "bind-propagation" => {
                self.typed
                    .push(("bind", "propagation", Value::text(value?)));
            }
            "tmpfs-size" => {
                let size = Value::Number(value?.parse().ok()?);
                self.typed.push(("tmpfs", "size", size));
            }
            _ => return None,
        }
        Some(())
    }

    fn mount(self) -> Option<Mount> {
        let kind = self.kind.unwrap_or_else(|| "volume".to_owned());
        let known = matches!(kind.as_str(), "bind" | "volume" | "tmpfs");
        let needs_source = kind == "bind" && self.source.is_none();
        if !known || needs_source || self.typed.iter().any(|(part, ..)| *part != kind) {
            return None;
        }
        let mut volume = Value::map();
        volume.set("type", Value::Text(kind.clone()));
        if let Some(source) = &self.source {
            volume.set("source", Value::text(source));
        }
        volume.set("target", Value::Text(self.target?));
        if self.read_only {
            volume.set("read_only", Value::Truth(true));
        }
        for (part, key, value) in self.typed {
            volume.child(part).set(key, value);
        }
        let named = self.source.filter(|name| kind == "volume" && is_name(name));
        Some(Mount { volume, named })
    }
}

/// The value of one `--mount`; `None` for a form that the converter does not convert.
pub(super) fn mount(text: &str) -> Option<Mount> {
    // Docker reads the fields as one line of CSV. A quoted field is not converted.
    if text.contains('"') {
        return None;
    }
    let mut fields = Fields::default();
    for field in text.split(',') {
        let (key, value) = match field.split_once('=') {
            Some((key, value)) => (key, Some(value)),
            None => (field, None),
        };
        fields.read(key, value)?;
    }
    fields.mount()
}
