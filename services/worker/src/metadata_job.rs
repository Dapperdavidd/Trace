//! Read ERC-20 metadata from configured chain RPC nodes.

use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use curve25519_dalek::edwards::CompressedEdwardsY;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use trace_domain::ChainId;
use trace_jobs::Failure;

pub const FETCH_TOKEN_METADATA: &str = "fetch_token_metadata";
const MAX_TOKENS: i64 = 200;
const DECIMALS: &str = "0x313ce567";
const SYMBOL: &str = "0x95d89b41";
const NAME: &str = "0x06fdde03";
const METADATA_PROGRAM: &str = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";
const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
// Stable on-chain discriminants from the official Token-2022 interface.
const TOKEN_2022_MINT_ACCOUNT_TYPE: u8 = 1;
const TOKEN_2022_METADATA_POINTER: u16 = 18;
const TOKEN_2022_TOKEN_METADATA: u16 = 19;
const TOKEN_METADATA_DISCRIMINATOR: [u8; 8] = [112, 132, 90, 90, 11, 88, 157, 87];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenMetadata {
    pub decimals: u8,
    pub symbol: Option<String>,
    pub name: Option<String>,
}

#[async_trait]
pub trait MetadataSource: Send + Sync {
    async fn fetch(&self, chain: ChainId, token: &str) -> Result<Option<TokenMetadata>, Failure>;
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct MetadataReport {
    pub requested: usize,
    pub completed: usize,
    pub unavailable: usize,
    pub conflicts: usize,
}

pub struct EvmMetadataRpc {
    urls: HashMap<ChainId, String>,
    http: reqwest::Client,
}

impl EvmMetadataRpc {
    pub fn new(urls: HashMap<ChainId, String>) -> Result<Self, Failure> {
        let http = reqwest::Client::builder()
            .timeout(StdDuration::from_secs(20))
            .build()
            .map_err(|e| Failure::Permanent(e.without_url().to_string()))?;
        Ok(Self { urls, http })
    }

    async fn call(
        &self,
        chain: ChainId,
        token: &str,
        data: &str,
    ) -> Result<Option<Value>, Failure> {
        let url = self
            .urls
            .get(&chain)
            .ok_or_else(|| Failure::Permanent(format!("no metadata RPC for {chain}")))?;
        let body = json!({
            "jsonrpc": "2.0", "id": 1, "method": "eth_call",
            "params": [{ "to": token, "data": data }, "latest"]
        });
        let response = self.http.post(url).json(&body).send().await.map_err(|e| {
            Failure::Transient(format!("metadata RPC request failed: {}", e.without_url()))
        })?;
        let code = response.status();
        if code.as_u16() == 429 || code.is_server_error() {
            return Err(Failure::Transient(format!(
                "metadata RPC returned HTTP {code}"
            )));
        }
        if !code.is_success() {
            return Err(Failure::Permanent(format!(
                "metadata RPC returned HTTP {code}"
            )));
        }
        let value: Value = response
            .json()
            .await
            .map_err(|_| Failure::Transient("metadata RPC response was not JSON".into()))?;
        if value.get("error").is_some() {
            return Ok(None);
        }
        Ok(value.get("result").cloned())
    }
}

fn bytes(value: &Value) -> Option<Vec<u8>> {
    let raw = value.as_str()?.strip_prefix("0x")?;
    (!raw.is_empty() && raw.len() % 2 == 0)
        .then(|| hex::decode(raw).ok())
        .flatten()
}

fn decode_decimals(value: &Value) -> Option<u8> {
    let raw = bytes(value)?;
    if raw.len() != 32 || raw[..31].iter().any(|b| *b != 0) {
        return None;
    }
    Some(raw[31])
}

fn clean_text(raw: &[u8]) -> Option<String> {
    let value = std::str::from_utf8(raw)
        .ok()?
        .trim_matches(char::from(0))
        .trim();
    if value.is_empty() || value.chars().count() > 80 || value.chars().any(char::is_control) {
        return None;
    }
    Some(value.to_string())
}

fn decode_text(value: &Value) -> Option<String> {
    let raw = bytes(value)?;
    if raw.len() == 32 {
        return clean_text(&raw);
    }
    if raw.len() < 64 {
        return None;
    }
    let offset = usize::try_from(u64::from_be_bytes(raw[24..32].try_into().ok()?)).ok()?;
    if offset.checked_add(32)? > raw.len() {
        return None;
    }
    let length = usize::try_from(u64::from_be_bytes(
        raw[offset + 24..offset + 32].try_into().ok()?,
    ))
    .ok()?;
    let start = offset.checked_add(32)?;
    let end = start.checked_add(length)?;
    clean_text(raw.get(start..end)?)
}

#[async_trait]
impl MetadataSource for EvmMetadataRpc {
    async fn fetch(&self, chain: ChainId, token: &str) -> Result<Option<TokenMetadata>, Failure> {
        if !matches!(chain, ChainId::Ethereum | ChainId::Base | ChainId::Bnb) {
            return Ok(None);
        }
        let Some(decimals) = self.call(chain, token, DECIMALS).await? else {
            return Ok(None);
        };
        let Some(decimals) = decode_decimals(&decimals).filter(|d| *d <= 38) else {
            return Ok(None);
        };
        let symbol = match self.call(chain, token, SYMBOL).await? {
            Some(v) => decode_text(&v),
            None => None,
        };
        let name = match self.call(chain, token, NAME).await? {
            Some(v) => decode_text(&v),
            None => None,
        };
        Ok(Some(TokenMetadata {
            decimals,
            symbol,
            name,
        }))
    }
}

pub struct SolanaMetadataRpc {
    url: String,
    http: reqwest::Client,
}

impl SolanaMetadataRpc {
    pub fn new(url: String) -> Result<Self, Failure> {
        let http = reqwest::Client::builder()
            .timeout(StdDuration::from_secs(20))
            .build()
            .map_err(|error| Failure::Permanent(error.without_url().to_string()))?;
        Ok(Self { url, http })
    }

