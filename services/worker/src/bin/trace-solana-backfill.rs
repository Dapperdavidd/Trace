use trace_worker::solana_backfill_job;
use uuid::Uuid;

fn usage() -> ! {
    eprintln!(
        "usage:\n  trace-solana-backfill request <protocol> <before-signature> <until-signature> <max-signatures>\n  trace-solana-backfill status [run-id]\n  trace-solana-backfill cancel <run-id>"
    );
    std::process::exit(2);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = trace_database::connect(&database_url, 2).await?;
    trace_database::migrate(&pool).await?;
    match args.as_slice() {
        [command, protocol, before, until, max] if command == "request" => {
            let id =
                solana_backfill_job::request(&pool, protocol, before, until, max.parse()?).await?;
            println!("queued Solana backfill {id}");
        }
        [command] if command == "status" => print_runs(&pool, None).await?,
        [command, id] if command == "status" => {
            print_runs(&pool, Some(Uuid::parse_str(id)?)).await?
        }
        [command, id] if command == "cancel" => {
            let id = Uuid::parse_str(id)?;
            if solana_backfill_job::cancel(&pool, id).await? {
                println!("cancelled Solana backfill {id}");
            } else {
                println!("Solana backfill {id} was not pending");
            }
        }
        _ => usage(),
    }
    Ok(())
}

async fn print_runs(pool: &sqlx::PgPool, id: Option<Uuid>) -> anyhow::Result<()> {
    for run in solana_backfill_job::list(pool, id).await? {
        println!(
            "{} {} {} processed={}/{} next_before={}{}",
            run.id,
            run.protocol,
            run.status,
            run.processed,
            run.max_signatures,
            run.next_before,
            run.last_error
                .as_deref()
                .map(|error| format!(" error={error}"))
                .unwrap_or_default()
        );
    }
    Ok(())
}
