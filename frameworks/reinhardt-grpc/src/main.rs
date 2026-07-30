use std::pin::Pin;

use reinhardt::grpc::GrpcServerSettings;
use tokio_stream::{Stream, StreamExt};
use tonic::transport::{Identity, Server, ServerTlsConfig};
use tonic::{Request, Response, Status, Streaming};

pub mod benchmark {
    tonic::include_proto!("benchmark");
}

use benchmark::benchmark_service_server::{BenchmarkService, BenchmarkServiceServer};
use benchmark::{StreamRequest, SumReply, SumRequest};

type ReplyStream = Pin<Box<dyn Stream<Item = Result<SumReply, Status>> + Send + 'static>>;

#[derive(Default)]
struct BenchmarkServiceImpl;

#[tonic::async_trait]
impl BenchmarkService for BenchmarkServiceImpl {
    async fn get_sum(&self, request: Request<SumRequest>) -> Result<Response<SumReply>, Status> {
        let request = request.into_inner();
        Ok(Response::new(SumReply {
            result: request.a + request.b,
        }))
    }

    type StreamSumStream = ReplyStream;

    async fn stream_sum(
        &self,
        request: Request<StreamRequest>,
    ) -> Result<Response<Self::StreamSumStream>, Status> {
        let request = request.into_inner();
        let count = request.count.max(0) as usize;
        let result = request.a + request.b;
        let replies = tokio_stream::iter((0..count).map(move |_| Ok(SumReply { result })));
        Ok(Response::new(Box::pin(replies)))
    }

    async fn collect_sum(
        &self,
        request: Request<Streaming<SumRequest>>,
    ) -> Result<Response<SumReply>, Status> {
        let mut stream = request.into_inner();
        let mut result = 0;
        while let Some(request) = stream.next().await {
            let request = request?;
            result += request.a + request.b;
        }
        Ok(Response::new(SumReply { result }))
    }

    type EchoSumStream = ReplyStream;

    async fn echo_sum(
        &self,
        request: Request<Streaming<SumRequest>>,
    ) -> Result<Response<Self::EchoSumStream>, Status> {
        let replies = request.into_inner().map(|request| {
            request.map(|request| SumReply {
                result: request.a + request.b,
            })
        });
        Ok(Response::new(Box::pin(replies)))
    }
}

fn configured_service() -> BenchmarkServiceServer<BenchmarkServiceImpl> {
    let settings = GrpcServerSettings {
        max_decoding_message_size: 4 * 1024 * 1024,
        max_encoding_message_size: 4 * 1024 * 1024,
        request_timeout_secs: 30,
        max_concurrent_connections: 4096,
    };

    BenchmarkServiceServer::new(BenchmarkServiceImpl)
        .max_decoding_message_size(settings.max_decoding_message_size)
        .max_encoding_message_size(settings.max_encoding_message_size)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unary_sum_uses_both_operands() {
        let response = BenchmarkServiceImpl
            .get_sum(Request::new(SumRequest { a: 13, b: 42 }))
            .await
            .unwrap();
        assert_eq!(response.into_inner().result, 55);
    }

    #[tokio::test]
    async fn stream_sum_returns_requested_number_of_identical_replies() {
        let response = BenchmarkServiceImpl
            .stream_sum(Request::new(StreamRequest {
                a: 13,
                b: 42,
                count: 3,
            }))
            .await
            .unwrap();
        let replies: Vec<_> = response.into_inner().collect().await;
        assert_eq!(replies.len(), 3);
        assert!(replies.into_iter().all(|reply| reply.unwrap().result == 55));
    }
}
