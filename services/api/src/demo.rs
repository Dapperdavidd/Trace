//! Demo data. Everything written here is flagged `demo = true`, uses obviously
//! synthetic token addresses and a non-existent protocol name, and is replaced
//! by nothing real. API responses built from it carry `data_status: demo`.
//!
//! Prices follow a seeded random walk so a fresh database always produces the
//! same story. This is not market data and proves nothing about performance.

use chrono::{DateTime, Duration, Utc};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde_json::json;
use sqlx::PgPool;
use trace_domain::{AssetId, Attribution, BaseUnits, ChainId, Finality, NormalizedSwap, WalletId};
use trace_signals::{Receipt, RULES_VERSION};
use uuid::Uuid;

const USDC: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const POOL: &str = "0x00000000000000000000000000000000000d3f00";
const MARKER: &str = "demo_seeded";

struct Tok {
    chain: ChainId,
    addr: &'static str,
    symbol: &'static str,
    name: &'static str,
    base_price_micro: i64,
    drift_bps: i64,
}

const TOKENS: [Tok; 5] = [
    Tok {
        chain: ChainId::Ethereum,
        addr: "0x000000000000000000000000000000000000d001",
        symbol: "NOVA",
        name: "Demo Nova",
        base_price_micro: 1_200_000,
        drift_bps: 12,
    },
    Tok {
        chain: ChainId::Ethereum,
        addr: "0x000000000000000000000000000000000000d002",
        symbol: "PRSM",
        name: "Demo Prism",
        base_price_micro: 340_000,
        drift_bps: 4,
    },
    Tok {
        chain: ChainId::Base,
        addr: "0x000000000000000000000000000000000000d003",
        symbol: "KELP",
        name: "Demo Kelp",
        base_price_micro: 52_000,
        drift_bps: 20,
    },
    Tok {
        chain: ChainId::Base,
        addr: "0x000000000000000000000000000000000000d004",
        symbol: "ORBT",
        name: "Demo Orbit",
        base_price_micro: 8_000,
        drift_bps: -3,
    },
    Tok {
        chain: ChainId::Ethereum,
        addr: "0x000000000000000000000000000000000000d005",
        symbol: "HNYP",
        name: "Demo Honeypot",
        base_price_micro: 150_000,
        drift_bps: 0,
    },
];

const STEP_MIN: i64 = 30;
const POINTS: i64 = 480; // 10 days of half-hour points

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state >> 33
}

/// Deterministic price path in micro-USD per whole token.
fn path(t: &Tok, salt: u64) -> Vec<i64> {
    let mut s = salt.wrapping_add(t.addr.len() as u64 * 7919) ^ (t.base_price_micro as u64);
    let mut p = t.base_price_micro;
    (0..POINTS)
        .map(|_| {
            let step = (lcg(&mut s) % 601) as i64 - 300 + t.drift_bps; // +/- 3% per half hour
            p = (p + p * step / 10_000).max(1_000);
            p
        })
        .collect()
}

fn micro(p: i64) -> Decimal {
    Decimal::new(p, 6)
}

fn tx(n: u64) -> String {
    format!("0x{:064x}", 0xde30_0000_0000_0000u64 + n)
}

fn asset(chain: ChainId, a: &str) -> AssetId {
    AssetId::parse(chain, a).expect("static demo address")
}

fn base_units(usd: Decimal, price: Decimal, decimals: u32) -> BaseUnits {
    let qty = usd / price * Decimal::from(10u64.pow(decimals.min(18)));
    BaseUnits(qty.trunc().to_u128().unwrap_or(0).max(1))
}

pub async fn is_seeded(pool: &PgPool) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM audit_events WHERE action = $1)")
        .bind(MARKER)
        .fetch_one(pool)
        .await
}

