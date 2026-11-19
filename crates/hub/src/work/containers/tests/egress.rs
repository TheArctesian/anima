//! The read of one digest at a registry, against a local stand-in (HOST-021): the requests
//! the hub sends, and what each answer becomes.

use super::registry::{NEW, OLD, TOKEN, answer, known, reference, serve};
use crate::work::containers::updates::egress::{Egress, FetchError};

#[tokio::test]
async fn host_021_the_hub_asks_for_a_token_with_no_account_then_for_the_head_of_the_manifest() {
    let registry = known().await;
    let egress = registry.egress();
    assert_eq!(
        egress.digest(&reference("alpine:3")).await,
        Ok(OLD.to_owned())
    );
    let heads = registry.heads();
    assert_eq!(heads.len(), 2, "{heads:?}");
    let asked = "GET /token?service=registry.docker.io&scope=repository%3Alibrary%2Falpine%3Apull ";
    assert!(heads[0].starts_with(asked), "{heads:?}");
    // The request for a token sends no credential.
    assert!(
        !heads[0].to_lowercase().contains("authorization"),
        "{heads:?}"
    );
    assert!(
        heads[1].starts_with("HEAD /v2/library/alpine/manifests/3 "),
        "{heads:?}"
    );
    let lower = heads[1].to_lowercase();
    assert!(lower.contains(&format!("authorization: bearer {}", TOKEN.to_lowercase())));
    assert!(
        lower.contains("application/vnd.oci.image.index.v1+json"),
        "{heads:?}"
    );
    assert!(!lower.contains("cookie"), "{heads:?}");
}

#[tokio::test]
async fn host_021_a_registry_that_asks_for_no_token_gets_one_request() {
    let registry = known().await;
    let read = registry
        .egress()
        .digest(&reference("quay.io/prometheus/prometheus"))
        .await;
    assert_eq!(read, Ok(NEW.to_owned()));
    assert_eq!(
        registry.lines(),
        ["HEAD /v2/prometheus/prometheus/manifests/latest HTTP/1.1"]
    );
    assert!(!registry.heads()[0].to_lowercase().contains("authorization"));
}

#[tokio::test]
async fn host_021_a_private_or_unknown_tag_is_not_there_and_no_redirect_is_followed() {
    let registry = known().await;
    let egress = registry.egress();
    for gone in ["ghcr.io/team/private:1", "alpine:nosuchtag"] {
        assert_eq!(
            egress.digest(&reference(gone)).await,
            Err(FetchError::NotThere),
            "{gone}"
        );
    }
    let before = registry.heads().len();
    assert_eq!(
        egress.digest(&reference("moved:1")).await,
        Err(FetchError::Failed)
    );
    // The token and the one manifest request; the redirect led to no third request.
    assert_eq!(registry.heads().len(), before + 2);
    assert_eq!(
        egress.digest(&reference("odd:1")).await,
        Err(FetchError::Failed)
    );
}

#[tokio::test]
async fn host_021_a_registry_at_an_address_the_policy_refuses_is_not_connected_to() {
    let registry = known().await;
    // The same server and the same trusted certificate as the tests that reach it: only the
    // policy differs. The default policy refuses loopback, where the stand-in listens.
    let (origin, root) = registry.place();
    let refused =
        Egress::new(crate::fleet::targets::AddressPolicy::default()).standing_in(origin, root);
    assert_eq!(
        refused.digest(&reference("alpine:3")).await,
        Err(FetchError::Failed)
    );
    assert_eq!(registry.heads(), [] as [String; 0]);
}

#[tokio::test]
async fn host_021_a_token_endpoint_that_refuses_is_a_registry_that_was_not_read() {
    let registry = serve(|_, _| answer("403 Forbidden", "")).await;
    let read = registry.egress().digest(&reference("alpine:3")).await;
    assert_eq!(read, Err(FetchError::Failed));
}

#[test]
fn host_021_each_request_goes_to_the_fixed_host_of_the_registry_of_the_name() {
    let egress = Egress::new(crate::fleet::targets::AddressPolicy::default());
    let places = |image: &str| {
        let reference = reference(image);
        let token = egress.token_url(&reference).unwrap().map(String::from);
        (
            token,
            String::from(egress.manifest_url(&reference).unwrap()),
        )
    };
    let scope = "scope=repository%3Alibrary%2Falpine%3Apull";
    assert_eq!(
        places("alpine:3"),
        (
            Some(format!(
                "https://auth.docker.io/token?service=registry.docker.io&{scope}"
            )),
            "https://registry-1.docker.io/v2/library/alpine/manifests/3".to_owned()
        )
    );
    assert_eq!(
        places("ghcr.io/a/b.c:v1_x"),
        (
            Some(
                "https://ghcr.io/token?service=ghcr.io&scope=repository%3Aa%2Fb.c%3Apull"
                    .to_owned()
            ),
            "https://ghcr.io/v2/a/b.c/manifests/v1_x".to_owned()
        )
    );
    assert_eq!(
        places("quay.io/prometheus/prometheus"),
        (
            None,
            "https://quay.io/v2/prometheus/prometheus/manifests/latest".to_owned()
        )
    );
    assert_eq!(
        places("mcr.microsoft.com/dotnet/runtime:8.0"),
        (
            None,
            "https://mcr.microsoft.com/v2/dotnet/runtime/manifests/8.0".to_owned()
        )
    );
}
