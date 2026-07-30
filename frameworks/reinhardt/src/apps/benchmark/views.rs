use std::fmt::Display;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use hyper::{Method, StatusCode};
use reinhardt::http::{Handler, Middleware, Request, Response};
use reinhardt::utils::cache::{Cache, InMemoryCache};
use reinhardt_db::backends::{DatabaseConnection, QueryValue, Row};
use reinhardt_middleware::{BrotliMiddleware, GZipMiddleware};
use serde::Serialize;

use super::models::{
    CrudCreate, CrudUpdate, DatasetItem, DbItem, DbResponse, JsonResponse, ProcessedItem, RatingOut,
};

#[derive(Clone)]
pub struct ArenaHandler {
    dataset: Arc<Vec<DatasetItem>>,
    static_dir: Arc<PathBuf>,
    gzip: Arc<GZipMiddleware>,
    brotli: Arc<BrotliMiddleware>,
    database: Option<DatabaseConnection>,
    crud_cache: InMemoryCache,
}

impl ArenaHandler {
    pub fn new(dataset: Vec<DatasetItem>, static_dir: PathBuf) -> Self {
        Self {
            dataset: Arc::new(dataset),
            static_dir: Arc::new(static_dir),
            gzip: Arc::new(GZipMiddleware::new()),
            brotli: Arc::new(BrotliMiddleware::new()),
            database: None,
            crud_cache: InMemoryCache::new(),
        }
    }

    pub fn with_database(mut self, database: DatabaseConnection) -> Self {
        self.database = Some(database);
        self
    }

    async fn dispatch(&self, request: Request) -> reinhardt::http::Result<Response> {
        let path = request.uri.path().to_string();
        match (request.method.clone(), path.as_str()) {
            (Method::GET, "/pipeline") => Ok(text_response(StatusCode::OK, "ok")),
            (Method::GET, "/baseline11") | (Method::GET, "/baseline2") => {
                let sum = query_i64(&request, "a") + query_i64(&request, "b");
                Ok(text_response(StatusCode::OK, sum.to_string()))
            }
            (Method::POST, "/baseline11") => {
                let body_value = std::str::from_utf8(request.body())
                    .ok()
                    .and_then(|body| body.trim().parse::<i64>().ok())
                    .unwrap_or(0);
                let sum = query_i64(&request, "a") + query_i64(&request, "b") + body_value;
                Ok(text_response(StatusCode::OK, sum.to_string()))
            }
            (Method::POST, "/upload") => Ok(text_response(
                StatusCode::OK,
                request.body().len().to_string(),
            )),
            (Method::GET, path) if path.starts_with("/json/") => {
                let count = path
                    .trim_start_matches("/json/")
                    .parse::<usize>()
                    .unwrap_or(0);
                Ok(self.json_response(count, query_i64(&request, "m").max(1)))
            }
            (Method::GET, path) if path.starts_with("/static/") => {
                Ok(self.static_response(path).await)
            }
            (Method::GET, "/async-db") => Ok(self.async_db_response(&request).await),
            (Method::GET, "/crud/items") => Ok(self.crud_list_response(&request).await),
            (Method::POST, "/crud/items") => Ok(self.crud_create_response(&request).await),
            (Method::GET, path) if path.starts_with("/crud/items/") => {
                Ok(self.crud_read_response(path).await)
            }
            (Method::PUT, path) if path.starts_with("/crud/items/") => {
                Ok(self.crud_update_response(path, &request).await)
            }
            (Method::GET, "/fortunes") => Ok(self.fortunes_response().await),
            _ => Ok(text_response(StatusCode::NOT_FOUND, "Not Found")),
        }
    }

    fn json_response(&self, count: usize, multiplier: i64) -> Response {
        let count = count.min(self.dataset.len());
        let items = self
            .dataset
            .iter()
            .take(count)
            .map(|item| ProcessedItem {
                id: item.id,
                name: &item.name,
                category: &item.category,
                price: item.price,
                quantity: item.quantity,
                active: item.active,
                tags: &item.tags,
                rating: RatingOut {
                    score: item.rating.score,
                    count: item.rating.count,
                },
                total: item.price * item.quantity * multiplier,
            })
            .collect();
        let payload = JsonResponse { items, count };
        let body =
            serde_json::to_vec(&payload).unwrap_or_else(|_| b"{\"items\":[],\"count\":0}".to_vec());
        response(StatusCode::OK, "application/json", body)
    }

