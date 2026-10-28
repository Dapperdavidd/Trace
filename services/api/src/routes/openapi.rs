use actix_web::{web, HttpResponse};
use serde_json::json;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/openapi.json", web::get().to(spec));
}

/// Hand-maintained until the surface stabilises. docs/openapi.json is a
/// checked-in snapshot produced from this handler (see tests).
pub fn document() -> serde_json::Value {
    json!({
        "openapi": "3.0.3",
        "info": { "title": "TRACE API", "version": "0.1.0", "description": "Discovery-first crypto research API. Watch-only; no signing, no custody." },
        "servers": [{ "url": "http://localhost:8080" }],
        "paths": {
            "/v1/health/live": { "get": { "summary": "Process liveness", "responses": { "200": { "description": "Process is up" } } } },
            "/v1/health/ready": { "get": { "summary": "Readiness including PostgreSQL", "responses": { "200": { "description": "Ready" }, "503": { "description": "Dependency unavailable", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Error" } } } } } } },
            "/v1/chains": { "get": { "summary": "Chain, protocol and freshness truth", "responses": { "200": { "description": "Coverage registry merged with persisted ingestion state" } } } }
        },
        "components": { "schemas": { "Error": {
            "type": "object",
            "required": ["code", "message", "request_id"],
            "properties": {
                "code": { "type": "string" },
                "message": { "type": "string" },
                "details": { "type": "object" },
                "request_id": { "type": "string" }
            }
        } } }
    })
}

async fn spec() -> HttpResponse {
    HttpResponse::Ok().json(document())
}
