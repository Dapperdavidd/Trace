//! PostgreSQL access. Schema is owned by forward-only SQLx migrations in
//! `/migrations`; nothing here creates tables at runtime.

use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

pub mod coverage;
pub mod ingestion;
pub mod reads;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("stored data could not be read: {0}")]
    Corrupt(String),
}

pub async fn connect(database_url: &str, max_connections: u32) -> Result<PgPool, DbError> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await?;
    Ok(pool)
}

pub async fn migrate(pool: &PgPool) -> Result<(), DbError> {
    MIGRATOR.run(pool).await?;
    Ok(())
}

/// Cheap liveness probe used by /v1/health/ready.
pub async fn ping(pool: &PgPool) -> Result<(), DbError> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}
