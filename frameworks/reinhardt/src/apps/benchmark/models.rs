//! Persistent models for the HttpArena benchmark application.

use reinhardt::prelude::*;

/// The shared `items` table used by the async-database and CRUD profiles.
#[model(app_label = "benchmark", table_name = "items")]
#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct Item {
    #[field(primary_key = true)]
    pub id: i64,

    #[field(max_length = 255)]
    pub name: String,

    #[field(max_length = 255)]
    pub category: String,

    pub price: i64,
    pub quantity: i64,

    #[field(default = false)]
    pub active: bool,

    #[field(max_length = 4096)]
    pub tags: Option<String>,
    pub rating_score: i64,
    pub rating_count: i64,
}

/// The reference fortunes dataset rendered by the fortunes profile.
#[model(app_label = "benchmark", table_name = "fortune")]
#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct Fortune {
    #[field(primary_key = true)]
    pub id: i64,

    #[field(max_length = 255)]
    pub message: String,
}
