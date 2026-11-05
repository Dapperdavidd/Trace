use std::time::Duration;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .json()
        .init();
    let url = std::env::var("DATABASE_URL")?;
    let pool = trace_database::connect(&url, 5).await?;
    trace_database::migrate(&pool).await?;
    let ctx = trace_worker::ingest_job::Ctx::from_env();
    let retention = trace_worker::retention_job::Policy::from_env()?;
    let targets = ctx.targets();
    if targets.is_empty() && trace_worker::solana_job::targets(&ctx).is_empty() {
        tracing::info!("no protocol is enabled with an RPC URL configured; ingestion is idle");
    }
    for d in targets {
        tracing::info!(chain = %d.chain, protocol = d.protocol, "ingestion active");
    }
    for d in trace_worker::solana_job::targets(&ctx) {
        tracing::info!(chain = "solana", protocol = d.protocol, "ingestion active");
    }
    if ctx.prices.is_none() {
        tracing::info!("TRACE_PRICE_PROVIDER is not set; price polling is off");
    }
    if retention.is_none() {
        tracing::info!("retention is off; no TRACE_RETENTION_*_DAYS setting is configured");
    }
    tracing::info!("trace-worker started");
    let mut tick = tokio::time::interval(Duration::from_secs(30));
    let mut stop = Box::pin(tokio::signal::ctrl_c());
    loop {
        tokio::select! {
            _ = tick.tick() => {
                let now = chrono::Utc::now();
                if let Err(e) = trace_worker::schedule_periodic(&pool, now).await {
                    tracing::warn!(error = %e, "could not schedule jobs");
                }
                if let Err(e) = trace_worker::ingest_job::schedule_ingest(&pool, &ctx, now).await {
                    tracing::warn!(error = %e, "could not schedule ingestion");
                }
                if let Err(e) = trace_worker::backfill_job::schedule(&pool, &ctx, now).await {
                    tracing::warn!(error = %e, "could not schedule backfill");
                }
                if let Err(e) = trace_worker::solana_job::schedule_solana(&pool, &ctx, now).await {
                    tracing::warn!(error = %e, "could not schedule solana ingestion");
                }
                if let Err(e) = trace_worker::solana_backfill_job::schedule(&pool, &ctx, now).await {
                    tracing::warn!(error = %e, "could not schedule solana backfill");
                }
                if let Err(e) = trace_worker::price_job::schedule_prices(&pool, ctx.prices.is_some(), now).await {
                    tracing::warn!(error = %e, "could not schedule price poll");
                }
                let historical_prices = ctx
                    .prices
                    .as_ref()
                    .is_some_and(|prices| prices.historical.is_some());
                if let Err(e) = trace_worker::price_backfill_job::schedule(&pool, historical_prices, now).await {
                    tracing::warn!(error = %e, "could not schedule price backfill");
                }
                if let Err(e) = trace_worker::market_job::schedule(&pool, ctx.prices.is_some(), now).await {
                    tracing::warn!(error = %e, "could not schedule market snapshots");
                }
                if let Err(e) = trace_worker::metadata_job::schedule(&pool, ctx.metadata.is_some(), now).await {
                    tracing::warn!(error = %e, "could not schedule token metadata");
                }
                if let Err(e) = trace_worker::retention_job::schedule(&pool, retention.as_ref(), now).await {
                    tracing::warn!(error = %e, "could not schedule retention");
                }
                match trace_worker::drain_with(&pool, now, &ctx).await {
                    Ok(n) if n > 0 => tracing::info!(ran = n, "jobs run"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!(error = %e, "job loop error"),
                }
            }
            _ = &mut stop => {
                tracing::info!("shutting down");
                return Ok(());
            }
        }
    }
}
