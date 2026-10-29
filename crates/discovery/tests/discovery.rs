use chrono::{Duration, TimeZone, Utc};
use rust_decimal::Decimal;
use std::{collections::BTreeSet, str::FromStr};
use trace_discovery::{buyers, candidates::*, movers::*};
use trace_domain::*;
use trace_risk::RiskState;

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}
fn now() -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(1_793_100_000, 0).unwrap()
}

fn swap(wallet: &str, n: u32, secs: i64, attribution: Attribution) -> NormalizedSwap {
    NormalizedSwap {
        chain: ChainId::Ethereum,
        tx_id: format!("0x{n:064x}"),
        event_index: 0,
        finality_revision: 0,
        wallet: WalletId::parse(ChainId::Ethereum, wallet).unwrap(),
        protocol: "uniswap_v2".into(),
        pool: "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc".into(),
        token_in: AssetId::parse(
            ChainId::Ethereum,
            "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
        )
        .unwrap(),
        amount_in: BaseUnits(1),
        token_out: AssetId::parse(ChainId::Ethereum, &format!("0x{:040x}", 1000 + (n % 7)))
            .unwrap(),
        amount_out: BaseUnits(1),
        block_height: None,
        block_hash: None,
        observed_at: now() + Duration::seconds(secs),
        finality: Finality::Final,
        attribution,
        parser_version: "t".into(),
        raw_ref: "t".into(),
        demo: true,
    }
}

const W1: &str = "0x1111111111111111111111111111111111111111";
const W2: &str = "0x2222222222222222222222222222222222222222";

#[test]
fn only_direct_swaps_are_credited_and_the_rest_is_counted() {
    let swaps = vec![
        swap(W1, 1, 0, Attribution::Direct),
        swap(W1, 2, 60, Attribution::Direct),
        swap(W1, 3, 90, Attribution::Ambiguous),
        swap(W2, 4, 0, Attribution::Routed),
    ];
    let s = summarize(&swaps);
    assert_eq!(
        s.len(),
        1,
        "wallets with no direct swaps are not summarised"
    );
    assert_eq!(s[0].trade_count, 2);
    assert_eq!(s[0].non_direct_swaps, 1);
    assert_eq!(s[0].median_gap_seconds, Some(60));
}

#[test]
fn candidates_need_enough_activity() {
    let swaps: Vec<_> = (0..3)
        .map(|i| swap(W1, i, i as i64 * 600, Attribution::Direct))
        .collect();
    let a = &summarize(&swaps)[0];
    assert_eq!(
        classify(&CandidateConfig::default(), a, &BTreeSet::new()),
        CandidateDecision::WaitingForActivity { have: 3, need: 5 }
    );
}

#[test]
fn enough_activity_makes_a_candidate() {
    let swaps: Vec<_> = (0..8)
        .map(|i| swap(W1, i, i as i64 * 600, Attribution::Direct))
        .collect();
    assert_eq!(
        classify(
            &CandidateConfig::default(),
            &summarize(&swaps)[0],
            &BTreeSet::new()
        ),
        CandidateDecision::Candidate
    );
}

#[test]
fn known_infrastructure_is_excluded() {
    let swaps: Vec<_> = (0..8)
        .map(|i| swap(W1, i, i as i64 * 600, Attribution::Direct))
        .collect();
    let infra: BTreeSet<_> = [Address::parse(ChainId::Ethereum, W1).unwrap()].into();
    assert_eq!(
        classify(&CandidateConfig::default(), &summarize(&swaps)[0], &infra),
        CandidateDecision::ExcludedInfrastructure
    );
}

#[test]
fn bot_like_cadence_is_marked_uncertain_not_silently_dropped() {
    let swaps: Vec<_> = (0..40)
        .map(|i| swap(W1, i, i as i64, Attribution::Direct))
        .collect();
    let c = classify(
        &CandidateConfig::default(),
        &summarize(&swaps)[0],
        &BTreeSet::new(),
    );
    assert!(matches!(c, CandidateDecision::UncertainBot { .. }));
}

// ----- independent buyers -----

fn s(x: &str) -> String {
    x.to_string()
}

#[test]
fn verified_links_collapse_wallets_and_weak_links_widen_the_range() {
    let buyers_list = vec![s("a"), s("b"), s("c"), s("d")];
    let verified = vec![(s("a"), s("b"))];
    let weak = vec![(s("c"), s("d")), (s("b"), s("c"))];
    let r = buyers::count(&buyers_list, &verified, &weak);
    assert_eq!(r.raw_wallets, 4);
    assert_eq!(r.max_independent, 3, "only the verified a-b link collapses");
    assert_eq!(r.min_independent, 1, "weak links also connect c-d and b-c");
    assert_eq!(r.weakly_linked_groups.len(), 1);
}

#[test]
fn no_links_means_every_buyer_is_independent() {
    let r = buyers::count(&[s("a"), s("b")], &[], &[]);
    assert_eq!((r.max_independent, r.min_independent), (2, 2));
    assert!(r.weakly_linked_groups.is_empty());
}

#[test]
fn duplicate_buyers_are_not_double_counted() {
    assert_eq!(
        buyers::count(&[s("a"), s("a"), s("b")], &[], &[]).raw_wallets,
        2
    );
}

