use anyhow::Result;
use reqwest::Client;
use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub client: Client,
}

/// Builds an `rustls` client using bundled Mozilla roots, independent of the OS cert store
/// (needed since this may run in a minimal container without `ca-certificates`).
pub fn build_client() -> Result<Client> {
    let roots = webpki_root_certs::TLS_SERVER_ROOT_CERTS
        .iter()
        .map(|cert| reqwest::Certificate::from_der(cert))
        .collect::<reqwest::Result<Vec<_>>>()?;
    let client = Client::builder().tls_certs_only(roots).build()?;
    Ok(client)
}
