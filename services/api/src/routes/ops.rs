//! Operations metrics in Prometheus text format.
//!
//! Off unless `TRACE_OPS_TOKEN` is set. Requires `Authorization: Bearer <token>`.
//! Not part of the public API contract (docs/openapi.snapshot.json).
//! Everything is read from the database; nothing is estimated.

use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::Row;

use crate::{error::ApiError, state::AppState};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/ops/metrics", web::get().to(metrics));
}

fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn authorized(req: &HttpRequest, token: &str) -> bool {
    req.headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|t| same(t.as_bytes(), token.as_bytes()))
}

#[derive(Default)]
pub struct Exposition {
    out: String,
}

fn esc(v: &str) -> String {
    v.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

impl Exposition {
    pub fn help(&mut self, name: &str, text: &str) {
        self.out
            .push_str(&format!("# HELP {name} {text}\n# TYPE {name} gauge\n"));
    }
    pub fn gauge(&mut self, name: &str, labels: &[(&str, &str)], value: f64) {
        let l = if labels.is_empty() {
            String::new()
        } else {
            let inner: Vec<String> = labels
                .iter()
                .map(|(k, v)| format!("{k}=\"{}\"", esc(v)))
                .collect();
            format!("{{{}}}", inner.join(","))
        };
        self.out.push_str(&format!("{name}{l} {value}\n"));
    }
    pub fn text(self) -> String {
        self.out
    }
}

async fn metrics(req: HttpRequest, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    // Disabled endpoints look like missing routes, not like a locked door.
    let Some(token) = state.ops_token.as_deref() else {
        return Ok(crate::error::not_found(req.clone()).await);
    };
    if !authorized(&req, token) {
        return Err(ApiError::Unauthorized);
    }
    let db = |e: sqlx::Error| ApiError::Internal(e.into());
    let mut x = Exposition::default();

    x.help("trace_jobs", "Queue jobs by kind and status.");
    for r in sqlx::query("SELECT kind, status, count(*) AS n FROM worker_jobs GROUP BY kind, status ORDER BY kind, status")
        .fetch_all(&state.pool).await.map_err(db)?
    {
        x.gauge("trace_jobs", &[("kind", &r.get::<String, _>("kind")), ("status", &r.get::<String, _>("status"))], r.get::<i64, _>("n") as f64);
    }

    x.help(
        "trace_oldest_due_job_age_seconds",
        "Age of the oldest queued job that is already due. Grows when workers stall.",
    );
    let age: Option<f64> = sqlx::query_scalar(
        "SELECT EXTRACT(EPOCH FROM (now() - min(run_after)))::float8 FROM worker_jobs WHERE status = 'queued' AND run_after <= now()",
    ).fetch_one(&state.pool).await.map_err(db)?;
    x.gauge(
        "trace_oldest_due_job_age_seconds",
        &[],
        age.unwrap_or(0.0).max(0.0),
    );

    x.help("trace_ingest_last_scan_age_seconds", "Seconds since a protocol's last successful scan. Only protocols that are enabled or partial.");
    for r in sqlx::query(
        "SELECT chain, protocol, EXTRACT(EPOCH FROM (now() - last_successful_scan_at))::float8 AS age
         FROM protocols WHERE support_status IN ('enabled','partial') AND last_successful_scan_at IS NOT NULL ORDER BY chain, protocol",
    ).fetch_all(&state.pool).await.map_err(db)? {
        x.gauge("trace_ingest_last_scan_age_seconds", &[("chain", &r.get::<String, _>("chain")), ("protocol", &r.get::<String, _>("protocol"))], r.get::<f64, _>("age"));
    }

    x.help(
        "trace_ingest_enabled_never_scanned",
        "Enabled or partial protocols that have never completed a scan.",
    );
    let never: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM protocols WHERE support_status IN ('enabled','partial') AND last_successful_scan_at IS NULL",
    ).fetch_one(&state.pool).await.map_err(db)?;
    x.gauge("trace_ingest_enabled_never_scanned", &[], never as f64);

    x.help(
        "trace_ingest_cursor_height",
        "Last block or slot handled per protocol.",
    );
    for r in sqlx::query("SELECT chain, protocol, height FROM ingestion_cursors WHERE height IS NOT NULL ORDER BY chain, protocol")
        .fetch_all(&state.pool).await.map_err(db)?
    {
        x.gauge("trace_ingest_cursor_height", &[("chain", &r.get::<String, _>("chain")), ("protocol", &r.get::<String, _>("protocol"))], r.get::<i64, _>("height") as f64);
    }

    x.help(
        "trace_swaps",
        "Stored real swaps by chain and finality (demo rows excluded).",
    );
    for r in sqlx::query("SELECT chain, finality, count(*) AS n FROM normalized_swaps WHERE NOT demo GROUP BY chain, finality ORDER BY chain, finality")
        .fetch_all(&state.pool).await.map_err(db)?
    {
        x.gauge("trace_swaps", &[("chain", &r.get::<String, _>("chain")), ("finality", &r.get::<String, _>("finality"))], r.get::<i64, _>("n") as f64);
    }

    x.help(
        "trace_ingest_gaps",
        "Audited ingestion gaps (ranges that were not read).",
    );
    let gaps: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_events WHERE action = 'ingest_gap'")
            .fetch_one(&state.pool)
            .await
            .map_err(db)?;
    x.gauge("trace_ingest_gaps", &[], gaps as f64);

    x.help(
        "trace_price_latest_age_seconds",
        "Age of the newest real price point. Absent when there are none.",
    );
    let price: Option<f64> = sqlx::query_scalar(
        "SELECT EXTRACT(EPOCH FROM (now() - max(observed_at)))::float8 FROM price_points WHERE NOT demo",
    ).fetch_one(&state.pool).await.map_err(db)?;
    if let Some(p) = price {
        x.gauge("trace_price_latest_age_seconds", &[], p);
    }

    Ok(HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(x.text()))
}
