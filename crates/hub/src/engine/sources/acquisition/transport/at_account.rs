//! The request of a source whose server is of the operator (K-E1 Section 7b, R-210): the URL
//! is the base of the account plus the path, and the host of the base is resolved one time and
//! checked with the address policy of the hub before the key is sent. The policy is the one of
//! each outbound connection: it permits private and VPN ranges and refuses loopback,
//! link-local addresses, multicast, reserved ranges and the ranges of `ANIMA_PROBE_DENY`.

use std::net::IpAddr;

use reqwest::{
    Client, Url,
    header::{ACCEPT, AUTHORIZATION, HeaderValue},
    redirect::Policy,
};
use source_exchange::{Method, Request};

use super::{Answer, CONNECT_TIMEOUT, Fault, TIMEOUT, Transport, address, bearer, read};
use crate::fleet::{
    credentials::Secret,
    targets::{TargetError, resolve_checked},
};

impl Transport {
    /// Whether this transport also sends over plain `http`: only one made for a loopback stub
    /// of the tests does.
    pub(in crate::engine::sources) const fn sends_plain_http(&self) -> bool {
        self.base_override.is_some()
    }

    /// Sends the provider's `request` to the server at `base` with the key of the account.
    /// The connection goes only to an address that the policy permitted; the name is not
    /// resolved a second time, and no redirect is followed.
    ///
    /// # Errors
    /// [`Fault::Refused`] for a base that is not `https`, a path that could leave the base, or
    /// an address that the policy refuses: nothing is sent. [`Fault::Network`] when the name
    /// does not resolve or the server does not answer.
    pub(in crate::engine::sources) async fn get_at(
        &self,
        base: &Url,
        request: &Request,
        key: &Secret,
    ) -> Result<Answer, Fault> {
        let Method::Get = request.method;
        if base.scheme() != "https" && !self.sends_plain_http() {
            return Err(Fault::Refused);
        }
        let url = address(base, &request.path, &request.query)?;
        let host = base.host_str().ok_or(Fault::Refused)?;
        let port = base.port_or_known_default().ok_or(Fault::Refused)?;
        let addresses =
            resolve_checked(host, port, &self.policy)
                .await
                .map_err(|error| match error {
                    TargetError::Refused => Fault::Refused,
                    TargetError::Unresolved => Fault::Network,
                })?;
        let mut client = Client::builder()
            .use_rustls_tls()
            .user_agent("anima-hub")
            .no_proxy()
            .redirect(Policy::none())
            .https_only(!self.sends_plain_http())
            .timeout(TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT);
        let literal = host.starts_with('[') || host.parse::<IpAddr>().is_ok();
        if !literal {
            client = client.resolve_to_addrs(host, &addresses);
        }
        let client = client.build().map_err(|_| Fault::Network)?;
        let sent = client
            .get(url)
            .header(AUTHORIZATION, bearer(key.expose())?)
            .header(ACCEPT, HeaderValue::from_static("application/json"));
        read(sent.send().await.map_err(|_| Fault::Network)?).await
    }
}
