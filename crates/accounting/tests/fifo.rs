use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use std::{collections::BTreeMap, str::FromStr};
use trace_accounting::*;
use trace_domain::*;

const USDC: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const TKN: &str = "0x00000000000000000000000000000000000000aa";

fn asset(a: &str) -> AssetId {
    AssetId::parse(ChainId::Ethereum, a).unwrap()
}

/// Prices: USDC is $1 per 1e6 base units; TKN priced from a per-test table keyed by time.
struct Prices {
    tkn_usd_per_unit: Vec<(i64, Decimal)>, // (unix ts, usd per base unit)
    priced: bool,
}

impl PriceSource for Prices {
    fn is_stable(&self, a: &AssetId) -> bool {
        a.contract_or_mint.as_str() == USDC
    }

    fn usd_value(
        &self,
        a: &AssetId,
        amount: BaseUnits,
        at: chrono::DateTime<Utc>,
    ) -> Option<PricedValue> {
        if !self.priced {
            return None;
        }
        if a.contract_or_mint.as_str() == USDC {
            return Some(PricedValue {
                usd: Decimal::from(amount.0 as u64) / Decimal::from(1_000_000u64),
                source: "stable".into(),
                is_quote: true,
            });
        }
        let p = self
            .tkn_usd_per_unit
            .iter()
            .rev()
            .find(|(t, _)| *t <= at.timestamp())?
            .1;
        Some(PricedValue {
            usd: p * Decimal::from(amount.0 as u64),
            source: "table".into(),
            is_quote: false,
        })
    }
}

fn swap(n: u32, secs: i64, tin: &str, ain: u128, tout: &str, aout: u128) -> NormalizedSwap {
    NormalizedSwap {
        chain: ChainId::Ethereum,
        tx_id: format!("0x{n:064x}"),
        event_index: 0,
        finality_revision: 0,
        wallet: WalletId::parse(
            ChainId::Ethereum,
            "0x1111111111111111111111111111111111111111",
        )
        .unwrap(),
        protocol: "uniswap_v2".into(),
        pool: "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc".into(),
        token_in: asset(tin),
        amount_in: BaseUnits(ain),
        token_out: asset(tout),
        amount_out: BaseUnits(aout),
        block_height: Some(n as u64),
        block_hash: None,
        observed_at: Utc.timestamp_opt(1_793_000_000 + secs, 0).unwrap(),
        finality: Finality::Final,
        attribution: Attribution::Direct,
        parser_version: "t".into(),
        raw_ref: "t".into(),
        demo: true,
    }
}

fn prices() -> Prices {
    // 1 base unit of TKN = $0.0001 from t=0, $0.0003 from t=100
    Prices {
        tkn_usd_per_unit: vec![
            (1_793_000_000, Decimal::from_str("0.0001").unwrap()),
            (1_793_000_100, Decimal::from_str("0.0003").unwrap()),
        ],
        priced: true,
    }
}

#[test]
fn simple_round_trip_has_exact_realized_profit() {
    // Buy 1,000,000 TKN units for 100 USDC ($100), sell all at $0.0003/unit = $300.
    let swaps = vec![
        swap(1, 0, USDC, 100_000_000, TKN, 1_000_000),
        swap(2, 200, TKN, 1_000_000, USDC, 300_000_000),
    ];
    let a = compute(&swaps, &prices(), &BTreeMap::new());
    assert_eq!(a.realized_pnl_usd, Some(Decimal::from(200)));
    assert_eq!(a.winning_closes, 1);
    assert!(a
        .open_positions
        .iter()
        .all(|p| p.base_units != "1000000" || p.asset != TKN));
}

#[test]
fn fifo_consumes_oldest_lot_first_and_splits_partially() {
    // Lot A: 1,000 units for $10 (t0). Lot B: 1,000 units for $30 (t100 price).
    // Sell 1,500 units at $50 total: consumes all of A and half of B.
    let swaps = vec![
        swap(1, 0, USDC, 10_000_000, TKN, 100_000),
        swap(2, 100, USDC, 30_000_000, TKN, 100_000),
        swap(3, 200, TKN, 150_000, USDC, 50_000_000),
    ];
    let a = compute(&swaps, &prices(), &BTreeMap::new());
    // proceeds 50 ; cost = 10 + 15 = 25 ; pnl 25
    assert_eq!(a.realized_pnl_usd, Some(Decimal::from(25)));
    assert_eq!(a.closed_lots.len(), 2);
    assert_eq!(a.closed_lots[0].base_units, "100000");
    assert_eq!(a.closed_lots[1].base_units, "50000");
    let open = a.open_positions.iter().find(|p| p.asset == TKN).unwrap();
    assert_eq!(open.base_units, "50000");
    assert_eq!(open.cost_usd, Some(Decimal::from(15)));
}

#[test]
fn selling_inventory_we_never_saw_bought_has_unresolved_basis_not_profit() {
    // Wallet only appears selling TKN it received by transfer.
    let swaps = vec![swap(1, 0, TKN, 1_000_000, USDC, 500_000_000)];
    let a = compute(&swaps, &prices(), &BTreeMap::new());
    assert_eq!(
        a.realized_pnl_usd, None,
        "must not fabricate a profit of $500"
    );
    assert_eq!(a.unresolved_disposals, 1);
    assert!(a
        .anomalies
        .iter()
        .any(|x| matches!(x, Anomaly::UnresolvedBasis { .. })));
    assert_eq!(a.pnl_confidence, PnlConfidence::Insufficient);
}

