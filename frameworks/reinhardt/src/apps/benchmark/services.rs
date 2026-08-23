//! Shared benchmark state and transport adapter.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use reinhardt::http::{Handler, Middleware, Request, Response};
use reinhardt::utils::cache::InMemoryCache;
use reinhardt_middleware::{BrotliMiddleware, GZipMiddleware};

use super::serializers::DatasetItem;

/// Process-wide data shared by the macro-registered benchmark views.
pub struct ArenaState {
    dataset: Vec<DatasetItem>,
    static_dir: PathBuf,
    crud_cache: InMemoryCache,
}

impl ArenaState {
    pub fn new(dataset: Vec<DatasetItem>, static_dir: PathBuf) -> Self {
        Self {
            dataset,
            static_dir,
            crud_cache: InMemoryCache::new(),
        }
    }

    pub fn dataset(&self) -> &[DatasetItem] {
        &self.dataset
    }

    pub fn static_dir(&self) -> &Path {
        &self.static_dir
    }

    pub fn crud_cache(&self) -> &InMemoryCache {
        &self.crud_cache
    }
}

static ARENA_STATE: OnceLock<ArenaState> = OnceLock::new();

pub fn initialize_state(state: ArenaState) -> Result<(), ArenaState> {
    ARENA_STATE.set(state)
}

pub fn state() -> &'static ArenaState {
    ARENA_STATE
        .get()
        .expect("benchmark state must be initialized before serving requests")
}

pub async fn database() -> Option<reinhardt::DatabaseConnection> {
    reinhardt_db::orm::get_connection().await.ok()
}

/// Applies the benchmark's negotiated response compression.
#[derive(Clone)]
pub struct CompressionMiddleware {
    gzip: Arc<GZipMiddleware>,
    brotli: Arc<BrotliMiddleware>,
}

impl CompressionMiddleware {
    pub fn new() -> Self {
        Self {
            gzip: Arc::new(GZipMiddleware::new()),
            brotli: Arc::new(BrotliMiddleware::new()),
        }
    }
}

#[async_trait]
impl Middleware for CompressionMiddleware {
    async fn process(
        &self,
        request: Request,
        next: Arc<dyn Handler>,
    ) -> reinhardt::http::Result<Response> {
        let accepted = request
            .headers
            .get("accept-encoding")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_ascii_lowercase();

        if accepted.contains("br") {
            self.brotli.process(request, next).await
        } else if accepted.contains("gzip") {
            self.gzip.process(request, next).await
        } else {
            next.handle(request).await
        }
    }
}

pub fn load_dataset() -> Vec<DatasetItem> {
    let path = std::env::var("DATASET_PATH").unwrap_or_else(|_| "/data/dataset.json".to_string());
    std::fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}
