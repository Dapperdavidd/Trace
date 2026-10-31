//! The DefiLlama adapter against a local server. Synthetic: not a live check.

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use trace_domain::{AssetId, ChainId};
use trace_pricing::defillama::DefiLlama;
use trace_pricing::{PriceProvider, Reason};

const WETH: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
const USDC: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";

async fn serve(
    status: u16,
    body: String,
) -> (String, tokio::sync::mpsc::UnboundedReceiver<String>) {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", l.local_addr().unwrap());
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        loop {
            let (mut s, _) = l.accept().await.unwrap();
            let mut buf = vec![0u8; 8192];
            let n = s.read(&mut buf).await.unwrap_or(0);
            let line = String::from_utf8_lossy(&buf[..n])
                .lines()
                .next()
                .unwrap_or_default()
                .to_string();
            let _ = tx.send(line);
            let resp = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = s.write_all(resp.as_bytes()).await;
        }
    });
    (url, rx)
}

fn assets() -> Vec<AssetId> {
    vec![
        AssetId::parse(ChainId::Ethereum, WETH).unwrap(),
        AssetId::parse(ChainId::Ethereum, USDC).unwrap(),
    ]
}

#[tokio::test]
async fn it_requests_chain_prefixed_keys_and_parses_the_answer() {
    let body = json!({ "coins": { format!("ethereum:{WETH}"): {
        "decimals": 18, "symbol": "WETH", "price": 3000.25, "timestamp": 1_792_999_990u64, "confidence": 0.99 } } });
    let (url, mut rx) = serve(200, body.to_string()).await;
    let b = DefiLlama::new(&url)
        .unwrap()
        .fetch(&assets())
        .await
        .unwrap();
    assert_eq!(b.quotes.len(), 1);
    assert_eq!(b.rejected.len(), 1);
    assert_eq!(
        b.rejected[0].1,
        Reason::Missing,
        "an asset the provider omitted is reported, not guessed"
    );
    let line = rx.recv().await.unwrap();
    assert!(
        line.starts_with("GET /prices/current/ethereum:0xc02aaa39"),
        "{line}"
    );
    assert!(line.contains(&format!("ethereum:{USDC}")));
}

#[tokio::test]
async fn http_errors_and_garbage_are_provider_errors() {
    let (url, _) = serve(503, "{}".into()).await;
    assert!(DefiLlama::new(&url)
        .unwrap()
        .fetch(&assets())
        .await
        .is_err());
    let (url, _) = serve(200, "not json".into()).await;
    assert!(DefiLlama::new(&url)
        .unwrap()
        .fetch(&assets())
        .await
        .is_err());
}

#[tokio::test]
async fn an_unreachable_provider_error_does_not_leak_the_url() {
    let e = DefiLlama::new("http://127.0.0.1:1/SECRET-KEY")
        .unwrap()
        .fetch(&assets())
        .await
        .unwrap_err()
        .to_string();
    assert!(!e.contains("SECRET-KEY") && !e.contains("127.0.0.1"), "{e}");
}