#[test]
fn partially_unknown_inventory_only_resolves_the_known_part() {
    // Buy 100 units (known), then sell 300 units for $90: only 100/300 is resolvable.
    let swaps = vec![
        swap(1, 0, USDC, 10_000_000, TKN, 100),
        swap(2, 50, TKN, 300, USDC, 90_000_000),
    ];
    let a = compute(&swaps, &prices(), &BTreeMap::new());
    // known part: proceeds 90 * 100/300 = 30, cost 10, pnl 20
    assert_eq!(a.realized_pnl_usd, Some(Decimal::from(20)));
    assert_eq!(a.unresolved_disposals, 1);
}

#[test]
fn unpriced_swaps_leave_basis_unknown_and_are_reported() {
    let unpriced = Prices {
        tkn_usd_per_unit: vec![],
        priced: false,
    };
    let swaps = vec![
        swap(1, 0, USDC, 10_000_000, TKN, 100),
        swap(2, 50, TKN, 100, USDC, 90_000_000),
    ];
    let a = compute(&swaps, &unpriced, &BTreeMap::new());
    assert_eq!(a.realized_pnl_usd, None);
    assert_eq!(a.unpriced_swaps, 2);
    assert_eq!(a.coverage_ratio, Some(Decimal::ZERO));
}

#[test]
fn reorged_swaps_are_ignored_and_tentative_is_flagged() {
    let mut reorged = swap(1, 0, USDC, 10_000_000, TKN, 100);
    reorged.finality = Finality::Reorged;
    let mut tentative = swap(2, 10, USDC, 10_000_000, TKN, 100);
    tentative.finality = Finality::Tentative;
    let a = compute(&[reorged, tentative], &prices(), &BTreeMap::new());
    assert_eq!(a.total_swaps, 1);
    assert!(a.has_tentative);
}

#[test]
fn non_direct_swaps_are_not_credited() {
    let mut s = swap(1, 0, USDC, 10_000_000, TKN, 100);
    s.attribution = Attribution::Ambiguous;
    let a = compute(&[s], &prices(), &BTreeMap::new());
    assert_eq!(a.credited_swaps, 0);
    assert!(a.open_positions.is_empty());
    assert!(a
        .anomalies
        .iter()
        .any(|x| matches!(x, Anomaly::NotCreditedToWallet { .. })));
}

#[test]
fn input_order_does_not_change_the_answer() {
    let swaps = vec![
        swap(1, 0, USDC, 100_000_000, TKN, 1_000_000),
        swap(2, 200, TKN, 1_000_000, USDC, 300_000_000),
    ];
    let mut rev = swaps.clone();
    rev.reverse();
    assert_eq!(
        compute(&swaps, &prices(), &BTreeMap::new()),
        compute(&rev, &prices(), &BTreeMap::new())
    );
}

#[test]
fn unrealized_uses_a_timestamped_mark() {
    let swaps = vec![swap(1, 0, USDC, 100_000_000, TKN, 1_000_000)];
    let mut marks = BTreeMap::new();
    let at = Utc.timestamp_opt(1_793_000_500, 0).unwrap();
    marks.insert(
        asset(TKN),
        Mark {
            usd_per_base_unit: Decimal::from_str("0.0002").unwrap(),
            observed_at: at,
            source: "t".into(),
        },
    );
    let a = compute(&swaps, &prices(), &marks);
    assert_eq!(a.unrealized_pnl_usd, Some(Decimal::from(100))); // 1e6 * 0.0002 - 100
    assert_eq!(a.open_positions[0].mark_observed_at, Some(at));
    // without a mark there is no unrealized figure
    assert_eq!(
        compute(&swaps, &prices(), &BTreeMap::new()).unrealized_pnl_usd,
        None
    );
}

#[test]
fn huge_amounts_outside_decimal_range_are_flagged_not_wrapped() {
    let big = u128::MAX / 2;
    let swaps = vec![
        swap(1, 0, USDC, 1_000_000, TKN, big),
        swap(2, 10, TKN, big, USDC, 2_000_000),
    ];
    let a = compute(&swaps, &prices(), &BTreeMap::new());
    assert!(a
        .anomalies
        .iter()
        .any(|x| matches!(x, Anomaly::AmountOutOfDecimalRange { .. })));
    assert_eq!(a.realized_pnl_usd, None);
}

#[test]
fn confidence_needs_sample_size_and_coverage() {
    let mut swaps = vec![];
    let mut t = 0;
    for n in 0..6u32 {
        swaps.push(swap(n * 2 + 1, t, USDC, 10_000_000, TKN, 100));
        swaps.push(swap(n * 2 + 2, t + 10, TKN, 100, USDC, 12_000_000));
        t += 20;
    }
    let a = compute(&swaps, &prices(), &BTreeMap::new());
    assert_eq!(a.coverage_ratio, Some(Decimal::ONE));
    assert_eq!(a.pnl_confidence, PnlConfidence::High);
    let small = compute(&swaps[..2], &prices(), &BTreeMap::new());
    assert_eq!(small.pnl_confidence, PnlConfidence::Insufficient);
}

#[test]
fn stable_quote_legs_are_neutral_and_never_unresolved() {
    let swaps = vec![swap(1, 0, USDC, 100_000_000, TKN, 1_000_000)];
    let a = compute(&swaps, &prices(), &BTreeMap::new());
    assert_eq!(a.unresolved_disposals, 0);
    assert_eq!(a.coverage_ratio, Some(Decimal::ONE));
    assert!(a.open_positions.iter().all(|p| p.asset != USDC));
}

#[test]
fn divergent_two_sided_prices_are_flagged() {
    // TKN table says 1,000,000 units = $100 at t0, but swap paid only $10 USDC.
    let swaps = vec![swap(1, 0, TKN, 1_000_000, USDC, 10_000_000)];
    let a = compute(&swaps, &prices(), &BTreeMap::new());
    assert!(a
        .anomalies
        .iter()
        .any(|x| matches!(x, Anomaly::PriceDivergence { .. })));
}
