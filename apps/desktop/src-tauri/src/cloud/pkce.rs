//! The start of a sign-in inside the app (mobile plan M3-5): the authorization code flow of `WorkOS`
//! `AuthKit` with PKCE (RFC 7636), for a public client that holds only the client ID (HMD-032, HMD-035).
//! The endpoint and its fields are those of
//! <https://workos.com/docs/reference/user-management/authentication/get-authorization-url>, read on
//! 2026-11-19. This file makes values and calls no network.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

const AUTHORIZE_URL: &str = "https://api.workos.com/user_management/authorize";

/// One sign-in inside the app. No `Debug`: the verifier is a secret until the exchange.
pub(super) struct Attempt {
    /// The code verifier: only the shell that made it can trade the code for tokens.
    pub verifier: Zeroizing<String>,
    /// The value that the return of the browser must carry, so that an answer to a different
    /// sign-in is refused.
    pub state: String,
}

impl Attempt {
    /// A sign-in from 64 random bytes: 32 for the verifier and 32 for the state.
    pub(super) fn from_random(random: &[u8; 64]) -> Self {
        Self {
            verifier: Zeroizing::new(URL_SAFE_NO_PAD.encode(&random[..32])),
            state: URL_SAFE_NO_PAD.encode(&random[32..]),
        }
    }

    /// The page that the person signs in on. `WorkOS` sends the browser to `redirect` after it.
    pub(super) fn page(&self, client_id: &str, redirect: &str) -> Option<String> {
        let challenge = challenge(&self.verifier);
        reqwest::Url::parse_with_params(
            AUTHORIZE_URL,
            [
                ("response_type", "code"),
                ("client_id", client_id),
                ("redirect_uri", redirect),
                ("provider", "authkit"),
                ("state", self.state.as_str()),
                ("code_challenge", challenge.as_str()),
                ("code_challenge_method", "S256"),
            ],
        )
        .ok()
        .map(String::from)
    }
}

/// The S256 code challenge of `verifier`.
fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::{Attempt, challenge};

    #[test]
    fn m3_5_the_challenge_is_the_one_of_rfc_7636() {
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn m3_5_the_verifier_and_the_state_are_43_safe_characters_and_differ() {
        let mut random = [7_u8; 64];
        random[40] = 9;
        let attempt = Attempt::from_random(&random);
        for value in [attempt.verifier.as_str(), attempt.state.as_str()] {
            assert_eq!(value.len(), 43);
            assert!(
                value
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            );
        }
        assert_ne!(attempt.verifier.as_str(), attempt.state);
    }

    #[test]
    fn m3_5_the_page_carries_the_client_the_return_address_the_state_and_the_challenge_only() {
        let mut random = [1_u8; 64];
        random[..32].fill(2);
        let attempt = Attempt::from_random(&random);
        let page = attempt
            .page("client_123", "http://127.0.0.1:47113/sign-in")
            .unwrap();
        assert!(page.starts_with("https://api.workos.com/user_management/authorize?"));
        for part in [
            "response_type=code",
            "client_id=client_123",
            "redirect_uri=http%3A%2F%2F127.0.0.1%3A47113%2Fsign-in",
            "provider=authkit",
            "code_challenge_method=S256",
        ] {
            assert!(page.contains(part), "{part}");
        }
        assert!(page.contains(&format!("state={}", attempt.state)));
        assert!(page.contains(&format!("code_challenge={}", challenge(&attempt.verifier))));
        assert!(!page.contains(attempt.verifier.as_str()));
    }
}
