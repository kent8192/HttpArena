use std::convert::Infallible;
use std::fs::File;
use std::io::{self, BufReader};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use httparena_reinhardt::apps::benchmark::{
    ArenaRouter, ArenaState, initialize_state, load_dataset,
};
use hyper::Method;
use hyper::body::Incoming;
use hyper::server::conn::{http1, http2};
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use reinhardt::DatabaseConnection;
use reinhardt::http::{Handler, Request, Response};
use reinhardt::server::serve_http2;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

fn load_tls_config(alpn_protocols: Vec<Vec<u8>>) -> io::Result<rustls::ServerConfig> {
    let cert_path = std::env::var("TLS_CERT").unwrap_or_else(|_| "/certs/server.crt".to_string());
    let key_path = std::env::var("TLS_KEY").unwrap_or_else(|_| "/certs/server.key".to_string());
    let cert_file = File::open(cert_path)?;
    let key_file = File::open(key_path)?;
    let certs =
        rustls_pemfile::certs(&mut BufReader::new(cert_file)).collect::<Result<Vec<_>, _>>()?;
    let key = rustls_pemfile::private_key(&mut BufReader::new(key_file))?
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing TLS private key"))?;
    let mut config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    config.alpn_protocols = alpn_protocols;
    Ok(config)
}

async fn serve_tls_http1(
    addr: SocketAddr,
    handler: ArenaRouter,
) -> Result<(), Box<dyn std::error::Error>> {
    let acceptor = TlsAcceptor::from(Arc::new(load_tls_config(vec![b"http/1.1".to_vec()])?));
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let handler = handler.clone();
        tokio::spawn(async move {
            let Ok(stream) = acceptor.accept(stream).await else {
                return;
            };
            let service = service_fn(move |request| {
                handle_hyper_request(request, handler.clone(), true, Some(remote_addr))
            });
            let _ = http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service)
                .await;
        });
    }
}

async fn serve_plain_http1(
    addr: SocketAddr,
    handler: ArenaRouter,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let handler = handler.clone();
        tokio::spawn(async move {
            let service = service_fn(move |request| {
                handle_hyper_request(request, handler.clone(), false, Some(remote_addr))
            });
            let _ = http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service)
                .await;
        });
    }
}

async fn serve_tls_http2(
    addr: SocketAddr,
    handler: ArenaRouter,
) -> Result<(), Box<dyn std::error::Error>> {
    let acceptor = TlsAcceptor::from(Arc::new(load_tls_config(vec![b"h2".to_vec()])?));
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let handler = handler.clone();
        tokio::spawn(async move {
            let Ok(stream) = acceptor.accept(stream).await else {
                return;
            };
            let service = service_fn(move |request| {
                handle_hyper_request(request, handler.clone(), true, Some(remote_addr))
            });
            let _ = http2::Builder::new(hyper_util::rt::TokioExecutor::new())
                .serve_connection(TokioIo::new(stream), service)
                .await;
        });
    }
}

async fn handle_hyper_request(
    request: hyper::Request<Incoming>,
    handler: ArenaRouter,
    secure: bool,
    remote_addr: Option<SocketAddr>,
) -> Result<hyper::Response<Full<Bytes>>, Infallible> {
    let (parts, body) = request.into_parts();
    let body = body
        .collect()
        .await
        .map(|collected| collected.to_bytes())
        .unwrap_or_default();
    let mut request = Request::builder()
        .method(parts.method)
        .uri(parts.uri)
        .version(parts.version)
        .headers(parts.headers)
        .body(body)
        .secure(secure);
    if let Some(remote_addr) = remote_addr {
        request = request.remote_addr(remote_addr);
    }
    let request = request.build().unwrap_or_else(|_| {
        Request::builder()
            .method(Method::GET)
            .uri("/")
            .build()
            .expect("fallback request must be valid")
    });
    let response = handler.handle(request).await.unwrap_or_else(Response::from);
    Ok(into_hyper_response(response))
}

fn into_hyper_response(response: Response) -> hyper::Response<Full<Bytes>> {
    let mut output = hyper::Response::new(Full::new(response.body));
    *output.status_mut() = response.status;
    *output.headers_mut() = response.headers;
    output
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let static_dir =
        PathBuf::from(std::env::var("STATIC_DIR").unwrap_or_else(|_| "/data/static".to_string()));
    let mut state = ArenaState::new(load_dataset(), static_dir);
    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        let pool_size = std::env::var("DATABASE_MAX_CONN")
            .ok()
            .and_then(|value| value.parse::<u32>().ok());
        let database = DatabaseConnection::connect_with_pool_size(&database_url, pool_size).await?;
        state = state.with_database(database);
    }
    if initialize_state(state).is_err() {
        panic!("benchmark state must only be initialized once");
    }
    let handler = ArenaRouter::new();

    let h1_addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    if std::env::var("HTTPARENA_GATEWAY_MODE").as_deref() == Ok("1") {
        serve_plain_http1(h1_addr, handler).await?;
        return Ok(());
    }

    println!("Reinhardt HttpArena server listening on :8080, :8081, :8082 and :8443");
    tokio::select! {
        result = serve_plain_http1(h1_addr, handler.clone()) => result?,
        result = serve_tls_http1(SocketAddr::from(([0, 0, 0, 0], 8081)), handler.clone()) => result?,
        result = serve_http2(SocketAddr::from(([0, 0, 0, 0], 8082)), handler.clone()) => result?,
        result = serve_tls_http2(SocketAddr::from(([0, 0, 0, 0], 8443)), handler) => result?,
    }
    Ok(())
}
