#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    httparena_reinhardt_grpc::config::urls::serve_grpc().await
}