    async fn account(&self, address: &str) -> Result<Option<(String, Vec<u8>)>, Failure> {
        let body = json!({
            "jsonrpc": "2.0", "id": 1, "method": "getAccountInfo",
            "params": [address, { "encoding": "base64", "commitment": "finalized" }]
        });
        let response = self
            .http
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .map_err(|error| {
                Failure::Transient(format!(
                    "Solana metadata RPC request failed: {}",
                    error.without_url()
                ))
            })?;
        let status = response.status();
        if status.as_u16() == 429 || status.is_server_error() {
            return Err(Failure::Transient(format!(
                "Solana metadata RPC returned HTTP {status}"
            )));
        }
        if !status.is_success() {
            return Err(Failure::Permanent(format!(
                "Solana metadata RPC returned HTTP {status}"
            )));
        }
        let response: Value = response
            .json()
            .await
            .map_err(|_| Failure::Transient("Solana metadata RPC response was not JSON".into()))?;
        if response.get("error").is_some() {
            return Ok(None);
        }
        let Some(account) = response.pointer("/result/value") else {
            return Ok(None);
        };
        if account.is_null() {
            return Ok(None);
        }
        let owner = account["owner"]
            .as_str()
            .ok_or_else(|| Failure::Transient("Solana account has no owner".into()))?;
        let encoded = account["data"]
            .as_array()
            .and_then(|data| data.first())
            .and_then(Value::as_str)
            .ok_or_else(|| Failure::Transient("Solana account has no base64 data".into()))?;
        let data = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| Failure::Transient("Solana account data is not valid base64".into()))?;
        Ok(Some((owner.to_string(), data)))
    }
}

fn pubkey(value: &str) -> Option<[u8; 32]> {
    let decoded = bs58::decode(value).into_vec().ok()?;
    decoded.try_into().ok()
}

fn metadata_pda(mint: &[u8; 32]) -> Option<String> {
    let program = pubkey(METADATA_PROGRAM)?;
    for bump in (0..=255u8).rev() {
        let mut hash = Sha256::new();
        hash.update(b"metadata");
        hash.update(program);
        hash.update(mint);
        hash.update([bump]);
        hash.update(program);
        hash.update(b"ProgramDerivedAddress");
        let address: [u8; 32] = hash.finalize().into();
        if CompressedEdwardsY(address).decompress().is_none() {
            return Some(bs58::encode(address).into_string());
        }
    }
    None
}

fn borsh_text(data: &[u8], offset: &mut usize, max: usize) -> Option<String> {
    let end = offset.checked_add(4)?;
    let length = u32::from_le_bytes(data.get(*offset..end)?.try_into().ok()?) as usize;
    if length > max {
        return None;
    }
    let start = end;
    let end = start.checked_add(length)?;
    *offset = end;
    clean_text(data.get(start..end)?)
}

