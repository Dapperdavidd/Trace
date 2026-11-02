use sqlx::FromRow;

#[derive(Debug, FromRow, PartialEq, Eq)]
struct StableAsset {
    chain: String,
    contract_or_mint: String,
    symbol: Option<String>,
    name: Option<String>,
    decimals: Option<i16>,
    is_stable: bool,
}

#[sqlx::test(migrator = "trace_database::MIGRATOR")]
async fn migration_installs_reviewed_stable_assets(pool: sqlx::PgPool) {
    let actual = sqlx::query_as::<_, StableAsset>(
        "SELECT chain, contract_or_mint, symbol, name, decimals, is_stable
         FROM tokens WHERE is_stable
         ORDER BY chain, contract_or_mint",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let expected = vec![
        StableAsset {
            chain: "base".into(),
            contract_or_mint: "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913".into(),
            symbol: Some("USDC".into()),
            name: Some("USD Coin".into()),
            decimals: Some(6),
            is_stable: true,
        },
        StableAsset {
            chain: "ethereum".into(),
            contract_or_mint: "0x6b175474e89094c44da98b954eedeac495271d0f".into(),
            symbol: Some("DAI".into()),
            name: Some("Dai Stablecoin".into()),
            decimals: Some(18),
            is_stable: true,
        },
        StableAsset {
            chain: "ethereum".into(),
            contract_or_mint: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".into(),
            symbol: Some("USDC".into()),
            name: Some("USD Coin".into()),
            decimals: Some(6),
            is_stable: true,
        },
        StableAsset {
            chain: "ethereum".into(),
            contract_or_mint: "0xdac17f958d2ee523a2206206994597c13d831ec7".into(),
            symbol: Some("USDT".into()),
            name: Some("Tether USD".into()),
            decimals: Some(6),
            is_stable: true,
        },
        StableAsset {
            chain: "solana".into(),
            contract_or_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".into(),
            symbol: Some("USDC".into()),
            name: Some("USD Coin".into()),
            decimals: Some(6),
            is_stable: true,
        },
        StableAsset {
            chain: "solana".into(),
            contract_or_mint: "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".into(),
            symbol: Some("USDT".into()),
            name: Some("Tether USD".into()),
            decimals: Some(6),
            is_stable: true,
        },
    ];

    assert_eq!(actual, expected);
}
