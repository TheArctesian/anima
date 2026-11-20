use serde::{Serialize, Serializer, ser::SerializeStruct};

/// Why a call of the sign-in or of the cloud stopped. Carries no token, code or answer text
/// (LOG-007); the client switches on the code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CloudError {
    /// This build has no client ID (`ANIMA_CLOUD_CLIENT_ID` was empty when it was built).
    #[error("this build has no Anima.ing sign-in")]
    Unconfigured,
    /// A web view other than the main one asked.
    #[error("only the main window can do this")]
    Caller,
    /// The person cancelled the sign-in in the browser (`access_denied`).
    #[error("the sign-in was cancelled")]
    Cancelled,
    /// The code ran out before the sign-in (`expired_token`), or no sign-in was started.
    #[error("the code expired")]
    Expired,
    /// No answer, or a server fault (5xx).
    #[error("the cloud did not answer")]
    Offline,
    /// Any other refusal of the cloud or of `WorkOS`.
    #[error("the cloud refused")]
    Refused,
    /// No valid session: the person signs in again.
    #[error("sign in again")]
    Unauthenticated,
    /// The payment of the account failed (402 `payment_failed`): the cloud makes and wakes nothing.
    #[error("the payment failed")]
    PaymentFailed,
    /// The hub of the account is not ready yet (409 `not_ready`).
    #[error("the hub is not ready")]
    NotReady,
    /// The key store refused to keep or give the refresh token.
    #[cfg_attr(
        mobile,
        expect(
            dead_code,
            reason = "a phone development build keeps the refresh token in memory, which cannot refuse (mobile plan M5-1)"
        )
    )]
    #[error("the key store refused")]
    Keystore,
}

impl CloudError {
    /// Stable code the client switches on.
    pub(super) fn code(self) -> &'static str {
        match self {
            Self::Unconfigured => "unconfigured",
            Self::Caller => "caller",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
            Self::Offline => "offline",
            Self::Refused => "refused",
            Self::Unauthenticated => "unauthenticated",
            Self::PaymentFailed => "payment-failed",
            Self::NotReady => "not-ready",
            Self::Keystore => "keystore",
        }
    }
}

/// Serialises as `{ "code": ... }` and nothing else (WP-31 "Shell interface").
impl Serialize for CloudError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("CloudError", 1)?;
        out.serialize_field("code", self.code())?;
        out.end()
    }
}

impl From<reqwest::Error> for CloudError {
    /// A transport failure: no answer arrived. Its text, which can quote the URL, is dropped.
    fn from(_: reqwest::Error) -> Self {
        Self::Offline
    }
}
