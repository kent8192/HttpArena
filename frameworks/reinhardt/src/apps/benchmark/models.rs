use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone)]
pub struct Rating {
    pub score: i64,
    pub count: i64,
}

#[derive(Deserialize, Clone)]
pub struct DatasetItem {
    pub id: i64,
    pub name: String,
    pub category: String,
    pub price: i64,
    pub quantity: i64,
    pub active: bool,
    pub tags: Vec<String>,
    pub rating: Rating,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RatingOut {
    pub score: i64,
    pub count: i64,
}

#[derive(Serialize)]
pub struct ProcessedItem<'a> {
    pub id: i64,
    pub name: &'a str,
    pub category: &'a str,
    pub price: i64,
    pub quantity: i64,
    pub active: bool,
    pub tags: &'a [String],
    pub rating: RatingOut,
    pub total: i64,
}

#[derive(Serialize)]
pub struct JsonResponse<'a> {
    pub items: Vec<ProcessedItem<'a>>,
    pub count: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DbItem {
    pub id: i64,
    pub name: String,
    pub category: String,
    pub price: i64,
    pub quantity: i64,
    pub active: bool,
    pub tags: serde_json::Value,
    pub rating: RatingOut,
}

#[derive(Serialize)]
pub struct DbResponse {
    pub items: Vec<DbItem>,
    pub count: usize,
}

#[derive(Deserialize, Serialize)]
pub struct CrudCreate {
    pub id: i64,
    pub name: String,
    pub category: String,
    pub price: i64,
    pub quantity: i64,
    #[serde(default)]
    pub active: bool,
    #[serde(default = "empty_tags")]
    pub tags: serde_json::Value,
}

#[derive(Deserialize)]
pub struct CrudUpdate {
    pub name: Option<String>,
    pub category: Option<String>,
    pub price: Option<i64>,
    pub quantity: Option<i64>,
}

fn empty_tags() -> serde_json::Value {
    serde_json::Value::Array(Vec::new())
}
