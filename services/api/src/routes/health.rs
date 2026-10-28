use actix_web::{web, HttpRequest, HttpResponse};
use serde_json::json;

use crate::{error::envelope, state::AppState};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/health/live", web::get().to(live))
        .route("/health/ready", web::get().to(ready));
}

/// Process is up. Does not touch dependencies.
async fn live() -> HttpResponse {
    HttpResponse::Ok().json(json!({ "status": "live" }))
}

/// Process and required dependencies (PostgreSQL) are usable.
async fn ready(req: HttpRequest, state: web::Data<AppState>) -> HttpResponse {
    match trace_database::ping(&state.pool).await {
        Ok(()) => HttpResponse::Ok().json(json!({ "status": "ready", "database": "ok" })),
        Err(e) => {
            tracing::warn!(error = %e, "readiness check failed");
            envelope(
                &req,
                actix_web::http::StatusCode::SERVICE_UNAVAILABLE,
                "dependency_unavailable",
                "Database is not reachable.",
                Some(json!({ "database": "unavailable" })),
            )
        }
    }
}
