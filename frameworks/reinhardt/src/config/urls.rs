//! URL configuration for httparena_reinhardt project (RESTful)
//!
//! The `routes` function defines all URL patterns for this project.

use reinhardt::routes;
use reinhardt::urls::prelude::UnifiedRouter;

#[routes]
pub fn routes() -> UnifiedRouter {
    UnifiedRouter::new()
        .server(|server| server.mount("/", crate::apps::benchmark::urls::server_url_patterns()))
        .with_middleware(crate::apps::benchmark::services::CompressionMiddleware::new())
}
