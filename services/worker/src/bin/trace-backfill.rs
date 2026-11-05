use trace_worker::backfill_job;
use uuid::Uuid;

fn usage() -> ! {
    eprintln!(
        "usage:\n  trace-backfill request <chain> <protocol> <from> <to>\n  trace-backfill status [run-id]\n  trace-backfill cancel <run-id>"
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
        [command, chain, protocol, from, to] if command == "request" => {
            let id =
                backfill_job::request(&pool, chain, protocol, from.parse()?, to.parse()?).await?;
            println!("queued backfill {id}");
        }
        [command] if command == "status" => print_runs(&pool, None).await?,
        [command, id] if command == "status" => {
            print_runs(&pool, Some(Uuid::parse_str(id)?)).await?
        }
        [command, id] if command == "cancel" => {
            let id = Uuid::parse_str(id)?;
            if backfill_job::cancel(&pool, id).await? {
                println!("cancelled backfill {id}");
            } else {
                println!("backfill {id} was not pending");
            }
        }
        _ => usage(),
    }
    Ok(())
}

async fn print_runs(pool: &sqlx::PgPool, id: Option<Uuid>) -> anyhow::Result<()> {
    for run in backfill_job::list(pool, id).await? {
        let done = (run.next_height - run.from_height).max(0);
        let total = run.to_height - run.from_height + 1;
        println!(
            "{} {}:{} {} {done}/{total} blocks{}",
            run.id,
            run.chain,
            run.protocol,
            run.status,
            run.last_error
                .as_deref()
                .map(|e| format!(" error={e}"))
                .unwrap_or_default()
        );
    }
    Ok(())
}