fn parse_metaplex(data: &[u8], expected_mint: &[u8; 32]) -> Option<(String, String)> {
    if data.len() < 65 || data[0] != 4 || data.get(33..65)? != expected_mint {
        return None;
    }
    let mut offset = 65;
    let name = borsh_text(data, &mut offset, 32)?;
    let symbol = borsh_text(data, &mut offset, 10)?;
    Some((name, symbol))
}

fn parse_solana_mint(owner: &str, data: &[u8]) -> Option<u8> {
    (matches!(owner, TOKEN_PROGRAM | TOKEN_2022_PROGRAM) && data.len() >= 46 && data[45] != 0)
        .then_some(data[44])
}

fn token_2022_extension(data: &[u8], wanted: u16) -> Option<&[u8]> {
    if data.len() <= 166
        || data.get(82..165)?.iter().any(|value| *value != 0)
        || data[165] != TOKEN_2022_MINT_ACCOUNT_TYPE
    {
        return None;
    }
    let mut offset = 166usize;
    while offset.checked_add(4)? <= data.len() {
        let extension_type = u16::from_le_bytes(data[offset..offset + 2].try_into().ok()?);
        let length = usize::from(u16::from_le_bytes(
            data[offset + 2..offset + 4].try_into().ok()?,
        ));
        let start = offset + 4;
        let end = start.checked_add(length)?;
        let value = data.get(start..end)?;
        if extension_type == wanted {
            return Some(value);
        }
        if extension_type == 0 {
            return None;
        }
        offset = end;
    }
    None
}

fn skip_borsh_text(data: &[u8], offset: &mut usize, max: usize) -> Option<()> {
    let end = offset.checked_add(4)?;
    let length =
        usize::try_from(u32::from_le_bytes(data.get(*offset..end)?.try_into().ok()?)).ok()?;
    if length > max {
        return None;
    }
    let start = end;
    let end = start.checked_add(length)?;
    std::str::from_utf8(data.get(start..end)?).ok()?;
    *offset = end;
    Some(())
}

fn token_2022_metadata_pointer(data: &[u8]) -> Option<[u8; 32]> {
    let pointer = token_2022_extension(data, TOKEN_2022_METADATA_POINTER)?;
    let address: [u8; 32] = pointer.get(32..64)?.try_into().ok()?;
    (pointer.len() == 64 && address != [0; 32]).then_some(address)
}

fn parse_token_metadata_value(
    metadata: &[u8],
    expected_mint: &[u8; 32],
) -> Option<(String, String)> {
    if metadata.len() < 64 || metadata.get(32..64)? != expected_mint {
        return None;
    }
    let mut offset = 64;
    let name = borsh_text(metadata, &mut offset, 80)?;
    let symbol = borsh_text(metadata, &mut offset, 20)?;
    skip_borsh_text(metadata, &mut offset, 2_048)?;
    let count_end = offset.checked_add(4)?;
    let additional = usize::try_from(u32::from_le_bytes(
        metadata.get(offset..count_end)?.try_into().ok()?,
    ))
    .ok()?;
    if additional > 64 {
        return None;
    }
    offset = count_end;
    for _ in 0..additional {
        skip_borsh_text(metadata, &mut offset, 128)?;
        skip_borsh_text(metadata, &mut offset, 1_024)?;
    }
    (offset == metadata.len()).then_some((name, symbol))
}

fn parse_token_2022_metadata(data: &[u8], expected_mint: &[u8; 32]) -> Option<(String, String)> {
    (token_2022_metadata_pointer(data)? == *expected_mint).then_some(())?;
    let metadata = token_2022_extension(data, TOKEN_2022_TOKEN_METADATA)?;
    parse_token_metadata_value(metadata, expected_mint)
}

fn parse_token_metadata_account(data: &[u8], expected_mint: &[u8; 32]) -> Option<(String, String)> {
    let mut offset = 0usize;
    while offset.checked_add(12)? <= data.len() {
        let discriminator: [u8; 8] = data[offset..offset + 8].try_into().ok()?;
        if discriminator == [0; 8] {
            return None;
        }
        let length = usize::try_from(u32::from_le_bytes(
            data[offset + 8..offset + 12].try_into().ok()?,
        ))
        .ok()?;
        let start = offset + 12;
        let end = start.checked_add(length)?;
        let value = data.get(start..end)?;
        if discriminator == TOKEN_METADATA_DISCRIMINATOR {
            return parse_token_metadata_value(value, expected_mint);
        }
        offset = end;
    }
    None
}

