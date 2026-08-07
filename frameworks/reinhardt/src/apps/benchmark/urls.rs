//! URL configuration for the benchmark app.

use reinhardt::ServerRouter;

use super::views;

/// Register every benchmark endpoint with the framework router.
pub fn server_url_patterns() -> ServerRouter {
    ServerRouter::new()
        .endpoint(views::pipeline)
        .endpoint(views::baseline11)
        .endpoint(views::baseline11_post)
        .endpoint(views::baseline2)
        .endpoint(views::upload)
        .endpoint(views::json_endpoint)
        .endpoint(views::static_file)
        .endpoint(views::async_db)
        .endpoint(views::crud_list)
        .endpoint(views::crud_read)
        .endpoint(views::crud_create)
        .endpoint(views::crud_update)
        .endpoint(views::fortunes)
}
