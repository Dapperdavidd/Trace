use std::net::TcpListener;
use trace_api::{config::Config, run, AppState};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenv_lite();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .json()
        .init();

    let cfg = Config::from_env()?;
    let problems = cfg.production_problems();
    if !problems.is_empty() {
        for p in &problems {
            tracing::error!("unsafe production configuration: {p}");
        }
        anyhow::bail!(
            "refusing to start: {} unsafe production setting(s)",
            problems.len()
        );
    }
    let pool = trace_database::connect(&cfg.database_url, cfg.db_max_connections).await?;
    trace_database::migrate(&pool).await?;
    sync_registry(&pool).await?;
    if cfg.seed_demo {
        let created = trace_api::demo::seed(&pool, chrono::Utc::now()).await?;
        tracing::info!(created, "demo data ready");
    }

    let listener = TcpListener::bind(&cfg.bind_addr)?;
    tracing::info!(addr = %cfg.bind_addr, "trace-api listening");
    let verifier: std::sync::Arc<dyn trace_api::auth::IdTokenVerifier> = match (
        &cfg.google_client_id,
        cfg.dev_auth,
    ) {
        (_, true) => {
            tracing::warn!("TRACE_DEV_AUTH is on: dev tokens are accepted. Never use this outside local development.");
            std::sync::Arc::new(trace_api::auth::DevVerifier)
        }
        (Some(id), false) => std::sync::Arc::new(trace_api::auth::GoogleVerifier::new(id.clone())),
        (None, false) => {
            tracing::warn!("GOOGLE_CLIENT_ID is not set: sign-in is disabled");
            std::sync::Arc::new(trace_api::auth::Disabled)
        }
    };
    run(
        listener,
        AppState::with_verifier(pool, verifier).with_ops_token(cfg.ops_token.clone()),
        cfg.cors_origins,
    )?
    .await?;
    Ok(())
}

/// Mirror the code-defined registry into PostgreSQL so coverage state has one home.
async fn sync_registry(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    for p in trace_chain_adapters::protocols() {
        let status = serde_json::to_value(p.status)?
            .as_str()
            .unwrap_or("disabled")
            .to_string();
        let lims: Vec<String> = p.limitations.iter().map(|s| s.to_string()).collect();
        trace_database::coverage::upsert_protocol(
            pool,
            p.chain.as_str(),
            p.protocol,
            &status,
            &lims,
            p.blocker,
        )
        .await?;
    }
    Ok(())
}

/// Minimal `.env` loader for local development; real env vars always win.
fn dotenv_lite() -> std::io::Result<()> {
    let text = std::fs::read_to_string(".env")?;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if std::env::var_os(k.trim()).is_none() {
                std::env::set_var(k.trim(), v.trim().trim_matches('"'));
            }
        }
    }
    Ok(())
}