#[async_trait]
impl MetadataSource for SolanaMetadataRpc {
    async fn fetch(&self, chain: ChainId, token: &str) -> Result<Option<TokenMetadata>, Failure> {
        if chain != ChainId::Solana {
            return Ok(None);
        }
        let Some(mint) = pubkey(token) else {
            return Ok(None);
        };
        let Some((owner, mint_data)) = self.account(token).await? else {
            return Ok(None);
        };
        let Some(decimals) = parse_solana_mint(&owner, &mint_data) else {
            return Ok(None);
        };
        let pointer = (owner == TOKEN_2022_PROGRAM)
            .then(|| token_2022_metadata_pointer(&mint_data))
            .flatten();
        let canonical = match pointer {
            Some(address) if address == mint => parse_token_2022_metadata(&mint_data, &mint),
            Some(address) => match self.account(&bs58::encode(address).into_string()).await? {
                Some((_, data)) => parse_token_metadata_account(&data, &mint),
                None => None,
            },
            None => match metadata_pda(&mint) {
                Some(address) => match self.account(&address).await? {
                    Some((owner, data)) if owner == METADATA_PROGRAM => {
                        parse_metaplex(&data, &mint)
                    }
                    _ => None,
                },
                None => None,
            },
        };
        let (name, symbol) = canonical
            .map(|(name, symbol)| (Some(name), Some(symbol)))
            .unwrap_or((None, None));
        Ok(Some(TokenMetadata {
            decimals,
            symbol,
            name,
        }))
    }
}

pub struct RpcMetadataSource {
    evm: Option<EvmMetadataRpc>,
    solana: Option<SolanaMetadataRpc>,
}

impl RpcMetadataSource {
    pub fn new(
        evm_urls: HashMap<ChainId, String>,
        solana_url: Option<String>,
    ) -> Result<Self, Failure> {
        Ok(Self {
            evm: (!evm_urls.is_empty())
                .then(|| EvmMetadataRpc::new(evm_urls))
                .transpose()?,
            solana: solana_url.map(SolanaMetadataRpc::new).transpose()?,
        })
    }
}

#[async_trait]
impl MetadataSource for RpcMetadataSource {
    async fn fetch(&self, chain: ChainId, token: &str) -> Result<Option<TokenMetadata>, Failure> {
        match chain {
            ChainId::Solana => match &self.solana {
                Some(source) => source.fetch(chain, token).await,
                None => Ok(None),
            },
            _ => match &self.evm {
                Some(source) => source.fetch(chain, token).await,
                None => Ok(None),
            },
        }
    }
}

