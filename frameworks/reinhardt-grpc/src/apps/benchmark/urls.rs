//! URL configuration for the benchmark app.

use reinhardt::ServerRouter;
use reinhardt::grpc::{GrpcRouter, GrpcServerSettings};
use reinhardt::urls::prelude::UnifiedRouter;

use crate::benchmark::benchmark_service_server::BenchmarkServiceServer;

use super::views::BenchmarkServiceImpl;

pub fn url_patterns() -> UnifiedRouter {
    UnifiedRouter::new()
        .server(|server| server.mount("/", ServerRouter::new()))
        .grpc(|grpc| grpc.merge(grpc_services()))
}

pub fn grpc_services() -> GrpcRouter {
    let settings = GrpcServerSettings {
        max_decoding_message_size: 4 * 1024 * 1024,
        max_encoding_message_size: 4 * 1024 * 1024,
        request_timeout_secs: 30,
        max_concurrent_connections: 4096,
    };

    GrpcRouter::new().service(
        BenchmarkServiceServer::new(BenchmarkServiceImpl::default())
            .max_decoding_message_size(settings.max_decoding_message_size)
            .max_encoding_message_size(settings.max_encoding_message_size),
    )
}
