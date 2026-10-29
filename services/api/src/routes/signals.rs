use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use trace_domain::DataStatus;
use trace_signals::{Outcome, OutcomeKind, Receipt, HORIZONS_SECS};
use uuid::Uuid;

use super::common::*;
use crate::{error::ApiError, state::AppState};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/signals", web::get().to(list))
        .route("/signals/{id}", web::get().to(get))
        .route("/signals/{id}/outcomes", web::get().to(outcomes));
}

#[derive(Deserialize)]
struct ListQuery {
    cursor: Option<String>,
    state: Option<String>,
    limit: Option<u32>,
}

const STATES: [&str; 9] = [
    "detected",
    "evaluated",
    "published_as_research",
    "rejected",
    "watch_only",
    "observation_due",
    "observed",
    "unpriced",
    "invalidated",
];

/// A stored receipt is only shown if its hash still matches. A mismatch is an
/// integrity failure and the content is withheld.
fn verified_receipt(row: &sqlx::postgres::PgRow) -> Result<Receipt, ApiError> {
    let receipt: Receipt = serde_json::from_value(row.get("receipt"))
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("receipt unreadable: {e}")))?;
    let stored: String = row.get("receipt_hash");
    if receipt.seal() != stored {
        tracing::error!(signal = %row.get::<Uuid, _>("id"), "receipt hash mismatch");
        return Err(ApiError::Internal(anyhow::anyhow!(
            "receipt integrity check failed"
        )));
    }
    Ok(receipt)
}

fn summary(row: &sqlx::postgres::PgRow, r: &Receipt) -> Value {
    json!({
        "id": row.get::<Uuid, _>("id"),
        "state": row.get::<String, _>("state"),
        "wallet": { "chain": row.get::<String, _>("wallet_chain"), "address": row.get::<String, _>("wallet_address"), "alias": null },
        "token": token_ref(&row.get::<String, _>("token_chain"), &row.get::<String, _>("token"), None, None),
        "published_at": row.get::<DateTime<Utc>, _>("published_at"),
        "risk_verdict": r.risk_verdict,
        "scores": { "wqs": r.wqs, "wcs": r.wcs, "tcq": r.tcq },
        "demo": row.get::<bool, _>("demo"),
    })
}

fn envelope(demo: bool, any: bool) -> Envelope {
    let now = Utc::now();
    if !any {
        Envelope::new(DataStatus::Unavailable, now, false)
            .note("No signals have been published yet.")
    } else if demo {
        Envelope::new(DataStatus::Demo, now, true)
            .note("Seeded demo signals. Not real market events.")
    } else {
        Envelope::new(DataStatus::Live, now, false)
    }
}

async fn list(
    state: web::Data<AppState>,
    q: web::Query<ListQuery>,
) -> Result<HttpResponse, ApiError> {
    if let Some(s) = &q.state {
        if !STATES.contains(&s.as_str()) {
            return Err(ApiError::BadRequest("Unknown signal state.".into()));
        }
    }
    let limit = page_limit(q.limit);
    let after = q.cursor.as_deref().map(decode_cursor).transpose()?;
    let rows = sqlx::query(
        r#"SELECT * FROM signals
           WHERE ($1::text IS NULL OR state = $1)
             AND ($2::timestamptz IS NULL OR (published_at, id) < ($2, $3))
           ORDER BY published_at DESC, id DESC LIMIT $4"#,
    )
    .bind(&q.state)
    .bind(after.map(|a| a.0))
    .bind(after.map(|a| a.1).unwrap_or_else(Uuid::nil))
    .bind(limit + 1)
    .fetch_all(&state.pool)
    .await?;
    let more = rows.len() as i64 > limit;
    let page: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let mut items = vec![];
    for r in &page {
        // One damaged receipt must not hide the others. Its content is withheld.
        match verified_receipt(r) {
            Ok(rc) => items.push(summary(r, &rc)),
            Err(_) => items.push(json!({
                "id": r.get::<Uuid, _>("id"),
                "state": r.get::<String, _>("state"),
                "published_at": r.get::<DateTime<Utc>, _>("published_at"),
                "integrity": "failed",
                "demo": r.get::<bool, _>("demo"),
            })),
        }
    }
    let next = if more {
        page.last()
            .map(|r| encode_cursor(r.get("published_at"), r.get("id")))
    } else {
        None
    };
    let demo = page.iter().any(|r| r.get::<bool, _>("demo"));
    let mut body = serde_json::to_value(envelope(demo, !page.is_empty())).unwrap_or(Value::Null);
    body["items"] = Value::Array(items);
    body["next_cursor"] = json!(next);
    Ok(HttpResponse::Ok().json(body))
}

