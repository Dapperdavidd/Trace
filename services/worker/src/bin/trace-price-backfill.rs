use chrono::{DateTime, Utc};
use trace_worker::price_backfill_job;
use uuid::Uuid;

fn usage() -> ! {
    eprintln!(
        "usage:\n  trace-price-backfill request <chain> <asset> <from-rfc3339> <to-rfc3339> <interval-seconds>\n  trace-price-backfill status [run-id]\n  trace-price-backfill cancel <run-id>"
    );
    std::process::exit(2);
}

fn time(value: &str) -> anyhow::Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = trace_database::connect(&database_url, 2).await?;
    trace_database::migrate(&pool).await?;

    match args.as_slice() {
        [command, chain, asset, from, to, interval] if command == "request" => {
            let id = price_backfill_job::request(
                &pool,
                chain,
                asset,
                time(from)?,
                time(to)?,
                interval.parse()?,
            )
            .await?;
            println!("queued price backfill {id}");
        }
        [command] if command == "status" => print_runs(&pool, None).await?,
        [command, id] if command == "status" => {
            print_runs(&pool, Some(Uuid::parse_str(id)?)).await?
        }
        [command, id] if command == "cancel" => {
            let id = Uuid::parse_str(id)?;
            if price_backfill_job::cancel(&pool, id).await? {
                println!("cancelled price backfill {id}");
            } else {
                println!("price backfill {id} was not pending");
            }
        }
        _ => usage(),
    }
    Ok(())
}

async fn print_runs(pool: &sqlx::PgPool, id: Option<Uuid>) -> anyhow::Result<()> {
    for run in price_backfill_job::list(pool, id).await? {
        println!(
            "{} {}:{} {} next={} stored={} rejected={}{}",
            run.id,
            run.chain,
            run.asset,
            run.status,
            run.next_time.to_rfc3339(),
            run.stored_points,
            run.rejected_points,
            run.last_error
                .as_deref()
                .map(|error| format!(" error={error}"))
                .unwrap_or_default()
        );
    }
    Ok(())
}
