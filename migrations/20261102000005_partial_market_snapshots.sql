-- A real price and volume observation must not invent liquidity.
-- Discovery and Shadow remain gated until a reviewed liquidity source fills it.
ALTER TABLE market_snapshots ALTER COLUMN liquidity_usd DROP NOT NULL;
