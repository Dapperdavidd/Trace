-- Exact arguments are retained so an anchor can be verified against NEAR RPC.
ALTER TABLE near_agent_mandates
    ADD COLUMN contract_args JSONB NOT NULL DEFAULT '{}';

