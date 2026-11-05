# TRACE NEAR mandates

This contract makes NEAR the authorization and evidence layer for TRACE agents.
Users create narrowly scoped mandates that specify the agent, chains, assets,
per-trade limit, daily limit, slippage limit and expiry. Owners may pause or
permanently revoke a mandate. Policy-checked execution receipts commit to the
full off-chain evidence with SHA-256.

The contract never stores private keys and cannot initiate an origin-chain
deposit. `approval_required` mandates require the owner to record the result;
`delegated` mandates accept only the named NEAR agent account.

Build the WebAssembly artifact with:

```sh
rustup target add wasm32-unknown-unknown
cargo build -p trace-mandates-contract --target wasm32-unknown-unknown --release
```

