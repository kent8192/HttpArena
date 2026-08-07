//! Macro-registered HTTP endpoints for the benchmark application.

use std::path::{Component, Path as FsPath};

use bytes::Bytes;
use reinhardt::db::orm::Model;
use reinhardt::http::{Request, Response, ViewResult};
use reinhardt::utils::cache::Cache;
use reinhardt::{Json, Path, StatusCode, get, post, put};
use reinhardt_db::backends::QueryValue;
use serde::Serialize;

use super::models::{Fortune, Item};
use super::serializers::{
    CrudCreate, CrudUpdate, DbItem, DbResponse, JsonResponse, ProcessedItem, RatingOut,
};
use super::services::state;

#[get("/pipeline", name = "pipeline")]
pub async fn pipeline() -> ViewResult<Response> {
    Ok(text_response(StatusCode::OK, "ok"))
}

#[get("/baseline11", name = "baseline11")]
pub async fn baseline11(request: Request) -> ViewResult<Response> {
    Ok(sum_response(&request, 0))
}

#[post("/baseline11", name = "baseline11-post")]
pub async fn baseline11_post(request: Request) -> ViewResult<Response> {
    let body_value = std::str::from_utf8(request.body())
        .ok()
        .and_then(|body| body.trim().parse::<i64>().ok())
        .unwrap_or(0);
    Ok(sum_response(&request, body_value))
}

#[get("/baseline2", name = "baseline2")]
pub async fn baseline2(request: Request) -> ViewResult<Response> {
    Ok(sum_response(&request, 0))
}

#[post("/upload", name = "upload")]
pub async fn upload(request: Request) -> ViewResult<Response> {
    Ok(text_response(
        StatusCode::OK,
        request.body().len().to_string(),
    ))
}

#[get("/json/{count}", name = "json")]
pub async fn json_endpoint(Path(count): Path<usize>, request: Request) -> ViewResult<Response> {
    let multiplier = query_i64(&request, "m").max(1);
    let count = count.min(state().dataset().len());
    let items = state()
        .dataset()
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
    Ok(json(StatusCode::OK, &JsonResponse { items, count }))
}

#[get("/static/{path}", name = "static")]
pub async fn static_file(Path(path): Path<String>) -> ViewResult<Response> {
    let Some(path) = safe_static_path(state().static_dir(), &path) else {
        return Ok(text_response(StatusCode::NOT_FOUND, "Not Found"));
    };

    match tokio::fs::read(&path).await {
        Ok(body) => Ok(response(
            StatusCode::OK,
            content_type(path.to_string_lossy().as_ref()),
            body,
        )),
        Err(_) => Ok(text_response(StatusCode::NOT_FOUND, "Not Found")),
    }
}

#[get("/async-db", name = "async-db")]
pub async fn async_db(request: Request) -> ViewResult<Response> {
    let min = query_i64_default(&request, "min", 10);
    let max = query_i64_default(&request, "max", 50);
    let limit = query_i64_default(&request, "limit", 50).clamp(1, 50) as usize;
    let Some(database) = state().database() else {
        return Ok(json(
            StatusCode::SERVICE_UNAVAILABLE,
            &DbResponse {
                items: Vec::new(),
                count: 0,
            },
        ));
    };

    match Item::objects()
        .filter(Item::field_price().gte(min))
        .filter(Item::field_price().lte(max))
        .limit(limit)
        .all_with_db(database)
        .await
    {
        Ok(items) => {
            let items: Vec<_> = items.iter().map(DbItem::from).collect();
            let count = items.len();
            Ok(json(StatusCode::OK, &DbResponse { items, count }))
        }
        Err(error) => Ok(database_error_response(error)),
    }
}

#[get("/crud/items", name = "crud-list")]
pub async fn crud_list(request: Request) -> ViewResult<Response> {
    let category = request
        .query_params
        .get("category")
        .cloned()
        .unwrap_or_else(|| "electronics".to_string());
    let page = query_i64_default(&request, "page", 1).max(1);
    let limit = query_i64_default(&request, "limit", 10).clamp(1, 50);
    let offset = (page - 1) * limit;
    let Some(database) = state().database() else {
        return Ok(text_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "Database unavailable",
        ));
    };

    match Item::objects()
        .filter(Item::field_category().eq(category))
        .order_by(&["id"])
        .limit(limit as usize)
        .offset(offset as usize)
        .all_with_db(database)
        .await
    {
        Ok(items) => {
            let items: Vec<_> = items.iter().map(DbItem::from).collect();
            Ok(json(
                StatusCode::OK,
                &serde_json::json!({
                    "total": items.len(),
                    "items": items,
                    "page": page,
                    "limit": limit,
                }),
            ))
        }
        Err(error) => Ok(database_error_response(error)),
    }
}

#[get("/crud/items/{id}", name = "crud-read")]
pub async fn crud_read(Path(id): Path<i64>) -> ViewResult<Response> {
    let cache_key = format!("crud:{id}");
    if let Ok(Some(body)) = state().crud_cache().get::<Vec<u8>>(&cache_key).await {
        return Ok(response(StatusCode::OK, "application/json", body).with_header("X-Cache", "HIT"));
    }
    let Some(database) = state().database() else {
        return Ok(text_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "Database unavailable",
        ));
    };

    match Item::objects().get(id).all_with_db(database).await {
        Ok(items) => match items.first() {
            Some(item) => {
                let body = serde_json::to_vec(&DbItem::from(item)).unwrap_or_default();
                let _ = state().crud_cache().set(&cache_key, &body, None).await;
                Ok(response(StatusCode::OK, "application/json", body)
                    .with_header("X-Cache", "MISS"))
            }
            None => Ok(text_response(StatusCode::NOT_FOUND, "Not Found")),
        },
        Err(error) => Ok(database_error_response(error)),
    }
}

