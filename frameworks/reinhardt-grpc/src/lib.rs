//! Reinhardt gRPC benchmark project.

pub mod apps;
pub mod config;

pub mod benchmark {
    tonic::include_proto!("benchmark");
}

pub use config::settings::get_settings;
pub use config::urls::routes;
