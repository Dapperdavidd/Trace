# Fixtures: how a protocol earns "Enabled"

Two kinds of fixture exist, and only one counts.

| Kind | Location | Proves | Counts toward Enabled |
|---|---|---|---|
| Synthetic | `crates/chain-adapters/tests/*.rs` | decoding and attribution logic | No |
| Real | `crates/chain-adapters/tests/fixtures/real/<chain>/<tx>.json` | the parser agrees with the actual chain | Only when `reviewed` |

All fixtures present today are synthetic. Every protocol is therefore Disabled or Unsupported.

## Capturing (run on your machine, with network access)

The RPC URL goes in the environment, never on the command line, and is never written to a fixture. Only its host name is recorded.

```
export RPC_URL='https://<your provider>/<key>'
cargo run -p trace-fixture-capture -- evm --chain ethereum --protocol uniswap_v2 \
    --tx 0x<hash> --router 0x7a250d5630b4cf539739df2c5dacb4c659f2488d
cargo run -p trace-fixture-capture -- solana --protocol raydium_amm_v4 \
    --program <program id> --tx <signature>
```

Pick varied transactions: a direct pair swap, a routed swap, a multi-hop route, a MEV or aggregator transaction (should be Ambiguous or Infrastructure), a fee-on-transfer token, and a failed or non-swap transaction. At least 3 distinct transactions per protocol are required.

EVM: receipt, transaction, block time, the `finalized` tag, and each swapping pair's `token0()` and `token1()`.
Solana: `getTransaction` with `jsonParsed`, version 0, finalized commitment.

The tool refuses to overwrite an existing fixture and refuses a result whose signature or block does not match the request.

## Reviewing (the step that makes it evidence)

A captured file has `review.status = "unreviewed"` and `review.expected = null`. `review.draft` is what the current parser produced. It is a starting point and not evidence, because the parser cannot grade itself.

1. Open the transaction in a block explorer.
2. Check wallet, both tokens and both amounts by hand.
3. Copy the draft into `expected` only for values you verified. Fix any that are wrong. A wrong draft means a parser bug, so fix the parser as well.
4. Set `review.status` to `"reviewed"`.

Do not edit `payload`. Its sha256 is checked, so a hand edit fails the suite.

## What the suite enforces

`cargo test -p trace-fixture-capture` runs:

* every real fixture is well formed, with an untampered payload and a bare host;
* every reviewed fixture reproduces its `expected` exactly;
* unreviewed fixtures are listed and never counted;
* no protocol is `Partial` or `Enabled` in `registry.rs` without 3 distinct reviewed transactions.

To enable a protocol, add the reviewed fixtures, then change its status in `registry.rs` and `docs/coverage-register.md` in the same commit.
