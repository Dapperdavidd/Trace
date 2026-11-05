use near_sdk::{
    env, near, require,
    store::{IterableMap, LookupMap},
    AccountId, PanicOnDefault,
};

const MAX_SLIPPAGE_BPS: u16 = 2_000;
const MAX_LIST_ITEMS: usize = 64;

#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MandateMode {
    Paper,
    ApprovalRequired,
    Delegated,
}

#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MandateStatus {
    Active,
    Paused,
    Revoked,
}

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct Mandate {
    pub id: String,
    pub owner: AccountId,
    pub agent: AccountId,
    pub strategy_id: String,
    pub mode: MandateMode,
    pub status: MandateStatus,
    pub allowed_chains: Vec<String>,
    pub allowed_assets: Vec<String>,
    pub max_trade_usd_micros: u128,
    pub max_daily_usd_micros: u128,
    pub max_slippage_bps: u16,
    pub expires_at_ms: u64,
    pub nonce: u64,
    pub created_at_ms: u64,
}

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct ExecutionReceipt {
    pub id: String,
    pub mandate_id: String,
    pub chain: String,
    pub origin_asset: String,
    pub destination_asset: String,
    pub amount_usd_micros: u128,
    pub slippage_bps: u16,
    pub intent_correlation_id: String,
    /// SHA-256 commitment to the complete evidence retained by TRACE.
    pub evidence_hash: String,
    pub executed_at_ms: u64,
    pub recorded_by: AccountId,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct TraceMandates {
    mandates: IterableMap<String, Mandate>,
    receipts: IterableMap<String, ExecutionReceipt>,
    /// Keyed by `<mandate_id>:<UTC day number>`.
    daily_spend: LookupMap<String, u128>,
}

#[near]
impl TraceMandates {
    #[init]
    pub fn new() -> Self {
        Self {
            mandates: IterableMap::new(&b"mandates"[..]),
            receipts: IterableMap::new(&b"receipts"[..]),
            daily_spend: LookupMap::new(&b"daily-spend"[..]),
        }
    }

    /// Creates a mandate owned by the caller. No caller may create authority
    /// for another user, and IDs cannot be overwritten.
    pub fn create_mandate(&mut self, mandate: Mandate) {
        require!(
            env::predecessor_account_id() == mandate.owner,
            "owner must create mandate"
        );
        require!(
            !self.mandates.contains_key(&mandate.id),
            "mandate already exists"
        );
        validate_mandate(&mandate);
        self.mandates.insert(mandate.id.clone(), mandate);
    }

    pub fn set_mandate_status(&mut self, id: String, status: MandateStatus) {
        let mandate = self.mandates.get_mut(&id).expect("mandate not found");
        require!(
            env::predecessor_account_id() == mandate.owner,
            "only owner may change mandate"
        );
        require!(
            mandate.status != MandateStatus::Revoked,
            "revocation is permanent"
        );
        mandate.status = status;
    }

    pub fn get_mandate(&self, id: String) -> Option<Mandate> {
        self.mandates.get(&id).cloned()
    }

    pub fn get_receipt(&self, id: String) -> Option<ExecutionReceipt> {
        self.receipts.get(&id).cloned()
    }

    /// Records a policy-checked result. Approval-required receipts may be
    /// recorded by the owner after signing; delegated receipts may be recorded
    /// by the specifically named agent. Paper mandates cannot claim execution.
    pub fn record_execution(&mut self, receipt: ExecutionReceipt) {
        require!(
            !self.receipts.contains_key(&receipt.id),
            "receipt already exists"
        );
        let mandate = self
            .mandates
            .get(&receipt.mandate_id)
            .expect("mandate not found");
        require!(
            mandate.status == MandateStatus::Active,
            "mandate is not active"
        );
        require!(mandate.expires_at_ms > now_ms(), "mandate expired");
        require!(
            mandate.mode != MandateMode::Paper,
            "paper mandate cannot execute"
        );
        let caller = env::predecessor_account_id();
        match mandate.mode {
            MandateMode::ApprovalRequired => {
                require!(caller == mandate.owner, "owner approval required")
            }
            MandateMode::Delegated => require!(caller == mandate.agent, "unauthorized agent"),
            MandateMode::Paper => unreachable!(),
        }
        require!(
            mandate.allowed_chains.contains(&receipt.chain),
            "chain not allowed"
        );
        require!(
            mandate.allowed_assets.contains(&receipt.origin_asset),
            "origin asset not allowed"
        );
        require!(
            mandate.allowed_assets.contains(&receipt.destination_asset),
            "destination asset not allowed"
        );
        require!(receipt.amount_usd_micros > 0, "amount must be positive");
        require!(
            receipt.amount_usd_micros <= mandate.max_trade_usd_micros,
            "trade limit exceeded"
        );
        require!(
            receipt.slippage_bps <= mandate.max_slippage_bps,
            "slippage limit exceeded"
        );
        require!(
            receipt.executed_at_ms <= now_ms() + 60_000,
            "execution time is in the future"
        );
        require!(
            receipt.evidence_hash.starts_with("sha256:"),
            "evidence hash must use sha256"
        );

        let day = receipt.executed_at_ms / 86_400_000;
        let spend_key = format!("{}:{day}", receipt.mandate_id);
        let spent = self.daily_spend.get(&spend_key).copied().unwrap_or(0);
        let next = spent
            .checked_add(receipt.amount_usd_micros)
            .expect("daily spend overflow");
        require!(next <= mandate.max_daily_usd_micros, "daily limit exceeded");
        self.daily_spend.insert(spend_key, next);

        let receipt = ExecutionReceipt {
            recorded_by: caller,
            ..receipt
        };
        self.receipts.insert(receipt.id.clone(), receipt);
    }
}

fn validate_mandate(mandate: &Mandate) {
    require!(!mandate.id.trim().is_empty(), "id is required");
    require!(
        !mandate.strategy_id.trim().is_empty(),
        "strategy_id is required"
    );
    require!(
        mandate.status == MandateStatus::Active,
        "new mandate must be active"
    );
    require!(
        !mandate.allowed_chains.is_empty(),
        "allowed_chains is required"
    );
    require!(
        mandate.allowed_chains.len() <= MAX_LIST_ITEMS,
        "too many allowed chains"
    );
    require!(
        mandate.allowed_assets.len() <= MAX_LIST_ITEMS,
        "too many allowed assets"
    );
    require!(
        mandate.mode == MandateMode::Paper || !mandate.allowed_assets.is_empty(),
        "live mandate requires allowed assets"
    );
    require!(
        mandate.max_trade_usd_micros > 0,
        "trade limit must be positive"
    );
    require!(
        mandate.max_daily_usd_micros >= mandate.max_trade_usd_micros,
        "daily limit below trade limit"
    );
    require!(
        mandate.max_slippage_bps <= MAX_SLIPPAGE_BPS,
        "slippage limit too high"
    );
    require!(
        mandate.expires_at_ms > now_ms(),
        "mandate must expire in the future"
    );
    require!(
        mandate.mode != MandateMode::Delegated || mandate.nonce > 0,
        "delegated mandate requires nonce"
    );
}

fn now_ms() -> u64 {
    env::block_timestamp_ms()
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;

    fn account(value: &str) -> AccountId {
        value.parse().unwrap()
    }

    fn context(caller: &str, now_ms: u64) {
        let mut builder = VMContextBuilder::new();
        builder
            .predecessor_account_id(account(caller))
            .block_timestamp(now_ms * 1_000_000);
        testing_env!(builder.build());
    }

    fn mandate(mode: MandateMode) -> Mandate {
        Mandate {
            id: "mandate-1".into(),
            owner: account("alice.near"),
            agent: account("agent.trace.near"),
            strategy_id: "strategy-1".into(),
            mode,
            status: MandateStatus::Active,
            allowed_chains: vec!["eth".into()],
            allowed_assets: vec![
                "nep141:eth.omft.near".into(),
                "nep141:eth-0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.omft.near".into(),
            ],
            max_trade_usd_micros: 100_000_000,
            max_daily_usd_micros: 200_000_000,
            max_slippage_bps: 100,
            expires_at_ms: 2_000_000,
            nonce: 1,
            created_at_ms: 1_000_000,
        }
    }

    fn receipt(id: &str) -> ExecutionReceipt {
        ExecutionReceipt {
            id: id.into(),
            mandate_id: "mandate-1".into(),
            chain: "eth".into(),
            origin_asset: "nep141:eth.omft.near".into(),
            destination_asset: "nep141:eth-0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.omft.near"
                .into(),
            amount_usd_micros: 75_000_000,
            slippage_bps: 50,
            intent_correlation_id: "quote-1".into(),
            evidence_hash: format!("sha256:{}", "a".repeat(64)),
            executed_at_ms: 1_000_000,
            recorded_by: account("placeholder.near"),
        }
    }

    #[test]
    fn owner_creates_and_revokes_mandate() {
        context("alice.near", 1_000_000);
        let mut contract = TraceMandates::new();
        contract.create_mandate(mandate(MandateMode::ApprovalRequired));
        contract.set_mandate_status("mandate-1".into(), MandateStatus::Revoked);
        assert_eq!(
            contract.get_mandate("mandate-1".into()).unwrap().status,
            MandateStatus::Revoked
        );
    }

    #[test]
    fn delegated_agent_records_only_within_limits() {
        context("alice.near", 1_000_000);
        let mut contract = TraceMandates::new();
        contract.create_mandate(mandate(MandateMode::Delegated));
        context("agent.trace.near", 1_000_000);
        contract.record_execution(receipt("receipt-1"));
        assert_eq!(
            contract
                .get_receipt("receipt-1".into())
                .unwrap()
                .recorded_by,
            account("agent.trace.near")
        );
    }

    #[test]
    #[should_panic(expected = "daily limit exceeded")]
    fn daily_limit_is_enforced_on_chain() {
        context("alice.near", 1_000_000);
        let mut contract = TraceMandates::new();
        contract.create_mandate(mandate(MandateMode::Delegated));
        context("agent.trace.near", 1_000_000);
        contract.record_execution(receipt("receipt-1"));
        contract.record_execution(receipt("receipt-2"));
        contract.record_execution(receipt("receipt-3"));
    }

    #[test]
    #[should_panic(expected = "owner approval required")]
    fn agent_cannot_execute_approval_required_mandate() {
        context("alice.near", 1_000_000);
        let mut contract = TraceMandates::new();
        contract.create_mandate(mandate(MandateMode::ApprovalRequired));
        context("agent.trace.near", 1_000_000);
        contract.record_execution(receipt("receipt-1"));
    }
}