pub async fn fetch_missing(
    pool: &PgPool,
    source: &dyn MetadataSource,
    now: DateTime<Utc>,
) -> Result<MetadataReport, Failure> {
    let rows = sqlx::query(
        "SELECT chain, contract_or_mint, decimals FROM tokens
         WHERE chain IN ('ethereum','base','bnb','solana')
           AND (metadata_status='unknown' OR metadata_checked_at < $1)
           AND (decimals IS NULL OR symbol IS NULL OR name IS NULL)
         ORDER BY first_seen_at LIMIT $2",
    )
    .bind(now - Duration::days(7))
    .bind(MAX_TOKENS)
    .fetch_all(pool)
    .await
    .map_err(|e| Failure::Transient(e.to_string()))?;
    let mut report = MetadataReport {
        requested: rows.len(),
        ..Default::default()
    };
    for row in rows {
        let chain: ChainId = row
            .get::<String, _>("chain")
            .parse()
            .map_err(|e| Failure::Permanent(format!("invalid stored chain: {e}")))?;
        let token: String = row.get("contract_or_mint");
        let stored_decimals: Option<i16> = row.get("decimals");
        match source.fetch(chain, &token).await? {
            Some(meta) => {
                if stored_decimals.is_some_and(|value| value != i16::from(meta.decimals)) {
                    sqlx::query(
                        "INSERT INTO audit_events (id,actor,action,subject,detail)
                         SELECT $1,'metadata_worker','token_decimals_conflict',$2,$3
                         WHERE NOT EXISTS (
                           SELECT 1 FROM audit_events
                           WHERE action='token_decimals_conflict' AND subject=$2
                             AND detail->>'stored'=$4 AND detail->>'reported'=$5
                         )",
                    )
                    .bind(uuid::Uuid::new_v4())
                    .bind(format!("{}:{token}", chain.as_str()))
                    .bind(serde_json::json!({
                        "stored": stored_decimals,
                        "reported": meta.decimals,
                        "source": "chain_metadata_rpc",
                    }))
                    .bind(stored_decimals.unwrap().to_string())
                    .bind(meta.decimals.to_string())
                    .execute(pool)
                    .await
                    .map_err(|e| Failure::Transient(e.to_string()))?;
                    report.conflicts += 1;
                }
                sqlx::query(
                    "UPDATE tokens SET decimals=COALESCE(decimals,$3),
                       symbol=COALESCE(symbol,$4), name=COALESCE(name,$5),
                       metadata_status='complete', metadata_checked_at=$6
                     WHERE chain=$1 AND contract_or_mint=$2",
                )
                .bind(chain.as_str())
                .bind(&token)
                .bind(i16::from(meta.decimals))
                .bind(meta.symbol)
                .bind(meta.name)
                .bind(now)
                .execute(pool)
                .await
                .map_err(|e| Failure::Transient(e.to_string()))?;
                report.completed += 1;
            }
            None => {
                sqlx::query(
                    "UPDATE tokens SET metadata_status='unavailable', metadata_checked_at=$3
                     WHERE chain=$1 AND contract_or_mint=$2",
                )
                .bind(chain.as_str())
                .bind(&token)
                .bind(now)
                .execute(pool)
                .await
                .map_err(|e| Failure::Transient(e.to_string()))?;
                report.unavailable += 1;
            }
        }
    }
    Ok(report)
}

pub async fn schedule(
    pool: &PgPool,
    enabled: bool,
    now: DateTime<Utc>,
) -> Result<bool, trace_jobs::JobError> {
    if !enabled {
        return Ok(false);
    }
    let key = format!("{FETCH_TOKEN_METADATA}:{}", now.timestamp() / 300);
    trace_jobs::enqueue(pool, FETCH_TOKEN_METADATA, Some(&key), json!({}), now, 3).await?;
    Ok(true)
}

