//! Shared benchmark state and transport adapter.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use reinhardt::http::{Handler, Middleware, Request, Response};
use reinhardt::utils::cache::InMemoryCache;
use reinhardt::{DatabaseConnection, ServerRouter};
use reinhardt_middleware::{BrotliMiddleware, GZipMiddleware};

use super::serializers::DatasetItem;

/// Process-wide data shared by the macro-registered benchmark views.
pub struct ArenaState {
    dataset: Vec<DatasetItem>,
    static_dir: PathBuf,
    database: Option<DatabaseConnection>,
    crud_cache: InMemoryCache,
}

impl ArenaState {
    pub fn new(dataset: Vec<DatasetItem>, static_dir: PathBuf) -> Self {
        Self {
            dataset,
            static_dir,
            database: None,
            crud_cache: InMemoryCache::new(),
        }
    }

    pub fn with_database(mut self, database: DatabaseConnection) -> Self {
        self.database = Some(database);
        self
    }

    pub fn dataset(&self) -> &[DatasetItem] {
        &self.dataset
    }

    pub fn static_dir(&self) -> &Path {
        &self.static_dir
    }

    pub fn database(&self) -> Option<&DatabaseConnection> {
        self.database.as_ref()
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

/// Adapts the macro-registered application router to the benchmark listeners.
#[derive(Clone)]
pub struct ArenaRouter {
    router: Arc<ServerRouter>,
    gzip: Arc<GZipMiddleware>,
    brotli: Arc<BrotliMiddleware>,
}

impl ArenaRouter {
    pub fn new() -> Self {
        Self {
            router: Arc::new(super::urls::server_url_patterns()),
            gzip: Arc::new(GZipMiddleware::new()),
            brotli: Arc::new(BrotliMiddleware::new()),
        }
    }
}

#[async_trait]
impl Handler for ArenaRouter {
    async fn handle(&self, request: Request) -> reinhardt::http::Result<Response> {
        let accepted = request
            .headers
            .get(hyper::header::ACCEPT_ENCODING)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let inner: Arc<dyn Handler> = self.router.clone();

        if accepted.contains("br") {
            self.brotli.process(request, inner).await
        } else if accepted.contains("gzip") {
            self.gzip.process(request, inner).await
        } else {
            self.router.handle(request).await
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
