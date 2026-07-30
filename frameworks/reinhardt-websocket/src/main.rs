use httparena_reinhardt_websocket::apps::benchmark::views::serve;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    serve("0.0.0.0:8080").await
}
