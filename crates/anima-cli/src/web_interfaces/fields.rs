//! The K1 `WebInterfaceInput` as flags: `--label`, `--url`, `--open-in`.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

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
