//! DexScreener liquidity parsing against a local mock server: pool keys, chain
//! filtering, and that refused or malformed answers yield no liquidity at all.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use trace_worker::market_job::DexScreenerLiquidity;

async fn serve_once(status: &'static str, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 4096];
        let _ = sock.read(&mut buf).await;
        let resp = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        sock.write_all(resp.as_bytes()).await.unwrap();
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn evm_pools_are_keyed_lowercase_and_other_chains_are_ignored() {
    let base = serve_once(
        "200 OK",
        r#"{"pairs":[
          {"chainId":"ethereum","pairAddress":"0xABCdef0000000000000000000000000000000001","liquidity":{"usd":31181165.554}},
          {"chainId":"bsc","pairAddress":"0x0000000000000000000000000000000000000002","liquidity":{"usd":5}},
          {"chainId":"ethereum","pairAddress":"0x0000000000000000000000000000000000000003"}
        ]}"#,
    )
    .await;
    let src = DexScreenerLiquidity::new(base);
    let got = src
        .fetch(
            "ethereum",
            &["0xabcdef0000000000000000000000000000000001".to_string()],
        )
        .await;
    assert_eq!(got.len(), 1);
    assert_eq!(
        got["0xabcdef0000000000000000000000000000000001"].to_string(),
        "31181165.55"
    );
}

#[tokio::test]
async fn solana_pool_case_is_preserved() {
    let base = serve_once(
        "200 OK",
        r#"{"pairs":[{"chainId":"solana","pairAddress":"38dtzoUQBgCtoVyVnix2XyG1HNrWCLchyYYejEkh5qLn","liquidity":{"usd":15665.31}}]}"#,
    )
    .await;
    let got = DexScreenerLiquidity::new(base)
        .fetch(
            "solana",
            &["38dtzoUQBgCtoVyVnix2XyG1HNrWCLchyYYejEkh5qLn".to_string()],
        )
        .await;
    assert!(got.contains_key("38dtzoUQBgCtoVyVnix2XyG1HNrWCLchyYYejEkh5qLn"));
}

#[tokio::test]
async fn refused_requests_give_no_liquidity() {
    let base = serve_once("429 Too Many Requests", r#"{"error":"slow down"}"#).await;
    let got = DexScreenerLiquidity::new(base)
        .fetch("ethereum", &["0x01".to_string()])
        .await;
    assert!(got.is_empty());
}