pub type SharedMetadataSource = Arc<dyn MetadataSource>;

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded_string(value: &str) -> Value {
        let mut data = vec![0u8; 64 + value.len().div_ceil(32) * 32];
        data[31] = 32;
        data[63] = value.len() as u8;
        data[64..64 + value.len()].copy_from_slice(value.as_bytes());
        Value::String(format!("0x{}", hex::encode(data)))
    }

    #[test]
    fn decodes_standard_and_bytes32_metadata() {
        let mut decimals = vec![0u8; 32];
        decimals[31] = 18;
        assert_eq!(
            decode_decimals(&Value::String(format!("0x{}", hex::encode(decimals)))),
            Some(18)
        );
        assert_eq!(
            decode_text(&encoded_string("Token Name")).as_deref(),
            Some("Token Name")
        );
        let mut bytes32 = [0u8; 32];
        bytes32[..3].copy_from_slice(b"TKN");
        assert_eq!(
            decode_text(&Value::String(format!("0x{}", hex::encode(bytes32)))).as_deref(),
            Some("TKN")
        );
    }

    #[test]
    fn derives_and_decodes_solana_metadata_without_guessing() {
        const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        let mint = pubkey(USDC).unwrap();
        assert_eq!(
            metadata_pda(&mint).as_deref(),
            Some("5x38Kp4hvdomTCnCrAny4UtMUt5rQBdB6px2K1Ui45Wq")
        );

        let mut metadata = vec![4u8];
        metadata.extend([0u8; 32]);
        metadata.extend(mint);
        for value in ["USD Coin", "USDC", "https://example.invalid"] {
            metadata.extend(u32::try_from(value.len()).unwrap().to_le_bytes());
            metadata.extend(value.as_bytes());
        }
        assert_eq!(
            parse_metaplex(&metadata, &mint),
            Some(("USD Coin".into(), "USDC".into()))
        );
        metadata[33] ^= 1;
        assert_eq!(parse_metaplex(&metadata, &mint), None, "mint must match");

        let mut mint_account = vec![0u8; 82];
        mint_account[44] = 6;
        mint_account[45] = 1;
        assert_eq!(parse_solana_mint(TOKEN_PROGRAM, &mint_account), Some(6));
        assert_eq!(parse_solana_mint("wrong-owner", &mint_account), None);
    }

    fn push_tlv(data: &mut Vec<u8>, extension: u16, value: &[u8]) {
        data.extend(extension.to_le_bytes());
        data.extend(u16::try_from(value.len()).unwrap().to_le_bytes());
        data.extend(value);
    }

    fn push_borsh_text(data: &mut Vec<u8>, value: &str) {
        data.extend(u32::try_from(value.len()).unwrap().to_le_bytes());
        data.extend(value.as_bytes());
    }

    #[test]
    fn token_2022_metadata_requires_a_two_way_mint_pointer() {
        let mint = [7u8; 32];
        let mut account = vec![0u8; 166];
        account[44] = 9;
        account[45] = 1;
        account[165] = TOKEN_2022_MINT_ACCOUNT_TYPE;

        let mut pointer = vec![0u8; 32];
        pointer.extend(mint);
        push_tlv(&mut account, TOKEN_2022_METADATA_POINTER, &pointer);

        let mut metadata = vec![0u8; 32];
        metadata.extend(mint);
        push_borsh_text(&mut metadata, "Extension Token");
        push_borsh_text(&mut metadata, "EXT");
        push_borsh_text(&mut metadata, "https://example.invalid");
        metadata.extend(0u32.to_le_bytes());
        push_tlv(&mut account, TOKEN_2022_TOKEN_METADATA, &metadata);

        assert_eq!(
            parse_token_2022_metadata(&account, &mint),
            Some(("Extension Token".into(), "EXT".into()))
        );

        let mut wrong_pointer = account.clone();
        wrong_pointer[166 + 4 + 32] ^= 1;
        assert_eq!(parse_token_2022_metadata(&wrong_pointer, &mint), None);

        let mut wrong_metadata_mint = account;
        let metadata_mint = 166 + 4 + pointer.len() + 4 + 32;
        wrong_metadata_mint[metadata_mint] ^= 1;
        assert_eq!(parse_token_2022_metadata(&wrong_metadata_mint, &mint), None);
    }

    #[test]
    fn external_token_metadata_interface_account_must_point_back_to_mint() {
        let mint = [9u8; 32];
        let mut value = vec![0u8; 32];
        value.extend(mint);
        push_borsh_text(&mut value, "External Token");
        push_borsh_text(&mut value, "XTK");
        push_borsh_text(&mut value, "");
        value.extend(0u32.to_le_bytes());

        let mut account = TOKEN_METADATA_DISCRIMINATOR.to_vec();
        account.extend(u32::try_from(value.len()).unwrap().to_le_bytes());
        account.extend(&value);
        assert_eq!(
            parse_token_metadata_account(&account, &mint),
            Some(("External Token".into(), "XTK".into()))
        );

        let mut wrong_mint = account.clone();
        wrong_mint[12 + 32] ^= 1;
        assert_eq!(parse_token_metadata_account(&wrong_mint, &mint), None);
        account[0] ^= 1;
        assert_eq!(parse_token_metadata_account(&account, &mint), None);
    }

    #[tokio::test]
    async fn rpc_failures_never_reveal_the_url_or_key() {
        let source = EvmMetadataRpc::new(HashMap::from([(
            ChainId::Ethereum,
            "http://127.0.0.1:9/private-key".to_string(),
        )]))
        .unwrap();
        let error = format!(
            "{:?}",
            source
                .fetch(
                    ChainId::Ethereum,
                    "0x00000000000000000000000000000000000000f1",
                )
                .await
                .unwrap_err()
        );
        assert!(!error.contains("private-key"));
        assert!(!error.contains("127.0.0.1"));

        let solana = SolanaMetadataRpc::new("http://127.0.0.1:9/solana-key".into()).unwrap();
        let error = format!(
            "{:?}",
            solana
                .fetch(
                    ChainId::Solana,
                    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                )
                .await
                .unwrap_err()
        );
        assert!(!error.contains("solana-key"));
        assert!(!error.contains("127.0.0.1"));
    }
}
