use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use trace_domain::ChainId;
use trace_jobs::Failure;
use trace_worker::metadata_job::{fetch_missing, MetadataSource, TokenMetadata};

const TOKEN: &str = "0x00000000000000000000000000000000000000f1";

fn now() -> DateTime<Utc> {
    Utc.timestamp_opt(1_793_000_000, 0).unwrap()
}

struct Stub {
    result: Option<TokenMetadata>,
    calls: Arc<Mutex<Vec<(ChainId, String)>>>,
}

#[async_trait]
impl MetadataSource for Stub {
    async fn fetch(&self, chain: ChainId, token: &str) -> Result<Option<TokenMetadata>, Failure> {
        self.calls.lock().unwrap().push((chain, token.into()));
        Ok(self.result.clone())
    }
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn fills_missing_metadata_without_overwriting_reviewed_labels(pool: PgPool) {
    sqlx::query(
        "INSERT INTO tokens (chain,contract_or_mint,symbol)
         VALUES ('ethereum',$1,'REVIEWED')",
    )
    .bind(TOKEN)
    .execute(&pool)
    .await
    .unwrap();
    let calls = Arc::new(Mutex::new(vec![]));
    let source = Stub {
        result: Some(TokenMetadata {
            decimals: 18,
            symbol: Some("RPC".into()),
            name: Some("RPC Token".into()),
        }),
        calls: calls.clone(),
    };
    let report = fetch_missing(&pool, &source, now()).await.unwrap();
    assert_eq!((report.requested, report.completed), (1, 1));
    assert_eq!(
        calls.lock().unwrap().as_slice(),
        &[(ChainId::Ethereum, TOKEN.to_string())]
    );
    let row: (Option<i16>, Option<String>, Option<String>, String) = sqlx::query_as(
        "SELECT decimals,symbol,name,metadata_status FROM tokens
         WHERE chain='ethereum' AND contract_or_mint=$1",
    )
    .bind(TOKEN)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.0, Some(18));
    assert_eq!(row.1.as_deref(), Some("REVIEWED"));
    assert_eq!(row.2.as_deref(), Some("RPC Token"));
    assert_eq!(row.3, "complete");

    let second = fetch_missing(&pool, &source, now()).await.unwrap();
    assert_eq!(second.requested, 0);
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn non_token_contract_is_marked_unavailable(pool: PgPool) {
    sqlx::query("INSERT INTO tokens (chain,contract_or_mint) VALUES ('ethereum',$1)")
        .bind(TOKEN)
        .execute(&pool)
        .await
        .unwrap();
    let source = Stub {
        result: None,
        calls: Arc::new(Mutex::new(vec![])),
    };
    let report = fetch_missing(&pool, &source, now()).await.unwrap();
    assert_eq!((report.requested, report.unavailable), (1, 1));
    let state: String = sqlx::query_scalar(
        "SELECT metadata_status FROM tokens WHERE chain='ethereum' AND contract_or_mint=$1",
    )
    .bind(TOKEN)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(state, "unavailable");
}
