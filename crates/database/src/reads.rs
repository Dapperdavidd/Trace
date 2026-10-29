//! Read-side queries that rebuild domain types from stored rows.

use chrono::{DateTime, Utc};
use sqlx::{postgres::PgRow, PgPool, Row};
use std::str::FromStr;
use trace_domain::{
    Address, AssetId, Attribution, BaseUnits, ChainId, Finality, NormalizedSwap, WalletId,
};

use crate::DbError;

const SWAP_COLUMNS: &str = r#"chain, tx_id, event_index, finality_revision, wallet, protocol, pool,
    token_in, amount_in::text AS amount_in, token_out, amount_out::text AS amount_out,
    block_height, block_hash, observed_at, finality, attribution, parser_version, raw_ref, demo"#;

fn corrupt(what: &str, detail: impl std::fmt::Display) -> DbError {
    DbError::Corrupt(format!("{what}: {detail}"))
}

fn enum_from_str<T: serde::de::DeserializeOwned>(what: &str, s: String) -> Result<T, DbError> {
    serde_json::from_value(serde_json::Value::String(s.clone()))
        .map_err(|_| corrupt(what, format!("unexpected value '{s}'")))
}

fn row_to_swap(r: &PgRow) -> Result<NormalizedSwap, DbError> {
    let chain = ChainId::from_str(&r.get::<String, _>("chain")).map_err(|e| corrupt("chain", e))?;
    let addr = |col: &str| -> Result<Address, DbError> {
        Address::parse(chain, &r.get::<String, _>(col)).map_err(|e| corrupt(col, e))
    };
    let amount = |col: &str| -> Result<BaseUnits, DbError> {
        BaseUnits::from_str(&r.get::<String, _>(col)).map_err(|e| corrupt(col, e))
    };
    Ok(NormalizedSwap {
        chain,
        tx_id: r.get("tx_id"),
        event_index: r.get::<i32, _>("event_index") as u32,
        finality_revision: r.get::<i32, _>("finality_revision") as u32,
        wallet: WalletId {
            chain,
            address: addr("wallet")?,
        },
        protocol: r.get("protocol"),
        pool: r.get("pool"),
        token_in: AssetId {
            chain,
            contract_or_mint: addr("token_in")?,
        },
        amount_in: amount("amount_in")?,
        token_out: AssetId {
            chain,
            contract_or_mint: addr("token_out")?,
        },
        amount_out: amount("amount_out")?,
        block_height: r.get::<Option<i64>, _>("block_height").map(|h| h as u64),
        block_hash: r.get("block_hash"),
        observed_at: r.get("observed_at"),
        finality: enum_from_str::<Finality>("finality", r.get("finality"))?,
        attribution: enum_from_str::<Attribution>("attribution", r.get("attribution"))?,
        parser_version: r.get("parser_version"),
        raw_ref: r.get("raw_ref"),
        demo: r.get("demo"),
    })
}

/// All non-reorged swaps for one wallet, oldest first, capped at `limit`
/// (the most recent `limit` when more exist).
pub async fn wallet_swaps(
    pool: &PgPool,
    chain: &str,
    address: &str,
    limit: i64,
) -> Result<Vec<NormalizedSwap>, DbError> {
    let sql = format!(
        "SELECT * FROM (SELECT {SWAP_COLUMNS} FROM normalized_swaps
           WHERE chain = $1 AND wallet = $2 AND finality <> 'reorged'
           ORDER BY observed_at DESC, tx_id DESC, event_index DESC LIMIT $3) t
         ORDER BY observed_at, tx_id, event_index"
    );
    sqlx::query(&sql)
        .bind(chain)
        .bind(address)
        .bind(limit)
        .fetch_all(pool)
        .await?
        .iter()
        .map(row_to_swap)
        .collect()
}

/// Direct buys of a token (token_out) since `since`, newest first.
pub async fn token_buys(
    pool: &PgPool,
    chain: &str,
    token: &str,
    since: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<NormalizedSwap>, DbError> {
    let sql = format!(
        "SELECT {SWAP_COLUMNS} FROM normalized_swaps
         WHERE chain = $1 AND token_out = $2 AND observed_at >= $3
           AND finality <> 'reorged' AND attribution = 'direct'
         ORDER BY observed_at DESC, tx_id DESC LIMIT $4"
    );
    sqlx::query(&sql)
        .bind(chain)
        .bind(token)
        .bind(since)
        .bind(limit)
        .fetch_all(pool)
        .await?
        .iter()
        .map(row_to_swap)
        .collect()
}

/// Distinct wallets that have at least one non-reorged swap, most active first.
pub async fn active_wallets(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<(String, String, i64)>, DbError> {
    Ok(sqlx::query(
        r#"SELECT chain, wallet, count(*) AS n FROM normalized_swaps
           WHERE finality <> 'reorged' AND attribution = 'direct'
           GROUP BY chain, wallet ORDER BY n DESC, chain, wallet LIMIT $1"#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| (r.get("chain"), r.get("wallet"), r.get("n")))
    .collect())
}
