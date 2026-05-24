mod auth;
mod config;
mod rasterize;

use axum::{
    extract::{DefaultBodyLimit, Multipart, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::{net::SocketAddr, sync::Arc, time::Instant};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

use crate::{
    auth::require_api_key,
    config::AppConfig,
    rasterize::{rasterize_pdf, RasterizeOptions, RasterizeResponse},
};

#[derive(Clone)]
struct AppState {
    config: Arc<AppConfig>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}

#[derive(serde::Deserialize, Default)]
struct RasterizeQuery {
    dpi: Option<u32>,
    max_pages: Option<u32>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "authentiq_pdf_service=info,tower_http=info".into()),
        )
        .init();

    let config = Arc::new(AppConfig::from_env());
    let state = AppState {
        config: Arc::clone(&config),
    };

    let max_body = config.max_upload_bytes;

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/rasterize", post(rasterize_handler))
        .layer(DefaultBodyLimit::max(max_body))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = format!("0.0.0.0:{}", config.port).parse().expect("invalid port");
    info!(%addr, max_upload_mb = max_body / 1024 / 1024, "authentiq-pdf-service starting");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");
    axum::serve(listener, app).await.expect("server error");
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "authentiq-pdf-service",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn rasterize_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<RasterizeQuery>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    if let Err(resp) = require_api_key(&headers, &state.config.api_key) {
        return resp.into_response();
    }

    let mut pdf_bytes: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_lowercase();
        if name == "file" || name == "pdf" {
            match field.bytes().await {
                Ok(bytes) => {
                    pdf_bytes = Some(bytes.to_vec());
                    break;
                }
                Err(err) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        &format!("Lecture du fichier impossible: {err}"),
                    );
                }
            }
        }
    }

    let pdf_bytes = match pdf_bytes {
        Some(bytes) if !bytes.is_empty() => bytes,
        _ => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Champ multipart « file » manquant ou vide.",
            );
        }
    };

    let started = Instant::now();
    let options = RasterizeOptions {
        dpi: query.dpi.unwrap_or(state.config.default_dpi).clamp(72, 300),
        max_pages: query
            .max_pages
            .unwrap_or(state.config.max_pages)
            .clamp(1, state.config.max_pages),
    };

    match rasterize_pdf(&pdf_bytes, &options, &state.config.pdftoppm_bin) {
        Ok(pages) => {
            let processing_ms = started.elapsed().as_millis() as u64;
            let response = RasterizeResponse {
                status: "success",
                page_count: pages.len(),
                pages,
                meta: rasterize::ResponseMeta {
                    dpi: options.dpi,
                    processing_ms,
                },
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => error_response(StatusCode::UNPROCESSABLE_ENTITY, &err.to_string()),
    }
}

fn error_response(status: StatusCode, message: &str) -> axum::response::Response {
    (
        status,
        Json(serde_json::json!({
            "status": "error",
            "message": message,
        })),
    )
        .into_response()
}
