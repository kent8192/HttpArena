//! URL configuration for the benchmark app.

use reinhardt::urls::prelude::UnifiedRouter;

pub fn url_patterns() -> UnifiedRouter {
    UnifiedRouter::new().websocket(|websocket| websocket.merge(super::views::ws_url_patterns()))
}