#[test]
fn timing_correlation_across_several_tokens_is_a_weak_link() {
    let mk = |w: &str, token: &str, secs: i64| buyers::Buy {
        wallet: w.into(),
        token: token.into(),
        at: now() + Duration::seconds(secs),
    };
    let buys = vec![
        mk("a", "t1", 0),
        mk("b", "t1", 20),
        mk("a", "t2", 100),
        mk("b", "t2", 110),
        mk("a", "t3", 500),
        mk("b", "t3", 505),
        mk("c", "t1", 5),
    ];
    let links = buyers::detect_weak_links(&buys, Duration::seconds(60), 3);
    assert_eq!(links, vec![(s("a"), s("b"))]);
    // one coincidence is not enough
    assert!(buyers::detect_weak_links(&buys, Duration::seconds(60), 4).is_empty());
}

// ----- movers -----

#[allow(clippy::too_many_arguments)]
fn snap(
    sym: &str,
    id: u32,
    now_p: &str,
    start_p: &str,
    vol: &str,
    base: Option<&str>,
    liq: &str,
    age: Option<u32>,
    smart: u32,
    risk: RiskState,
    age_secs: i64,
) -> Snapshot {
    Snapshot {
        token: AssetId::parse(ChainId::Ethereum, &format!("0x{id:040x}")).unwrap(),
        symbol: Some(sym.into()),
        pool: "pool".into(),
        price_now_usd: d(now_p),
        price_window_start_usd: d(start_p),
        volume_window_usd: d(vol),
        volume_baseline_usd: base.map(d),
        liquidity_usd: d(liq),
        pool_age_hours: age,
        independent_smart_buyers: smart,
        large_buyer_flows: 0,
        risk,
        last_updated: now() - Duration::seconds(age_secs),
    }
}

#[test]
fn thin_liquidity_pumps_stale_data_and_blocked_tokens_are_excluded_and_counted() {
    let snaps = vec![
        snap(
            "GOOD",
            1,
            "1.4",
            "1.0",
            "900000",
            Some("100000"),
            "500000",
            None,
            0,
            RiskState::Pass,
            30,
        ),
        snap(
            "THIN",
            2,
            "9",
            "1",
            "900000",
            Some("1000"),
            "2000",
            None,
            3,
            RiskState::Pass,
            30,
        ),
        snap(
            "OLD",
            3,
            "2",
            "1",
            "900000",
            Some("100000"),
            "500000",
            None,
            0,
            RiskState::Pass,
            7_200,
        ),
        snap(
            "RUG",
            4,
            "2",
            "1",
            "900000",
            Some("100000"),
            "500000",
            None,
            4,
            RiskState::Block,
            30,
        ),
        snap(
            "BADPX",
            5,
            "0",
            "1",
            "900000",
            Some("100000"),
            "500000",
            None,
            0,
            RiskState::Pass,
            30,
        ),
    ];
    let (ranked, ex) = rank(&MoverConfig::default(), Tab::Trending, &snaps, now());
    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].symbol.as_deref(), Some("GOOD"));
    assert_eq!(ranked[0].reason, ReasonCode::HighVolumeSpike);
    assert_eq!(
        (
            ex.thin_liquidity,
            ex.stale,
            ex.blocked_risk,
            ex.invalid_price
        ),
        (1, 1, 1, 1)
    );
    assert_eq!(ranked[0].rank, 1);
    assert_eq!(ranked[0].source_timestamp, now() - Duration::seconds(30));
}

#[test]
fn tabs_filter_and_label_reasons_honestly() {
    let snaps = vec![
        snap(
            "SMART",
            1,
            "1.2",
            "1.0",
            "300000",
            Some("200000"),
            "400000",
            None,
            3,
            RiskState::Pass,
            10,
        ),
        snap(
            "FRESH",
            2,
            "1.1",
            "1.0",
            "100000",
            Some("90000"),
            "60000",
            Some(5),
            0,
            RiskState::Warn,
            10,
        ),
        snap(
            "PLAIN",
            3,
            "1.5",
            "1.0",
            "100000",
            Some("90000"),
            "800000",
            None,
            0,
            RiskState::Pass,
            10,
        ),
    ];
    let cfg = MoverConfig::default();
    let smart = rank(&cfg, Tab::SmartMoney, &snaps, now()).0;
    assert_eq!(
        smart
            .iter()
            .map(|m| m.symbol.clone().unwrap())
            .collect::<Vec<_>>(),
        vec!["SMART"]
    );
    assert_eq!(smart[0].reason, ReasonCode::SmartWalletsBuying);
    let fresh = rank(&cfg, Tab::New, &snaps, now()).0;
    assert_eq!(fresh[0].symbol.as_deref(), Some("FRESH"));
    assert_eq!(fresh[0].reason, ReasonCode::NewPool);
    // PLAIN has no explainable reason, so it is not listed anywhere
    assert!(rank(&cfg, Tab::Trending, &snaps, now())
        .0
        .iter()
        .all(|m| m.symbol.as_deref() != Some("PLAIN")));
}

#[test]
fn ranking_is_deterministic() {
    let snaps = vec![
        snap(
            "A",
            1,
            "1.5",
            "1.0",
            "900000",
            Some("100000"),
            "500000",
            None,
            0,
            RiskState::Pass,
            30,
        ),
        snap(
            "B",
            2,
            "1.5",
            "1.0",
            "900000",
            Some("100000"),
            "500000",
            None,
            0,
            RiskState::Pass,
            30,
        ),
    ];
    let a = rank(&MoverConfig::default(), Tab::Trending, &snaps, now());
    let mut rev = snaps.clone();
    rev.reverse();
    assert_eq!(a, rank(&MoverConfig::default(), Tab::Trending, &rev, now()));
}
