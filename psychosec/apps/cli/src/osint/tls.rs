//! TLS handshake inspection (verified against the Mozilla root set).
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use x509_parser::prelude::*;

pub struct CertInfo {
    pub subject: String,
    pub issuer: String,
    pub not_before: String,
    pub not_after: String,
    pub days_left: i64,
    pub sans: Vec<String>,
    pub serial: String,
}

pub struct TlsInfo {
    pub version: String,
    pub cipher: String,
    pub alpn: Option<String>,
    pub leaf: CertInfo,
}

fn config() -> Arc<rustls::ClientConfig> {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut cfg = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("tls versions")
        .with_root_certificates(roots)
        .with_no_client_auth();
    cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Arc::new(cfg)
}

pub async fn inspect(host: &str, port: u16, timeout: std::time::Duration) -> Result<TlsInfo, String> {
    let name = rustls::pki_types::ServerName::try_from(host.to_string()).map_err(|_| "invalid server name".to_string())?;
    let fut = async {
        let tcp = TcpStream::connect((host, port)).await.map_err(|e| format!("connect failed: {e}"))?;
        let tls = TlsConnector::from(config())
            .connect(name, tcp)
            .await
            .map_err(|e| format!("TLS handshake/verification failed: {e}"))?;
        let (_, conn) = tls.get_ref();
        let certs = conn.peer_certificates().ok_or("server presented no certificate")?;
        let (_, leaf) = X509Certificate::from_der(certs[0].as_ref()).map_err(|e| format!("cannot parse certificate: {e}"))?;
        let now = chrono::Utc::now().timestamp();
        let nb = leaf.validity().not_before.timestamp();
        let na = leaf.validity().not_after.timestamp();
        let fmt = |t: i64| chrono::DateTime::from_timestamp(t, 0).map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default();
        let mut sans = vec![];
        if let Ok(Some(ext)) = leaf.subject_alternative_name() {
            for n in &ext.value.general_names {
                if let GeneralName::DNSName(d) = n {
                    sans.push(d.to_string());
                }
            }
        }
        Ok(TlsInfo {
            version: conn.protocol_version().map(|v| format!("{v:?}")).unwrap_or_else(|| "unknown".into()),
            cipher: conn.negotiated_cipher_suite().map(|c| format!("{:?}", c.suite())).unwrap_or_else(|| "unknown".into()),
            alpn: conn.alpn_protocol().map(|p| String::from_utf8_lossy(p).into_owned()),
            leaf: CertInfo {
                subject: leaf.subject().to_string(),
                issuer: leaf.issuer().to_string(),
                not_before: fmt(nb),
                not_after: fmt(na),
                days_left: (na - now) / 86400,
                sans,
                serial: leaf.raw_serial_as_string(),
            },
        })
    };
    tokio::time::timeout(timeout, fut).await.map_err(|_| "TLS inspection timed out".to_string())?
}
