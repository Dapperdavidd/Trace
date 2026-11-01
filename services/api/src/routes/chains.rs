use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde::Serialize;
use trace_chain_adapters::ChainCapability;
use trace_database::coverage::{list_protocol_coverage, ProtocolCoverageRow};
use trace_domain::DataStatus;

use crate::{error::ApiError, state::AppState};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/chains", web::get().to(list_chains));
}

#[derive(Serialize)]
struct ChainsResponse {
    chains: Vec<ChainCapability>,
    /// Persisted ingestion state per protocol (last successful scan, lookback).
    ingestion: Vec<ProtocolCoverageRow>,
    /// `partial` until at least one protocol has been enabled with fixture evidence.
    data_status: DataStatus,
    as_of: DateTime<Utc>,
    demo: bool,
    coverage_note: &'static str,
    note: &'static str,
}

/// Chain, protocol and freshness truth. Never claims coverage a parser has
/// not earned; watch-only support is separate from parser support.
async fn list_chains(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let chains = trace_chain_adapters::chains();
    let ingestion = list_protocol_coverage(&state.pool)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?;
    let any_enabled = ingestion.iter().any(|r| r.support_status == "enabled");
    Ok(HttpResponse::Ok().json(ChainsResponse {
        chains,
        ingestion,
        data_status: if any_enabled { DataStatus::Partial } else { DataStatus::Unavailable },
        as_of: Utc::now(),
        demo: false,
        coverage_note: "Ethereum Uniswap V2 is enabled. Other listed protocols remain disabled or unsupported.",
        note: "Any valid public address can be watched on a listed chain. Parser coverage is reported per protocol and is never assumed.",
    }))
}
