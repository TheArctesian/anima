//! The TLS of the names that serve: one certificate for each name, found by the SNI field
//! (Section 12.3). A name whose certificate is not valid for it is left out, so a certificate
//! can never serve a different name. I/O free.

use std::sync::Arc;

use rustls::{
    ServerConfig,
    crypto::{CryptoProvider, ring},
    server::ResolvesServerCertUsingSni,
    sign::CertifiedKey,
};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject as _};

use crate::wire::Table;

/// The server configuration of `table`, and the names that were left out.
///
/// # Errors
/// When the TLS library has no safe default for the provider (never with `ring`).
pub fn server_config(table: &Table) -> Result<(Arc<ServerConfig>, Vec<String>), rustls::Error> {
    let provider = Arc::new(ring::default_provider());
    let mut resolver = ResolvesServerCertUsingSni::new();
    let mut left_out = Vec::new();
    for named in &table.names {
        let added = certified(&provider, &named.chain_pem, named.key_pem.as_bytes())
            .and_then(|key| resolver.add(&named.name, key).ok());
        if added.is_none() {
            left_out.push(named.name.clone());
        }
    }
    let mut config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(resolver));
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok((Arc::new(config), left_out))
}

/// The chain and the key as rustls holds them; `None` when either does not parse.
fn certified(provider: &CryptoProvider, chain: &str, key: &[u8]) -> Option<CertifiedKey> {
    let chain: Vec<CertificateDer<'static>> = CertificateDer::pem_slice_iter(chain.as_bytes())
        .collect::<Result<_, _>>()
        .ok()
        .filter(|chain: &Vec<_>| !chain.is_empty())?;
    let key = PrivateKeyDer::from_pem_slice(key).ok()?;
    let signing = provider.key_provider.load_private_key(key).ok()?;
    Some(CertifiedKey::new(chain, signing))
}
