//! Relay identity and TLS plumbing.
//!
//! A syncweb node identifies itself to a Syncthing relay with an X.509 client
//! certificate. The device ID is the SHA-256 of the certificate in DER form.
//! The certificate is generated once, stored in the relay data directory and
//! reloaded on subsequent starts so the identity is stable across restarts.

use std::path::Path;
use std::sync::Arc;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use sha2::{Digest, Sha256};
use tokio_rustls::TlsConnector;

use crate::error::{Result, SyncwebError};

/// The TLS ALPN we request from relay servers.
pub const RELAY_ALPN: &[u8] = super::protocol::PROTOCOL_NAME;
/// File holding the DER-encoded relay certificate.
const CERT_FILE: &str = "relay-cert.der";
/// File holding the PKCS#8 DER-encoded relay key.
const KEY_FILE: &str = "relay-key.der";

/// The persistent relay identity of this node.
#[derive(Debug)]
pub struct RelayCerts {
    cert_der: CertificateDer<'static>,
    key_der: PrivateKeyDer<'static>,
    relay_id: [u8; 32],
}

impl RelayCerts {
    /// Load the identity from `data_dir`, generating and persisting it on
    /// first use.
    pub(crate) fn load_or_create(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let cert_path = data_dir.join(CERT_FILE);
        let key_path = data_dir.join(KEY_FILE);
        if let (Ok(cert), Ok(key)) = (std::fs::read(&cert_path), std::fs::read(&key_path)) {
            return Self::from_der(cert, key);
        }
        let generated = Self::generate()?;
        std::fs::write(&cert_path, generated.cert_der().as_ref())?;
        std::fs::write(&key_path, generated.key_der_for_storage())?;
        Ok(generated)
    }

    /// Wrap freshly generated or reloaded key material.
    fn from_der(cert_bytes: Vec<u8>, key_bytes: Vec<u8>) -> Result<Self> {
        let key_der = PrivateKeyDer::try_from(key_bytes)
            .map_err(|error| SyncwebError::RelayDecode(format!("stored relay key is invalid: {error}")))?;
        let relay_id = relay_id_from_cert(&cert_bytes);
        Ok(Self {
            cert_der: CertificateDer::from(cert_bytes),
            key_der,
            relay_id,
        })
    }

    /// Generate a fresh identity backed by a P-256 ECDSA key.
    fn generate() -> Result<Self> {
        use rcgen::{CertificateParams, KeyPair, PKCS_ECDSA_P256_SHA256};

        let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)
            .map_err(|error| SyncwebError::RelayDecode(format!("relay key generation failed: {error}")))?;
        let mut params =
            CertificateParams::new(Vec::new()).map_err(|error| SyncwebError::RelayDecode(error.to_string()))?;
        params.distinguished_name.push(rcgen::DnType::CommonName, "syncweb");
        params.is_ca = rcgen::IsCa::NoCa;
        params.key_usages = vec![rcgen::KeyUsagePurpose::DigitalSignature];
        let certificate = params
            .self_signed(&key_pair)
            .map_err(|error| SyncwebError::RelayDecode(format!("relay certificate signing failed: {error}")))?;
        let cert_der = certificate.der().as_ref().to_vec();
        let key_der = key_pair.serialize_der();
        Self::from_der(cert_der, key_der)
    }

    /// The 32-byte relay identity (SHA-256 of the certificate DER).
    #[must_use]
    pub const fn relay_id(&self) -> &[u8; 32] {
        &self.relay_id
    }

    /// The certificate sent to relays during the handshake.
    #[must_use]
    pub const fn cert_der(&self) -> &CertificateDer<'static> {
        &self.cert_der
    }

    /// A copy of the private key for the TLS client auth callback.
    #[must_use]
    pub const fn key_der(&self) -> &PrivateKeyDer<'static> {
        &self.key_der
    }

    fn key_der_for_storage(&self) -> &[u8] {
        self.key_der.secret_der()
    }
}

fn relay_id_from_cert(cert_der: &[u8]) -> [u8; 32] {
    Sha256::digest(cert_der).into()
}

/// Compute the [`ServerName`] for a relay host, IPv4/IPv6 literals included.
///
/// # Errors
///
/// Returns an error if `host` is neither a valid IP nor a valid DNS name.
pub fn server_name_for(host: &str) -> Result<ServerName<'static>> {
    if let Ok(address) = host.parse::<std::net::IpAddr>() {
        return Ok(ServerName::IpAddress(address.into()));
    }
    ServerName::try_from(host.to_owned()).map_err(|error| SyncwebError::RelayUnreachable {
        reasons: format!("{host} is not a usable relay host: {error}"),
    })
}

/// Build a TLS connector that trusts any relay endpoint and presents our
/// client certificate. Relays are a trust-on-first-certificate service:
/// the certificate *we* present identifies *us*; the relay's own certificate
/// is authenticated out of band, so we deliberately skip server
/// verification (matching the upstream sync client).
///
/// # Errors
///
/// Returns an error if the provider or the client configuration is unusable.
pub fn tls_connector(certs: &RelayCerts) -> Result<TlsConnector> {
    ensure_provider_installed();
    let tls_error = |error: rustls::Error| SyncwebError::RelayUnreachable {
        reasons: format!("relay TLS setup failed: {error}"),
    };
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
        .map_err(tls_error)?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(RelayServerVerifier))
        .with_client_auth_cert(vec![certs.cert_der().clone()], certs.key_der().clone_key())
        .map_err(tls_error)?;
    config.alpn_protocols = vec![RELAY_ALPN.to_vec()];
    Ok(TlsConnector::from(Arc::new(config)))
}

/// Install the ring provider as the process default if none has been set.
/// iroh's own provider choice is not guaranteed to be registered as the
/// rustls default, which other crates (and our HTTP pool client) depend on.
///
/// # Panics
///
/// Panics if a provider install aborts the process.
pub fn ensure_provider_installed() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
}

/// A server cert verifier that accepts every certificate a relay presents.
#[derive(Debug)]
struct RelayServerVerifier;

impl rustls::client::danger::ServerCertVerifier for RelayServerVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_round_trips_on_disk() {
        let dir = std::env::temp_dir().join(format!("syncweb-relay-cert-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let first = RelayCerts::load_or_create(&dir).expect("create");
        let second = RelayCerts::load_or_create(&dir).expect("reload");
        assert_eq!(first.relay_id(), second.relay_id());
        assert_eq!(first.cert_der(), second.cert_der());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn connector_builds_for_generated_and_reloaded_certs() {
        let dir = std::env::temp_dir().join(format!("syncweb-relay-cert-{}-2", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let certs = RelayCerts::load_or_create(&dir).expect("create");
        tls_connector(&certs).expect("connector");
        let reloaded = RelayCerts::load_or_create(&dir).expect("reload");
        tls_connector(&reloaded).expect("connector for reloaded");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn server_name_from_host() {
        let ip = server_name_for("127.0.0.1").expect("ip");
        assert!(matches!(ip, ServerName::IpAddress(_)));
        let dns = server_name_for("relay.example.com").expect("dns");
        assert!(matches!(dns, ServerName::DnsName(_)));
        assert!(server_name_for("not a host").is_err());
    }

    #[test]
    fn relay_id_is_sha256_of_der() {
        let certs = RelayCerts::load_or_create(&std::env::temp_dir()).expect("create");
        let expected: [u8; 32] = Sha256::digest(certs.cert_der().as_ref()).into();
        assert_eq!(certs.relay_id(), &expected);
    }
}