    async fn static_response(&self, request_path: &str) -> Response {
        let relative = request_path.trim_start_matches("/static/");
        let Some(path) = safe_static_path(&self.static_dir, relative) else {
            return text_response(StatusCode::NOT_FOUND, "Not Found");
        };
        match tokio::fs::read(path).await {
            Ok(body) => response(StatusCode::OK, content_type(relative), body),
            Err(_) => text_response(StatusCode::NOT_FOUND, "Not Found"),
        }
    }

    async fn async_db_response(&self, request: &Request) -> Response {
        let min = query_i64_default(request, "min", 10);
        let max = query_i64_default(request, "max", 50);
        let limit = query_i64_default(request, "limit", 50).clamp(1, 50);
        let Some(database) = &self.database else {
            return json(
                StatusCode::SERVICE_UNAVAILABLE,
                &DbResponse {
                    items: Vec::new(),
                    count: 0,
                },
            );
        };
        let rows = database
            .fetch_all(
                "SELECT id, name, category, price, quantity, active, tags::text AS tags, \
                 rating_score, rating_count FROM items WHERE price BETWEEN $1 AND $2 LIMIT $3",
                vec![min.into(), max.into(), limit.into()],
            )
            .await;
        match rows {
            Ok(rows) => {
                let items: Vec<_> = rows.iter().filter_map(db_item_from_row).collect();
                let count = items.len();
                json(StatusCode::OK, &DbResponse { items, count })
            }
            Err(error) => database_error_response(error),
        }
    }

    async fn crud_list_response(&self, request: &Request) -> Response {
        let category = request
            .query_params
            .get("category")
            .cloned()
            .unwrap_or_else(|| "electronics".to_string());
        let page = query_i64_default(request, "page", 1).max(1);
        let limit = query_i64_default(request, "limit", 10).clamp(1, 50);
        let offset = (page - 1) * limit;
        let Some(database) = &self.database else {
            return text_response(StatusCode::SERVICE_UNAVAILABLE, "Database unavailable");
        };
        let rows = database
            .fetch_all(
                "SELECT id, name, category, price, quantity, active, tags::text AS tags, \
                 rating_score, rating_count FROM items WHERE category = $1 ORDER BY id LIMIT $2 \
                 OFFSET $3",
                vec![category.into(), limit.into(), offset.into()],
            )
            .await;
        match rows {
            Ok(rows) => {
                let items: Vec<_> = rows.iter().filter_map(db_item_from_row).collect();
                json(
                    StatusCode::OK,
                    &serde_json::json!({
                        "total": items.len(),
                        "items": items,
                        "page": page,
                        "limit": limit
                    }),
                )
            }
            Err(error) => database_error_response(error),
        }
    }