async fn get(state: web::Data<AppState>, id: web::Path<Uuid>) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    let row = sqlx::query("SELECT * FROM signals WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound)?;
    let receipt = verified_receipt(&row)?;
    let events =
        sqlx::query("SELECT state, at, note FROM signal_events WHERE signal_id = $1 ORDER BY seq")
            .bind(id)
            .fetch_all(&state.pool)
            .await?;
    let demo = row.get::<bool, _>("demo");
    let mut body = serde_json::to_value(envelope(demo, true)).unwrap_or(Value::Null);
    if let (Value::Object(b), Value::Object(s)) = (&mut body, summary(&row, &receipt)) {
        b.extend(s);
    }
    body["receipt"] = serde_json::to_value(&receipt).unwrap_or(Value::Null);
    body["receipt_hash"] = json!(row.get::<String, _>("receipt_hash"));
    body["events"] = Value::Array(
        events
            .iter()
            .map(|e| json!({ "state": e.get::<String, _>("state"), "at": e.get::<DateTime<Utc>, _>("at"), "note": e.get::<Option<String>, _>("note") }))
            .collect(),
    );
    Ok(HttpResponse::Ok().json(body))
}

fn horizon_label(secs: i64) -> &'static str {
    match secs {
        3_600 => "1h",
        86_400 => "24h",
        604_800 => "7d",
        2_592_000 => "30d",
        _ => "custom",
    }
}

/// All four horizons are always listed. A losing, unpriced or untradeable result
/// is shown exactly like a gain; a horizon not yet due says so.
async fn outcomes(
    state: web::Data<AppState>,
    id: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    let row = sqlx::query("SELECT * FROM signals WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound)?;
    let receipt = verified_receipt(&row)?;
    let stored = sqlx::query("SELECT horizon_secs, kind, price_usd, return_pct, evaluated_at FROM signal_outcomes WHERE signal_id = $1")
        .bind(id)
        .fetch_all(&state.pool)
        .await?;
    let now = Utc::now();
    let mut items = vec![];
    let mut recorded: Vec<Outcome> = vec![];
    for h in HORIZONS_SECS {
        let due = receipt.published_at + chrono::Duration::seconds(h);
        match stored.iter().find(|r| r.get::<i64, _>("horizon_secs") == h) {
            Some(r) => {
                let kind: String = r.get("kind");
                let ret: Option<rust_decimal::Decimal> = r.get("return_pct");
                let px: Option<rust_decimal::Decimal> = r.get("price_usd");
                // Rebuild for the summary counts only.
                let k = match (kind.as_str(), ret, px) {
                    ("observed", Some(p), Some(x)) => OutcomeKind::Observed {
                        return_bps: (p * rust_decimal::Decimal::from(100)).round().to_string().parse().unwrap_or(0),
                        exit_price_usd: x,
                    },
                    ("untradeable", _, _) => OutcomeKind::Untradeable,
                    ("data_unavailable", _, _) => OutcomeKind::DataUnavailable,
                    ("expired", _, _) => OutcomeKind::Expired,
                    ("error", _, _) => OutcomeKind::Error { message: "evaluation error".into() },
                    _ => OutcomeKind::Unpriced,
                };
                recorded.push(Outcome { horizon_secs: h, recorded_at: r.get("evaluated_at"), kind: k });
                items.push(json!({
                    "horizon": horizon_label(h), "horizon_secs": h, "status": "recorded", "kind": kind,
                    "return_pct": ret.map(dec_str), "exit_price_usd": px.map(dec_str),
                    "evaluated_at": r.get::<DateTime<Utc>, _>("evaluated_at"), "due_at": due,
                }));
            }
            None => items.push(json!({
                "horizon": horizon_label(h), "horizon_secs": h,
                "status": if due > now { "pending" } else { "awaiting_evaluation" },
                "kind": null, "return_pct": null, "exit_price_usd": null, "evaluated_at": null, "due_at": due,
            })),
        }
    }
    let demo = row.get::<bool, _>("demo");
    let mut body = serde_json::to_value(envelope(demo, true)).unwrap_or(Value::Null);
    body["signal_id"] = json!(id);
    body["exit_policy"] = json!(receipt.exit_policy);
    body["items"] = Value::Array(items);
    body["summary"] =
        serde_json::to_value(trace_signals::summarize(&recorded)).unwrap_or(Value::Null);
    Ok(HttpResponse::Ok().json(body))
}
