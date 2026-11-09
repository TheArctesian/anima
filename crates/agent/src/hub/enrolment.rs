//! `anima-agent enrol`: trade a code for a token and keep both (MET-012, MET-013, K8 Section 3).

use agent_core::wire::{EnrolmentCode, RefusalCode};

use crate::config::Config;
use crate::config::home::{Home, HomeError};
use crate::config::origin::Origin;
use crate::hub::HubClient;
use crate::hub::answer::{Answer, Cause};
use crate::token_store::{Credential, TokenStore, TokenStoreError};

/// What `enrol` prints for every refusal: the hub says nothing about which codes exist, and
/// neither does the agent (K8 Section 3).
pub const REFUSED: &str = "The hub refused this enrolment code. Make a new code on the device's page and run the command again.";

#[derive(Debug, thiserror::Error)]
pub enum EnrolError {
    #[error("{REFUSED}")]
    Refused,
    #[error("{0}; if the hub may have received the code, make a new one")]
    Failed(Cause),
    #[error("enrolled, but the token could not be kept: {0}")]
    Token(#[from] TokenStoreError),
    #[error("enrolled, but the configuration could not be kept: {0}")]
    Home(#[from] HomeError),
}

/// The places an enrolment writes to and the hub it speaks to.
pub struct Enrolment<'a> {
    pub hub: &'a HubClient,
    /// The origin stored in the configuration; the client may have been built from another URL
    /// only in tests.
    pub origin: Origin,
    pub home: &'a Home,
    pub store: &'a TokenStore,
}

impl Enrolment<'_> {
    /// Spends `code` at the hub, stores the token with its expiry, then the configuration.
    /// Returns the name of the device.
    ///
    /// # Errors
    /// [`EnrolError::Refused`] for `ENROLMENT_REFUSED` and [`EnrolError::Failed`] for any other
    /// failure of the request. A failure to store after a good answer is reported as such: the
    /// code is spent then (MET-012).
    pub async fn spend(self, code: EnrolmentCode) -> Result<String, EnrolError> {
        let enrolled = match self.hub.enrol(code).await {
            Answer::Accepted(enrolled) => enrolled,
            Answer::Refused(RefusalCode::EnrolmentRefused) => return Err(EnrolError::Refused),
            Answer::Refused(_) => return Err(EnrolError::Failed(Cause::Status(401))),
            Answer::Failed(cause) => return Err(EnrolError::Failed(cause)),
        };
        self.store.save(&Credential {
            token: enrolled.token,
            expires_at: enrolled.expires_at,
        })?;
        self.home.write_config(&Config {
            hub: self.origin,
            device: enrolled.device,
            device_name: enrolled.device_name.clone(),
        })?;
        Ok(enrolled.device_name)
    }
}
