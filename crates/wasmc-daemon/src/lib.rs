use std::sync::Arc;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use wasmc_cache::{CacheStatus, CompilationCache};
use wasmc_diagnostics::Diagnostic;
use wasmc_driver::{CompilationDriver, CompilerOptions, SourceInput};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonCompileSource {
    pub path: String,
    pub content_utf8: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonMemoryConfig {
    #[serde(default = "default_stack_size")]
    pub shadow_stack_bytes: u32,
    #[serde(default = "default_true")]
    pub emit_stack_overflow_check: bool,
    #[serde(default = "default_true")]
    pub bundle_default_allocator: bool,
}

fn default_stack_size() -> u32 {
    1048576
}

fn default_true() -> bool {
    true
}

impl Default for DaemonMemoryConfig {
    fn default() -> Self {
        Self {
            shadow_stack_bytes: 1048576,
            emit_stack_overflow_check: true,
            bundle_default_allocator: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonCompileRequest {
    #[serde(default)]
    pub project_id: String,
    #[serde(default = "default_target")]
    pub target: String,
    #[serde(default = "default_opt_level")]
    pub opt_level: String,
    #[serde(default)]
    pub debug: bool,
    #[serde(default)]
    pub memory_config: DaemonMemoryConfig,
    pub sources: Vec<DaemonCompileSource>,
}

fn default_target() -> String {
    "wasm32-unknown-web".to_string()
}

fn default_opt_level() -> String {
    "O2".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonMemoryLayoutResponse {
    pub null_guard_end: u32,
    pub static_data_end: u32,
    pub stack_limit: u32,
    pub stack_pointer_init: u32,
    pub heap_base: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonArtifactsResponse {
    pub wasm_base64: String,
    pub wasm_blake3_hash: String,
    pub wasm_size_bytes: usize,
    pub wat_text: Option<String>,
    pub memory_layout: DaemonMemoryLayoutResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonCompileResponse {
    pub status: String,
    pub session_id: String,
    pub duration_ms: u128,
    pub cache_hit: bool,
    pub artifacts: Option<DaemonArtifactsResponse>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonCheckResponse {
    pub status: String,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct AppState {
    pub cache: Option<CompilationCache>,
}

pub fn create_router(cache: Option<CompilationCache>) -> Router {
    let state = Arc::new(AppState { cache });

    Router::new()
        .route("/api/v1/compile", post(handle_compile))
        .route("/api/v1/check", post(handle_check))
        .route("/api/v1/validate", post(handle_check))
        .route("/api/v1/cache/status", get(handle_cache_status))
        .route("/api/v1/health", get(handle_health))
        .with_state(state)
}

async fn handle_health() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "UP" })))
}

async fn handle_cache_status(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if let Some(ref cache) = state.cache {
        match cache.get_status() {
            Ok(status) => (StatusCode::OK, Json(serde_json::to_value(status).unwrap())).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response(),
        }
    } else {
        (
            StatusCode::OK,
            Json(serde_json::to_value(CacheStatus {
                total_units: 0,
                total_functions: 0,
                total_sessions: 0,
                db_size_bytes: 0,
            }).unwrap()),
        )
            .into_response()
    }
}

async fn handle_check(
    Json(payload): Json<DaemonCompileRequest>,
) -> impl IntoResponse {
    let options = CompilerOptions {
        target: payload.target,
        opt_level: payload.opt_level,
        debug: payload.debug,
        shadow_stack_size: payload.memory_config.shadow_stack_bytes,
        emit_stack_check: payload.memory_config.emit_stack_overflow_check,
        bundle_allocator: payload.memory_config.bundle_default_allocator,
        ..Default::default()
    };

    let driver = CompilationDriver::new(options);
    let sources: Vec<SourceInput> = payload
        .sources
        .into_iter()
        .map(|s| SourceInput {
            path: s.path,
            content: s.content_utf8,
        })
        .collect();

    match driver.check(&sources) {
        Ok(sink) => (
            StatusCode::OK,
            Json(DaemonCheckResponse {
                status: "SUCCESS".to_string(),
                diagnostics: sink.into_diagnostics(),
            }),
        ),
        Err(sink) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(DaemonCheckResponse {
                status: "FAILED".to_string(),
                diagnostics: sink.into_diagnostics(),
            }),
        ),
    }
}

async fn handle_compile(
    Json(payload): Json<DaemonCompileRequest>,
) -> impl IntoResponse {
    let session_id = blake3::hash(format!("{}:{}", payload.project_id, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()).as_bytes()).to_hex().to_string();

    let options = CompilerOptions {
        target: payload.target,
        opt_level: payload.opt_level,
        debug: payload.debug,
        shadow_stack_size: payload.memory_config.shadow_stack_bytes,
        emit_stack_check: payload.memory_config.emit_stack_overflow_check,
        bundle_allocator: payload.memory_config.bundle_default_allocator,
        emit_wat: true,
        ..Default::default()
    };

    let driver = CompilationDriver::new(options);
    let sources: Vec<SourceInput> = payload
        .sources
        .into_iter()
        .map(|s| SourceInput {
            path: s.path,
            content: s.content_utf8,
        })
        .collect();

    match driver.compile(&sources) {
        Ok(artifacts) => {
            // Encode binary to base64 manually without third-party base64 crate
            const B64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let mut b64 = String::new();
            for chunk in artifacts.wasm_bytes.chunks(3) {
                let b0 = chunk[0];
                let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
                let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

                b64.push(B64_CHARS[(b0 >> 2) as usize] as char);
                b64.push(B64_CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
                if chunk.len() > 1 {
                    b64.push(B64_CHARS[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
                } else {
                    b64.push('=');
                }
                if chunk.len() > 2 {
                    b64.push(B64_CHARS[(b2 & 0x3F) as usize] as char);
                } else {
                    b64.push('=');
                }
            }

            let resp = DaemonCompileResponse {
                status: "SUCCESS".to_string(),
                session_id,
                duration_ms: artifacts.duration_ms,
                cache_hit: artifacts.cache_hit,
                artifacts: Some(DaemonArtifactsResponse {
                    wasm_base64: b64,
                    wasm_blake3_hash: artifacts.wasm_blake3_hash,
                    wasm_size_bytes: artifacts.wasm_size_bytes,
                    wat_text: artifacts.wat_text,
                    memory_layout: DaemonMemoryLayoutResponse {
                        null_guard_end: artifacts.memory_map.null_guard_end,
                        static_data_end: artifacts.memory_map.static_data_end,
                        stack_limit: artifacts.memory_map.stack_limit,
                        stack_pointer_init: artifacts.memory_map.stack_pointer_init,
                        heap_base: artifacts.memory_map.heap_base,
                    },
                }),
                diagnostics: artifacts.diagnostics,
            };

            (StatusCode::OK, Json(resp))
        }
        Err(sink) => {
            let resp = DaemonCompileResponse {
                status: "FAILED".to_string(),
                session_id,
                duration_ms: 0,
                cache_hit: false,
                artifacts: None,
                diagnostics: sink.into_diagnostics(),
            };
            (StatusCode::UNPROCESSABLE_ENTITY, Json(resp))
        }
    }
}