    async fn crud_read_response(&self, path: &str) -> Response {
        let Some(id) = path_id(path) else {
            return text_response(StatusCode::BAD_REQUEST, "Invalid id");
        };
        let cache_key = format!("crud:{id}");
        if let Ok(Some(body)) = self.crud_cache.get::<Vec<u8>>(&cache_key).await {
            return response(StatusCode::OK, "application/json", body)
                .with_header("X-Cache", "HIT");
        }
        let Some(database) = &self.database else {
            return text_response(StatusCode::SERVICE_UNAVAILABLE, "Database unavailable");
        };
        match database
            .fetch_optional(
                "SELECT id, name, category, price, quantity, active, tags::text AS tags, \
                 rating_score, rating_count FROM items WHERE id = $1",
                vec![id.into()],
            )
            .await
        {
            Ok(Some(row)) => {
                let Some(item) = db_item_from_row(&row) else {
                    return text_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Invalid database row",
                    );
                };
                let body = serde_json::to_vec(&item).unwrap_or_default();
                let _ = self.crud_cache.set(&cache_key, &body, None).await;
                response(StatusCode::OK, "application/json", body).with_header("X-Cache", "MISS")
            }
            Ok(None) => text_response(StatusCode::NOT_FOUND, "Not Found"),
            Err(error) => database_error_response(error),
        }
    }

    async fn crud_create_response(&self, request: &Request) -> Response {
        let input: CrudCreate = match serde_json::from_slice(request.body()) {
            Ok(input) => input,
            Err(_) => return text_response(StatusCode::UNPROCESSABLE_ENTITY, "Invalid JSON"),
        };
        let Some(database) = &self.database else {
            return text_response(StatusCode::SERVICE_UNAVAILABLE, "Database unavailable");
        };
        let tags = input.tags.to_string();
        let result = database
            .execute(
                "INSERT INTO items (id, name, category, price, quantity, active, tags, \
                 rating_score, rating_count) VALUES ($1, $2, $3, $4, $5, $6, $7::text::jsonb, 0, 0) \
                 ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, category = EXCLUDED.category, \
                 price = EXCLUDED.price, quantity = EXCLUDED.quantity, active = EXCLUDED.active, \
                 tags = EXCLUDED.tags",
                vec![
                    input.id.into(),
                    input.name.clone().into(),
                    input.category.clone().into(),
                    input.price.into(),
                    input.quantity.into(),
                    input.active.into(),
                    tags.into(),
                ],
            )
            .await;
        match result {
            Ok(_) => {
                let _ = self.crud_cache.delete(&format!("crud:{}", input.id)).await;
                json(
                    StatusCode::CREATED,
                    &serde_json::json!({
                        "id": input.id,
                        "name": input.name,
                        "category": input.category,
                        "price": input.price,
                        "quantity": input.quantity,
                        "active": input.active,
                        "tags": input.tags,
                        "rating": {"score": 0, "count": 0}
                    }),
                )
            }
            Err(error) => database_error_response(error),
        }
    }

    async fn crud_update_response(&self, path: &str, request: &Request) -> Response {
        let Some(id) = path_id(path) else {
            return text_response(StatusCode::BAD_REQUEST, "Invalid id");
        };
        let input: CrudUpdate = match serde_json::from_slice(request.body()) {
            Ok(input) => input,
            Err(_) => return text_response(StatusCode::UNPROCESSABLE_ENTITY, "Invalid JSON"),
        };
        let Some(database) = &self.database else {
            return text_response(StatusCode::SERVICE_UNAVAILABLE, "Database unavailable");
        };
        let current = match database
            .fetch_optional(
                "SELECT id, name, category, price, quantity, active, tags::text AS tags, \
                 rating_score, rating_count FROM items WHERE id = $1",
                vec![id.into()],
            )
            .await
        {
            Ok(Some(row)) => match db_item_from_row(&row) {
                Some(item) => item,
                None => {
                    return text_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Invalid database row",
                    );
                }
            },
            Ok(None) => return text_response(StatusCode::NOT_FOUND, "Not Found"),
            Err(error) => return database_error_response(error),
        };
        let params = vec![
            QueryValue::Int(id),
            QueryValue::String(input.name.unwrap_or(current.name)),
            QueryValue::String(input.category.unwrap_or(current.category)),
            QueryValue::Int(input.price.unwrap_or(current.price)),
            QueryValue::Int(input.quantity.unwrap_or(current.quantity)),
        ];
        match database
            .fetch_optional(
                "UPDATE items SET name = $2, category = $3, price = $4, quantity = $5 \
                 WHERE id = $1 RETURNING id, name, category, price, quantity, active, \
                 tags::text AS tags, rating_score, rating_count",
                params,
            )
            .await
        {
            Ok(Some(row)) => {
                let _ = self.crud_cache.delete(&format!("crud:{id}")).await;
                match db_item_from_row(&row) {
                    Some(item) => json(StatusCode::OK, &item),
                    None => {
                        text_response(StatusCode::INTERNAL_SERVER_ERROR, "Invalid database row")
                    }
                }
            }
            Ok(None) => text_response(StatusCode::NOT_FOUND, "Not Found"),
            Err(error) => database_error_response(error),
        }
    }

    async fn fortunes_response(&self) -> Response {
        let Some(database) = &self.database else {
            return text_response(StatusCode::SERVICE_UNAVAILABLE, "Database unavailable");
        };
        let rows = match database
            .fetch_all("SELECT id, message FROM fortune", Vec::new())
            .await
        {
            Ok(rows) => rows,
            Err(error) => return database_error_response(error),
        };
        let mut fortunes: Vec<(i64, String)> = rows
            .into_iter()
            .filter_map(|row| Some((row.get("id").ok()?, row.get("message").ok()?)))
            .collect();
        fortunes.push((0, "Additional fortune added at request time.".to_string()));
        fortunes.sort_by(|left, right| left.1.as_bytes().cmp(right.1.as_bytes()));
        let mut html = String::with_capacity(32 * 1024);
        html.push_str("<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Fortunes</title>");
        html.push_str("<style>body{font-family:system-ui;margin:2rem}table{border-collapse:collapse;width:100%}th,td{border:1px solid #bbb;padding:.45rem .7rem;text-align:left}tbody tr:nth-child(even){background:#f4f4f4}</style></head><body>");
        html.push_str(
            "<h1>Fortunes</h1><table><thead><tr><th>id</th><th>message</th></tr></thead><tbody>",
        );
        for (id, message) in fortunes {
            html.push_str("<tr><td class=\"fortune-id\">");
            html.push_str(&id.to_string());
            html.push_str("</td><td class=\"fortune-message\">");
            escape_html_into(&message, &mut html);
            html.push_str("</td></tr>");
        }
        html.push_str("</tbody></table></body></html>");
        response(StatusCode::OK, "text/html; charset=utf-8", html)
    }
}

