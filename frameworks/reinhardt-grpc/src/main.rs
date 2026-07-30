use httparena_reinhardt_grpc::apps::benchmark::views::configured_service;
use tonic::transport::{Identity, Server, ServerTlsConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let certificate_path =
        std::env::var("TLS_CERT").unwrap_or_else(|_| "/certs/server.crt".to_string());
    let private_key_path =
        std::env::var("TLS_KEY").unwrap_or_else(|_| "/certs/server.key".to_string());
    let certificate = tokio::fs::read(certificate_path).await?;
    let private_key = tokio::fs::read(private_key_path).await?;
    let identity = Identity::from_pem(certificate, private_key);

    println!("Reinhardt gRPC server listening on :8080 and :8443");

    let plaintext = Server::builder()
        .add_service(configured_service())
        .serve("0.0.0.0:8080".parse()?);
    let tls = Server::builder()
        .tls_config(ServerTlsConfig::new().identity(identity))?
        .add_service(configured_service())
        .serve("0.0.0.0:8443".parse()?);

    tokio::try_join!(plaintext, tls)?;
    Ok(())
}
