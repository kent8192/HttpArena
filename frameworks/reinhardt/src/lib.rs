//! httparena_reinhardt library
//!
//! This is the main library crate for httparena_reinhardt.

pub mod apps;
pub mod config;

// Re-export commonly used items
pub use config::settings::get_settings;
pub use config::urls::routes;
