//! HttpArena benchmark application.

use reinhardt::app_config;

pub mod admin;
pub mod models;
pub mod serializers;
pub mod services;
pub mod tests;
pub mod urls;
pub mod views;

pub use services::{ArenaRouter, ArenaState, initialize_state, load_dataset};

#[app_config(name = "benchmark", label = "benchmark")]
pub struct BenchmarkConfig;
