//! URL configuration for the benchmark app.

use reinhardt::ServerRouter;
use reinhardt::grpc::GrpcServerSettings;

use crate::benchmark::benchmark_service_server::BenchmarkServiceServer;

use super::views::BenchmarkServiceImpl;

pub fn server_url_patterns() -> ServerRouter {
    ServerRouter::new()
}

pub fn grpc_service() -> BenchmarkServiceServer<BenchmarkServiceImpl> {
    let settings = GrpcServerSettings {
        max_decoding_message_size: 4 * 1024 * 1024,
        max_encoding_message_size: 4 * 1024 * 1024,
        request_timeout_secs: 30,
        max_concurrent_connections: 4096,
    };

    BenchmarkServiceServer::new(BenchmarkServiceImpl::default())
        .max_decoding_message_size(settings.max_decoding_message_size)
        .max_encoding_message_size(settings.max_encoding_message_size)
}
