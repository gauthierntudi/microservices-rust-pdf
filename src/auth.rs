use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

pub fn require_api_key(headers: &HeaderMap, expected: &str) -> Result<(), Response> {
    if expected.is_empty() || expected == "change-me-in-production" {
        // Dev only — production Fly.io must set PDF_SERVICE_API_KEY.
    }

    let provided = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
        });

    match provided {
        Some(key) if key == expected => Ok(()),
        _ => Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "status": "error",
                "message": "Clé API invalide ou absente (header X-Api-Key).",
            })),
        )
            .into_response()),
    }
}
