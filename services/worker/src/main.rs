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
                match trace_worker::drain(&pool, now).await {
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
