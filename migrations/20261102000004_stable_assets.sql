-- Reviewed stable assets used by accounting and PnL.
-- EVM addresses are stored lowercase to match normalized swap assets.

INSERT INTO tokens (chain, contract_or_mint, symbol, name, decimals, is_stable) VALUES
  ('ethereum', '0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48', 'USDC', 'USD Coin',       6,  TRUE),
  ('ethereum', '0xdac17f958d2ee523a2206206994597c13d831ec7', 'USDT', 'Tether USD',     6,  TRUE),
  ('ethereum', '0x6b175474e89094c44da98b954eedeac495271d0f', 'DAI',  'Dai Stablecoin', 18, TRUE),
  ('base',     '0x833589fcd6edb6e08f4c7c32d4f71b54bda02913', 'USDC', 'USD Coin',       6,  TRUE),
  ('solana',   'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', 'USDC', 'USD Coin',   6, TRUE),
  ('solana',   'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB', 'USDT', 'Tether USD', 6, TRUE)
ON CONFLICT (chain, contract_or_mint) DO UPDATE SET
  symbol = EXCLUDED.symbol,
  name = EXCLUDED.name,
  decimals = EXCLUDED.decimals,
  is_stable = TRUE;