/// Seed once. A second call only refreshes timestamps.
pub async fn seed(pool: &PgPool, now: DateTime<Utc>) -> anyhow::Result<bool> {
    if is_seeded(pool).await? {
        refresh(pool, now).await?;
        return Ok(false);
    }
    let start = now - Duration::minutes(STEP_MIN * POINTS);

    // Tokens and price points.
    sqlx::query("INSERT INTO tokens (chain, contract_or_mint, symbol, name, decimals, is_stable) VALUES ('ethereum',$1,'USDC','USD Coin',6,TRUE) ON CONFLICT (chain, contract_or_mint) DO UPDATE SET symbol='USDC', name='USD Coin', decimals=6, is_stable=TRUE")
        .bind(USDC)
        .execute(pool)
        .await?;
    let mut paths = vec![];
    for (i, t) in TOKENS.iter().enumerate() {
        sqlx::query("INSERT INTO tokens (chain, contract_or_mint, symbol, name, decimals) VALUES ($1,$2,$3,$4,18) ON CONFLICT (chain, contract_or_mint) DO UPDATE SET symbol=$3, name=$4, decimals=18")
            .bind(t.chain.as_str())
            .bind(t.addr)
            .bind(t.symbol)
            .bind(t.name)
            .execute(pool)
            .await?;
        let pth = path(t, 1000 + i as u64);
        for (k, p) in pth.iter().enumerate() {
            let at = start + Duration::minutes(STEP_MIN * k as i64);
            sqlx::query("INSERT INTO price_points (chain, asset, observed_at, usd_per_base_unit, source, demo) VALUES ($1,$2,$3,$4,'demo_seed',TRUE) ON CONFLICT DO NOTHING")
                .bind(t.chain.as_str())
                .bind(t.addr)
                .bind(at)
                .bind(micro(*p) / Decimal::from(1_000_000_000_000_000_000u64))
                .execute(pool)
                .await?;
        }
        paths.push(pth);
    }

    // Three demo wallets on Ethereum trading the two Ethereum tokens NOVA and PRSM.
    let wallets = [
        (
            "0x00000000000000000000000000000000000a0001",
            14usize,
            85u64,
            11u64,
        ),
        ("0x00000000000000000000000000000000000a0002", 12, 40, 23),
        ("0x00000000000000000000000000000000000a0003", 3, 50, 37),
    ];
    let mut n: u64 = 0;
    for (waddr, trips, win_pct, salt) in wallets {
        let w = WalletId::parse(ChainId::Ethereum, waddr)?;
        let mut rng = salt * 977;
        for k in 0..trips {
            let ti = k % 2; // NOVA or PRSM
            let t = &TOKENS[ti];
            let pth = &paths[ti];
            let buy_idx = 10 + (k * 29 + salt as usize) % 380;
            let wants_win = lcg(&mut rng) % 100 < win_pct;
            let window = &pth[buy_idx + 2..(buy_idx + 50).min(pth.len())];
            let sell_off = if wants_win {
                window
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, p)| **p)
                    .map(|(i, _)| i)
            } else {
                window
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, p)| **p)
                    .map(|(i, _)| i)
            }
            .unwrap_or(0);
            let sell_idx = buy_idx + 2 + sell_off;
            let usd = Decimal::from(250 + (lcg(&mut rng) % 8) * 125);
            let (bp, sp) = (micro(pth[buy_idx]), micro(pth[sell_idx]));
            let bu = base_units(usd, bp, 18);
            let buy_at = start + Duration::minutes(STEP_MIN * buy_idx as i64);
            let sell_at = start + Duration::minutes(STEP_MIN * sell_idx as i64);
            let mk = |id: u64,
                      ti_in: &str,
                      a_in: BaseUnits,
                      ti_out: &str,
                      a_out: BaseUnits,
                      at: DateTime<Utc>| NormalizedSwap {
                chain: ChainId::Ethereum,
                tx_id: tx(id),
                event_index: 0,
                finality_revision: 0,
                wallet: w.clone(),
                protocol: "demo_amm".into(),
                pool: POOL.into(),
                token_in: asset(ChainId::Ethereum, ti_in),
                amount_in: a_in,
                token_out: asset(ChainId::Ethereum, ti_out),
                amount_out: a_out,
                block_height: None,
                block_hash: None,
                observed_at: at,
                finality: Finality::Final,
                attribution: Attribution::Direct,
                parser_version: "demo-seed/1".into(),
                raw_ref: "demo:seed".into(),
                demo: true,
            };
            n += 1;
            let usdc_in = BaseUnits(
                (usd * Decimal::from(1_000_000))
                    .trunc()
                    .to_u128()
                    .unwrap_or(1),
            );
            trace_database::ingestion::insert_swap(pool, &mk(n, USDC, usdc_in, t.addr, bu, buy_at))
                .await?;
            n += 1;
            let proceeds = Decimal::from(bu.0) / Decimal::from(10u64.pow(18)) * sp;
            let usdc_out = BaseUnits(
                (proceeds * Decimal::from(1_000_000))
                    .trunc()
                    .to_u128()
                    .unwrap_or(1)
                    .max(1),
            );
            trace_database::ingestion::insert_swap(
                pool,
                &mk(n, t.addr, bu, USDC, usdc_out, sell_at),
            )
            .await?;
        }
    }

    // Market snapshots: (token index, liquidity, vol window, vol baseline, smart buyers, large flows, pool age, risk inputs)
    #[allow(clippy::type_complexity)]
    let specs: [(
        usize,
        i64,
        i64,
        Option<i64>,
        i32,
        i32,
        Option<i32>,
        serde_json::Value,
    ); 5] = [
        (
            0,
            850_000,
            2_400_000,
            Some(400_000),
            3,
            0,
            None,
            json!({"honeypot": false, "sell_tax_bps": 0, "mint_authority_active": false, "blacklist_function": false, "price_impact_bps": 80, "reference_size_usd": "1000", "top10_holder_pct": "38", "abnormal_activity": false}),
        ),
        (
            1,
            320_000,
            1_050_000,
            Some(300_000),
            0,
            0,
            None,
            json!({"honeypot": false, "sell_tax_bps": 100, "mint_authority_active": false, "blacklist_function": false, "price_impact_bps": 150, "reference_size_usd": "1000", "top10_holder_pct": "72", "abnormal_activity": false}),
        ),
        (
            2,
            95_000,
            610_000,
            Some(150_000),
            1,
            3,
            Some(6),
            json!({"honeypot": false, "sell_tax_bps": 0, "mint_authority_active": false, "blacklist_function": false, "price_impact_bps": 260, "reference_size_usd": "1000", "top10_holder_pct": "55", "abnormal_activity": false}),
        ),
        (
            3,
            4_000,
            900_000,
            Some(1_000),
            3,
            0,
            None,
            json!({"honeypot": false}),
        ),
        (
            4,
            600_000,
            1_800_000,
            Some(200_000),
            4,
            0,
            None,
            json!({"honeypot": true}),
        ),
    ];
    for (ti, liq, vol, base, smart, large, age, risk) in specs {
        let t = &TOKENS[ti];
        let pth = &paths[ti];
        for (win, back) in [("1h", 2usize), ("24h", 48), ("7d", 336)] {
            let now_p = micro(*pth.last().unwrap());
            let start_p = micro(pth[pth.len() - 1 - back]);
            let spark: Vec<f64> = pth[pth.len() - 24..]
                .iter()
                .map(|p| *p as f64 / 1e6)
                .collect();
            sqlx::query(
                r#"INSERT INTO market_snapshots (chain, token, pool, win, price_now_usd, price_window_start_usd, volume_window_usd,
                   volume_baseline_usd, liquidity_usd, market_cap_usd, holders, pool_age_hours, independent_smart_buyers,
                   large_buyer_flows, sparkline, risk_inputs, last_updated, demo)
                   VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,TRUE)
                   ON CONFLICT (chain, token, pool, win) DO NOTHING"#,
            )
            .bind(t.chain.as_str())
            .bind(t.addr)
            .bind(POOL)
            .bind(win)
            .bind(now_p)
            .bind(start_p)
            .bind(Decimal::from(vol))
            .bind(base.map(Decimal::from))
            .bind(Decimal::from(liq))
            .bind(Decimal::from(liq * 12))
            .bind(1_000 + 400 * ti as i32)
            .bind(age)
            .bind(smart)
            .bind(large)
            .bind(json!(spark))
            .bind(&risk)
            .bind(now)
            .execute(pool)
            .await?;
        }
    }

    seed_signals(pool, now).await?;
    sqlx::query(
        "INSERT INTO audit_events (id, actor, action, subject) VALUES ($1,'system',$2,'demo')",
    )
    .bind(Uuid::new_v4())
    .bind(MARKER)
    .execute(pool)
    .await?;
    Ok(true)
}