#[post("/crud/items", name = "crud-create")]
pub async fn crud_create(Json(input): Json<CrudCreate>) -> ViewResult<Response> {
    let Some(database) = state().database() else {
        return Ok(text_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "Database unavailable",
        ));
    };
    let item = Item::build()
        .id(input.id)
        .name(input.name)
        .category(input.category)
        .price(input.price)
        .quantity(input.quantity)
        .active(input.active)
        .tags(Some(input.tags.to_string()))
        .rating_score(0)
        .rating_count(0)
        .finish();

    // The 0.3.5 ORM's PostgreSQL binder does not cast JSONB parameters. Keep
    // the domain value in `Item`, while supplying the required database cast.
    match database
        .execute(
            "INSERT INTO items (id, name, category, price, quantity, active, tags, rating_score, rating_count) \
             VALUES ($1, $2, $3, $4, $5, $6, CAST($7 AS JSONB), $8, $9) \
             ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, category = EXCLUDED.category, \
             price = EXCLUDED.price, quantity = EXCLUDED.quantity, active = EXCLUDED.active, tags = EXCLUDED.tags",
            vec![
                QueryValue::Int(item.id),
                QueryValue::String(item.name.clone()),
                QueryValue::String(item.category.clone()),
                QueryValue::Int(item.price),
                QueryValue::Int(item.quantity),
                QueryValue::Bool(item.active),
                QueryValue::String(input.tags.to_string()),
                QueryValue::Int(item.rating_score),
                QueryValue::Int(item.rating_count),
            ],
        )
        .await
    {
        Ok(_) => {
            let _ = state()
                .crud_cache()
                .delete(&format!("crud:{}", item.id))
                .await;
            Ok(json(StatusCode::CREATED, &DbItem::from(&item)))
        }
        Err(error) => Ok(database_error_response(error)),
    }
}

#[put("/crud/items/{id}", name = "crud-update")]
pub async fn crud_update(
    Path(id): Path<i64>,
    Json(input): Json<CrudUpdate>,
) -> ViewResult<Response> {
    let Some(database) = state().database() else {
        return Ok(text_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "Database unavailable",
        ));
    };
    let items = match Item::objects().get(id).all_with_db(database).await {
        Ok(items) => items,
        Err(error) => return Ok(database_error_response(error)),
    };
    let Some(mut item) = items.into_iter().next() else {
        return Ok(text_response(StatusCode::NOT_FOUND, "Not Found"));
    };

    if let Some(name) = input.name {
        item.name = name;
    }
    if let Some(category) = input.category {
        item.category = category;
    }
    if let Some(price) = input.price {
        item.price = price;
    }
    if let Some(quantity) = input.quantity {
        item.quantity = quantity;
    }
    match database
        .execute(
            "UPDATE items SET name = $2, category = $3, price = $4, quantity = $5 WHERE id = $1",
            vec![
                QueryValue::Int(id),
                QueryValue::String(item.name.clone()),
                QueryValue::String(item.category.clone()),
                QueryValue::Int(item.price),
                QueryValue::Int(item.quantity),
            ],
        )
        .await
    {
        Ok(_) => {
            let _ = state().crud_cache().delete(&format!("crud:{id}")).await;
            Ok(json(StatusCode::OK, &DbItem::from(&item)))
        }
        Err(error) => Ok(database_error_response(error)),
    }
}

#[get("/fortunes", name = "fortunes")]
pub async fn fortunes() -> ViewResult<Response> {
    let Some(database) = state().database() else {
        return Ok(text_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "Database unavailable",
        ));
    };
    let mut fortunes = match Fortune::objects().all().all_with_db(database).await {
        Ok(fortunes) => fortunes,
        Err(error) => return Ok(database_error_response(error)),
    };
    fortunes.push(
        Fortune::build()
            .id(0)
            .message("Additional fortune added at request time.")
            .finish(),
    );
    fortunes.sort_by(|left, right| left.message.as_bytes().cmp(right.message.as_bytes()));

    let mut html = String::with_capacity(32 * 1024);
    html.push_str("<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Fortunes</title>");
    html.push_str("<style>body{font-family:system-ui;margin:2rem}table{border-collapse:collapse;width:100%}th,td{border:1px solid #bbb;padding:.45rem .7rem;text-align:left}tbody tr:nth-child(even){background:#f4f4f4}</style></head><body>");
    html.push_str(
        "<h1>Fortunes</h1><table><thead><tr><th>id</th><th>message</th></tr></thead><tbody>",
    );
    for fortune in fortunes {
        html.push_str("<tr><td class=\"fortune-id\">");
        html.push_str(&fortune.id.to_string());
        html.push_str("</td><td class=\"fortune-message\">");
        escape_html_into(&fortune.message, &mut html);
        html.push_str("</td></tr>");
    }
    html.push_str("</tbody></table></body></html>");
    Ok(response(StatusCode::OK, "text/html; charset=utf-8", html))
}

fn sum_response(request: &Request, body_value: i64) -> Response {
    text_response(
        StatusCode::OK,
        (query_i64(request, "a") + query_i64(request, "b") + body_value).to_string(),
    )
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

fn json<T: Serialize>(status: StatusCode, value: &T) -> Response {
    let body = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    response(status, "application/json", body)
}

fn database_error_response(error: impl std::fmt::Display) -> Response {
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

fn safe_static_path(root: &FsPath, relative: &str) -> Option<std::path::PathBuf> {
    let relative = FsPath::new(relative);
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
    match FsPath::new(path)
        .extension()
        .and_then(|value| value.to_str())
    {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_paths_reject_traversal() {
        let root = FsPath::new("/data/static");
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
