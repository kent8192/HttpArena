//! Project URL composition.

use reinhardt::routes;
use reinhardt::urls::prelude::UnifiedRouter;

#[routes]
pub fn routes() -> UnifiedRouter {
    UnifiedRouter::new()
}
