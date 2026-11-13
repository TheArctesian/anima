//! The K1 `WebInterfaceInput` as flags: `--label`, `--url`, `--open-in`. `create` takes them
//! all with `--url` required; `update` takes each as a change over the current values.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use super::current::Current;
use crate::failure::Failure;

#[derive(Clone, Copy, Debug, clap::ValueEnum, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum OpenInArg {
    AnimaTab,
    SystemBrowser,
}

#[derive(Debug, clap::Args)]
pub struct WebFields {
    /// The address, such as `https://nas.lan:8443/` (the hub reads the host and port from it).
    #[arg(long)]
    url: String,
    /// The label; the URL's host when left out.
    #[arg(long)]
    label: Option<String>,
    /// Where it opens; the Anima tab when left out.
    #[arg(long, value_enum)]
    open_in: Option<OpenInArg>,
}

impl WebFields {
    /// The input as the K1 input type `T` of a document.
    ///
    /// # Errors
    /// [`Failure::Usage`] when K1 would not accept it.
    pub fn typed<T: DeserializeOwned>(&self) -> Result<T, Failure> {
        let mut input = Map::new();
        input.insert("url".to_owned(), json!(self.url));
        if let Some(label) = &self.label {
            input.insert("label".to_owned(), json!(label));
        }
        if let Some(open_in) = &self.open_in {
            input.insert("openIn".to_owned(), json!(open_in));
        }
        serde_json::from_value(Value::Object(input)).map_err(|error| Failure::Usage {
            message: format!("The web interface is not valid: {error}."),
            hint: None,
        })
    }
}

#[derive(Debug, clap::Args)]
pub struct WebChanges {
    /// The new address, such as `https://nas.lan:8443/`.
    #[arg(long)]
    url: Option<String>,
    /// The new label.
    #[arg(long)]
    label: Option<String>,
    /// Where it opens.
    #[arg(long, value_enum)]
    open_in: Option<OpenInArg>,
}

impl WebChanges {
    /// Whether no flag was given.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.url.is_none() && self.label.is_none() && self.open_in.is_none()
    }

    /// The input as the K1 input type `T`: the flags laid over the `current` values.
    ///
    /// # Errors
    /// [`Failure::Usage`] when K1 would not accept it.
    pub fn typed_over<T: DeserializeOwned>(&self, current: Current) -> Result<T, Failure> {
        let open_in = self
            .open_in
            .map_or(json!(current.open_in), |open_in| json!(open_in));
        let input = json!({
            "url": self.url.clone().unwrap_or(current.url),
            "label": self.label.clone().unwrap_or(current.label),
            "openIn": open_in,
        });
        serde_json::from_value(input).map_err(|error| Failure::Usage {
            message: format!("The web interface is not valid: {error}."),
            hint: None,
        })
    }
}
