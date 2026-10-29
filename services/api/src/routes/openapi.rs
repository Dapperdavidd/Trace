use actix_web::{web, HttpResponse};
use serde_json::{json, Map, Value};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/openapi.json", web::get().to(spec));
}

/// (method, path, summary, needs sign-in)
pub const ROUTES: &[(&str, &str, &str, bool)] = &[
    ("get", "/v1/health/live", "Process liveness", false),
    (
        "get",
        "/v1/health/ready",
        "Readiness including PostgreSQL",
        false,
    ),
    (
        "get",
        "/v1/chains",
        "Chain, protocol and freshness truth",
        false,
    ),
    (
        "post",
        "/v1/auth/google",
        "Exchange a Google ID token for an app session",
        false,
    ),
    (
        "post",
        "/v1/auth/logout",
        "Revoke the current session",
        true,
    ),
    ("get", "/v1/me", "Current account", true),
    ("delete", "/v1/me", "Delete the account and its data", true),
    (
        "get",
        "/v1/me/wallets",
        "Watch-only wallets. No spendable wallet exists",
        true,
    ),
    (
        "get",
        "/v1/watchlists",
        "List watched wallets with monitoring status",
        true,
    ),
    (
        "post",
        "/v1/watchlists",
        "Watch a wallet (idempotent)",
        true,
    ),
    (
        "patch",
        "/v1/watchlists/{id}",
        "Rename or pause a watched wallet",
        true,
    ),
    (
        "delete",
        "/v1/watchlists/{id}",
        "Stop watching a wallet",
        true,
    ),
    ("get", "/v1/alert-preferences", "Alert type switches", true),
    (
        "patch",
        "/v1/alert-preferences",
        "Change alert type switches",
        true,
    ),
    (
        "get",
        "/v1/alerts",
        "Alert feed, newest first, keyset paginated",
        true,
    ),
    (
        "get",
        "/v1/discovery/movers",
        "Top Movers with exclusions reported",
        false,
    ),
    (
        "get",
        "/v1/discovery/wallets",
        "Discovered wallets with decisions and confidence",
        false,
    ),
    (
        "get",
        "/v1/wallets/{chain}/{address}",
        "Wallet overview",
        false,
    ),
    (
        "get",
        "/v1/wallets/{chain}/{address}/trades",
        "Trades credited to the wallet",
        false,
    ),
    (
        "get",
        "/v1/wallets/{chain}/{address}/metrics",
        "PnL metrics and WQS",
        false,
    ),
    (
        "get",
        "/v1/wallets/{chain}/{address}/dna",
        "Trading DNA",
        false,
    ),
    (
        "get",
        "/v1/tokens/{chain}/{address}",
        "Token page with risk and labelled facts and hypotheses",
        false,
    ),
    (
        "get",
        "/v1/tokens/{chain}/{address}/timeline",
        "Token event timeline",
        false,
    ),
    (
        "get",
        "/v1/tokens/compare",
        "Compare two to four tokens",
        false,
    ),
    ("get", "/v1/signals", "Published research signals", false),
    (
        "get",
        "/v1/signals/{id}",
        "Signal with immutable receipt",
        false,
    ),
    (
        "get",
        "/v1/signals/{id}/outcomes",
        "All four horizons including losses and unpriced",
        false,
    ),
    (
        "post",
        "/v1/shadow-strategies",
        "Create a paper strategy",
        true,
    ),
    (
        "get",
        "/v1/shadow-strategies",
        "List your paper strategies",
        true,
    ),
    (
        "get",
        "/v1/shadow-strategies/{id}/results",
        "Paper simulation results",
        true,
    ),
    (
        "get",
        "/v1/briefings",
        "Daily briefing built from stored data",
        false,
    ),
    ("get", "/v1/openapi.json", "This document", false),
];

/// Hand-maintained. The test suite checks every listed route is registered.
pub fn document() -> Value {
    let mut paths: Map<String, Value> = Map::new();
    for (method, path, summary, auth) in ROUTES {
        let mut op = json!({
            "summary": summary,
            "responses": {
                "200": { "description": "OK" },
                "400": { "description": "Invalid input", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Error" } } } }
            }
        });
        if *auth {
            op["security"] = json!([{ "bearer": [] }]);
            op["responses"]["401"] =
                json!({ "description": "Missing, expired or revoked session" });
        }
        paths
            .entry(path.to_string())
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .expect("object")
            .insert(method.to_string(), op);
    }
    json!({
        "openapi": "3.0.3",
        "info": { "title": "TRACE API", "version": "0.1.0", "description": "Discovery-first crypto research API. Watch-only; no signing, no custody." },
        "servers": [{ "url": "http://localhost:8080" }],
        "paths": paths,
        "components": {
            "securitySchemes": { "bearer": { "type": "http", "scheme": "bearer" } },
            "schemas": { "Error": {
                "type": "object",
                "required": ["code", "message", "request_id"],
                "properties": {
                    "code": { "type": "string" },
                    "message": { "type": "string" },
                    "details": { "type": "object" },
                    "request_id": { "type": "string" }
                }
            } }
        }
    })
}

async fn spec() -> HttpResponse {
    HttpResponse::Ok().json(document())
}