#[async_trait]
impl Handler for ArenaHandler {
    async fn handle(&self, request: Request) -> reinhardt::http::Result<Response> {
        let accepted = request
            .headers
            .get(hyper::header::ACCEPT_ENCODING)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let inner: Arc<dyn Handler> = Arc::new(DispatchHandler(self.clone()));
        if accepted.contains("br") {
            self.brotli.process(request, inner).await
        } else if accepted.contains("gzip") {
            self.gzip.process(request, inner).await
        } else {
            self.dispatch(request).await
        }
    }
}

#[derive(Clone)]
struct DispatchHandler(ArenaHandler);

#[async_trait]
impl Handler for DispatchHandler {
    async fn handle(&self, request: Request) -> reinhardt::http::Result<Response> {
        self.0.dispatch(request).await
    }
}

fn query_i64(request: &Request, key: &str) -> i64 {
    request
        .query_params
        .get(key)
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0)
}

fn query_i64_default(request: &Request, key: &str, default: i64) -> i64 {
    request
        .query_params
        .get(key)
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(default)
}

fn path_id(path: &str) -> Option<i64> {
    path.strip_prefix("/crud/items/")?.parse().ok()
}

fn db_item_from_row(row: &Row) -> Option<DbItem> {
    let tags: String = row.get("tags").ok()?;
    Some(DbItem {
        id: row.get("id").ok()?,
        name: row.get("name").ok()?,
        category: row.get("category").ok()?,
        price: row.get("price").ok()?,
        quantity: row.get("quantity").ok()?,
        active: row.get("active").ok()?,
        tags: serde_json::from_str(&tags).unwrap_or_else(|_| serde_json::Value::Array(Vec::new())),
        rating: RatingOut {
            score: row.get("rating_score").ok()?,
            count: row.get("rating_count").ok()?,
        },
    })
}

fn json<T: Serialize>(status: StatusCode, value: &T) -> Response {
    let body = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    response(status, "application/json", body)
}

fn database_error_response(error: impl Display) -> Response {
    eprintln!("database operation failed: {error}");
    text_response(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
}

fn escape_html_into(input: &str, output: &mut String) {
    for character in input.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(character),
        }
    }
}

fn safe_static_path(root: &Path, relative: &str) -> Option<PathBuf> {
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return None;
    }
    Some(root.join(relative))
}

fn content_type(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|value| value.to_str()) {
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("json") => "application/json",
        Some("html") => "text/html; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        _ => "application/octet-stream",
    }
}

fn text_response(status: StatusCode, body: impl Into<Bytes>) -> Response {
    response(status, "text/plain", body)
}

fn response(status: StatusCode, content_type: &'static str, body: impl Into<Bytes>) -> Response {
    Response::new(status)
        .with_body(body)
        .with_header("Content-Type", content_type)
        .with_header("Server", "reinhardt")
}

pub fn load_dataset() -> Vec<DatasetItem> {
    let path = std::env::var("DATASET_PATH").unwrap_or_else(|_| "/data/dataset.json".to_string());
    std::fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_paths_reject_traversal() {
        let root = Path::new("/data/static");
        assert!(safe_static_path(root, "app.js").is_some());
        assert!(safe_static_path(root, "../dataset.json").is_none());
        assert!(safe_static_path(root, "nested/../app.js").is_none());
    }

    #[test]
    fn static_content_types_match_validator_assets() {
        assert_eq!(content_type("reset.css"), "text/css");
        assert_eq!(content_type("app.js"), "application/javascript");
    }
}