async fn seed_signals(pool: &PgPool, now: DateTime<Utc>) -> anyhow::Result<()> {
    // (id, age, token index, state, outcomes: (horizon, kind, price, return_pct))
    type Out = (
        i64,
        &'static str,
        Option<&'static str>,
        Option<&'static str>,
    );
    let specs: [(u128, Duration, usize, &str, Vec<Out>); 4] = [
        (
            1,
            Duration::days(3),
            0,
            "observation_due",
            vec![
                (3_600, "observed", Some("1.31"), Some("2.40")),
                (86_400, "observed", Some("1.18"), Some("-6.10")),
            ],
        ),
        (4, Duration::hours(2), 2, "published_as_research", vec![]),
        (
            2,
            Duration::days(10),
            1,
            "observation_due",
            vec![
                (3_600, "unpriced", None, None),
                (86_400, "observed", Some("0.36"), Some("5.20")),
                (604_800, "observed", Some("0.31"), Some("-8.30")),
            ],
        ),
        (
            3,
            Duration::days(40),
            0,
            "observed",
            vec![
                (3_600, "observed", Some("1.10"), Some("0.90")),
                (86_400, "untradeable", None, None),
                (604_800, "observed", Some("1.50"), Some("31.00")),
                (2_592_000, "observed", Some("0.98"), Some("-14.00")),
            ],
        ),
    ];
    for (n, age, ti, state, outs) in specs {
        let t = &TOKENS[ti];
        let published = now - age;
        let receipt = Receipt {
            chain: t.chain.as_str().into(),
            token: t.addr.into(),
            source_tx_ids: vec![tx(900 + n as u64)],
            wallet_evidence_ids: vec!["demo-wallet-a0001".into()],
            block_time: published - Duration::minutes(2),
            detected_at: published - Duration::minutes(1),
            published_at: published,
            reference_price_usd: Decimal::new(128, 2),
            reference_amount_usd: Decimal::from(1_000),
            fee_bps: 30,
            slippage_bps: 50,
            risk_verdict: "pass".into(),
            security_findings: vec![],
            wqs: Some(71),
            wcs: None,
            tcq: Some(64),
            score_versions: vec!["wqs/1".into(), "tcq/1".into()],
            rules_version: RULES_VERSION.into(),
            exit_policy: "Exit at the first available price at or after each horizon.".into(),
            horizons_secs: trace_signals::HORIZONS_SECS.to_vec(),
        };
        let id = Uuid::from_u128(0xde30_0000_0000_0000_0000_0000_0000_0000 + n);
        sqlx::query("INSERT INTO signals (id, wallet_chain, wallet_address, token_chain, token, receipt, receipt_hash, state, published_at, demo) VALUES ($1,'ethereum','0x00000000000000000000000000000000000a0001',$2,$3,$4,$5,$6,$7,TRUE) ON CONFLICT DO NOTHING")
            .bind(id)
            .bind(t.chain.as_str())
            .bind(t.addr)
            .bind(serde_json::to_value(&receipt)?)
            .bind(receipt.seal())
            .bind(state)
            .bind(published)
            .execute(pool)
            .await?;
        for (seq, (st, note)) in [
            ("detected", "Swap detected"),
            ("evaluated", "Checks run"),
            ("published_as_research", "Published as research"),
            ("observation_due", "Awaiting horizons"),
        ]
        .iter()
        .take(if state == "published_as_research" {
            3
        } else {
            4
        })
        .enumerate()
        {
            sqlx::query("INSERT INTO signal_events (signal_id, seq, state, at, note) VALUES ($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING")
                .bind(id).bind(seq as i32).bind(st).bind(published + Duration::seconds(seq as i64 * 20)).bind(note)
                .execute(pool).await?;
        }
        for (h, kind, px, ret) in outs {
            sqlx::query("INSERT INTO signal_outcomes (signal_id, horizon_secs, kind, price_usd, return_pct, evaluated_at) VALUES ($1,$2,$3,$4::numeric,$5::numeric,$6) ON CONFLICT DO NOTHING")
                .bind(id).bind(h).bind(kind).bind(px).bind(ret).bind(published + Duration::seconds(h))
                .execute(pool).await?;
        }
    }
    Ok(())
}

/// Keep demo snapshots looking current so movers are not excluded as stale.
/// Only demo rows are touched.
pub async fn refresh(pool: &PgPool, now: DateTime<Utc>) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE market_snapshots SET last_updated = $1 WHERE demo")
        .bind(now)
        .execute(pool)
        .await?;
    Ok(())
}

/// Starter alerts for a new account in demo mode. Skipped if the user has any.
pub async fn seed_alerts_for(
    pool: &PgPool,
    user_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let has: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM alerts WHERE user_id = $1)")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    if has {
        return Ok(());
    }
    let items = [
        (
            "wallet_buy",
            "Demo wallet bought Demo Nova",
            "A demo wallet bought about $750 of Demo Nova. This is seeded demo data.",
            25,
        ),
        (
            "risk_event",
            "Demo Prism concentration warning",
            "Top holders control a large share of Demo Prism. Seeded demo data.",
            70,
        ),
        (
            "signal",
            "New research signal (demo)",
            "A demo signal was published with its receipt. Seeded demo data.",
            130,
        ),
        (
            "wallet_sell",
            "Demo wallet sold Demo Nova",
            "A demo wallet exited a Demo Nova position. Seeded demo data.",
            400,
        ),
    ];
    for (ty, title, body, mins) in items {
        sqlx::query("INSERT INTO alerts (id, user_id, type, title, body, evidence_tx, event_time, published_at, demo) VALUES ($1,$2,$3,$4,$5,ARRAY[$6],$7,$7,TRUE)")
            .bind(Uuid::new_v4())
            .bind(user_id)
            .bind(ty)
            .bind(title)
            .bind(body)
            .bind(tx(900))
            .bind(now - Duration::minutes(mins))
            .execute(pool)
            .await?;
    }
    Ok(())
}
