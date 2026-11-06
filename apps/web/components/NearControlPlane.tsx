"use client";

import { NearConnector, type NearWalletBase } from "@hot-labs/near-connect";
import Link from "next/link";
import { useEffect, useRef, useState, type FormEvent } from "react";
import { useApi, useAuthToken } from "@/hooks/useApi";
import { api, describeError } from "@/lib/api/client";
import type {
  CreateNearMandateBody,
  NearAsset,
  NearMandateCreated,
  NearStatus,
} from "@/lib/api/types";
import { readLocal, SHADOW_IDS_KEY } from "@/lib/local";
import { ErrorState, EmptyState, PageHead, Panel, SkeletonRows, StatusBadge } from "./ui";

type WalletState = { wallet: NearWalletBase; accountId: string } | null;

function outcomeHash(value: unknown): string | null {
  if (!value || typeof value !== "object") return null;
  const transaction = (value as { transaction?: unknown }).transaction;
  if (!transaction || typeof transaction !== "object") return null;
  const hash = (transaction as { hash?: unknown }).hash;
  return typeof hash === "string" ? hash : null;
}

function statusLabel(status: NearStatus | null): string {
  if (!status?.contract.configured) return "Contract configuration required";
  if (!status.agent.configured) return "Agent account configuration required";
  return "Ready for owner-approved testnet mandates";
}

export function NearControlPlane() {
  const appToken = useAuthToken();
  const status = useApi(() => api.nearStatus(), []);
  const assets = useApi(() => api.nearAssets(), [appToken], !!appToken);
  const mandates = useApi(() => api.nearMandates(), [appToken], !!appToken);
  const connector = useRef<NearConnector | null>(null);
  const [walletState, setWalletState] = useState<WalletState>(null);
  const [walletBusy, setWalletBusy] = useState(false);
  const [walletError, setWalletError] = useState<string | null>(null);
  const [shadowIds, setShadowIds] = useState<string[]>([]);
  const [strategyId, setStrategyId] = useState("");
  const [originAsset, setOriginAsset] = useState("");
  const [destinationAsset, setDestinationAsset] = useState("");
  const [mode, setMode] = useState<CreateNearMandateBody["mode"]>("approval_required");
  const [maxTrade, setMaxTrade] = useState("100");
  const [maxDaily, setMaxDaily] = useState("250");
  const [slippage, setSlippage] = useState("100");
  const [created, setCreated] = useState<NearMandateCreated | null>(null);
  const [submitBusy, setSubmitBusy] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [anchorMessage, setAnchorMessage] = useState<string | null>(null);

  const network = status.data?.network ?? "testnet";

  useEffect(() => {
    const ids = readLocal<string[]>(SHADOW_IDS_KEY, []);
    setShadowIds(ids);
    setStrategyId((current) => current || ids[0] || "");
  }, []);

  useEffect(() => {
    const items = assets.data?.items ?? [];
    if (!originAsset && items[0]) setOriginAsset(items[0].assetId);
    if (!destinationAsset && items[1]) setDestinationAsset(items[1].assetId);
  }, [assets.data, destinationAsset, originAsset]);

  useEffect(() => {
    if (!status.data) return;
    const near = new NearConnector({
      network,
      features:
        network === "testnet"
          ? { signAndSendTransaction: true, testnet: true }
          : { signAndSendTransaction: true, mainnet: true },
      footerBranding: {
        heading: "TRACE",
        link: window.location.origin,
        linkText: "NEAR-controlled agent mandates",
      },
    });
    connector.current = near;
    near
      .getConnectedWallet()
      .then(({ wallet, accounts }) => {
        const accountId = accounts[0]?.accountId;
        if (accountId) setWalletState({ wallet, accountId });
      })
      .catch(() => undefined);
    return () => {
      near.removeAllListeners();
      if (connector.current === near) connector.current = null;
    };
  }, [network, status.data]);

  const connect = async () => {
    if (!connector.current) return;
    setWalletBusy(true);
    setWalletError(null);
    try {
      const wallet = await connector.current.connect();
      const accounts = await wallet.getAccounts({ network });
      const accountId = accounts[0]?.accountId;
      if (!accountId) throw new Error("The wallet returned no NEAR account.");
      setWalletState({ wallet, accountId });
    } catch (error) {
      setWalletError(error instanceof Error ? error.message : "NEAR wallet connection failed.");
    } finally {
      setWalletBusy(false);
    }
  };

  const disconnect = async () => {
    if (!walletState) return;
    setWalletBusy(true);
    try {
      await walletState.wallet.signOut({ network });
      setWalletState(null);
      setCreated(null);
    } finally {
      setWalletBusy(false);
    }
  };

  const asset = (id: string): NearAsset | undefined =>
    assets.data?.items.find((item) => item.assetId === id);

  const createMandate = async (event: FormEvent) => {
    event.preventDefault();
    setSubmitError(null);
    setAnchorMessage(null);
    if (!walletState) {
      setSubmitError("Connect the NEAR wallet that will own this mandate.");
      return;
    }
    if (!strategyId) {
      setSubmitError("Create and run a Shadow strategy first.");
      return;
    }
    const origin = asset(originAsset);
    const destination = asset(destinationAsset);
    if (!origin || !destination || origin.assetId === destination.assetId) {
      setSubmitError("Choose two different assets reported by NEAR Intents.");
      return;
    }
    const slippageBps = Number(slippage);
    if (!Number.isInteger(slippageBps) || slippageBps < 0 || slippageBps > 2_000) {
      setSubmitError("Slippage must be between 0 and 2,000 basis points.");
      return;
    }
    setSubmitBusy(true);
    try {
      const next = await api.createNearMandate({
        shadow_strategy_id: strategyId,
        owner_near_account: walletState.accountId,
        agent_near_account: status.data?.agent.account_id ?? undefined,
        mode,
        allowed_chains: Array.from(new Set([origin.blockchain, destination.blockchain])),
        allowed_assets: [
          { chain: origin.blockchain, asset_id: origin.assetId },
          { chain: destination.blockchain, asset_id: destination.assetId },
        ],
        max_trade_usd: maxTrade,
        max_daily_usd: maxDaily,
        max_slippage_bps: slippageBps,
        expires_at: new Date(Date.now() + 7 * 24 * 60 * 60 * 1_000).toISOString(),
      });
      setCreated(next);
      mandates.reload();
    } catch (error) {
      const described = describeError(error);
      setSubmitError(
        `${described.message}${described.requestId ? ` Request id: ${described.requestId}` : ""}`,
      );
    } finally {
      setSubmitBusy(false);
    }
  };

  const signMandate = async () => {
    if (!created?.near_transaction || !walletState) return;
    setSubmitBusy(true);
    setSubmitError(null);
    setAnchorMessage(null);
    try {
      const transaction = created.near_transaction;
      const outcome = await walletState.wallet.signAndSendTransaction({
        network,
        signerId: walletState.accountId,
        receiverId: transaction.receiverId,
        actions: transaction.actions,
      });
      const hash = outcomeHash(outcome);
      if (!hash) throw new Error("The wallet did not return a transaction hash.");
      await api.confirmNearAnchor(created.id, {
        tx_hash: hash,
        signer_account_id: walletState.accountId,
      });
      setAnchorMessage(`Mandate anchored and verified on NEAR. Transaction ${hash}`);
      setCreated(null);
      mandates.reload();
    } catch (error) {
      const described = describeError(error);
      setSubmitError(
        `${described.message}${described.requestId ? ` Request id: ${described.requestId}` : ""}`,
      );
    } finally {
      setSubmitBusy(false);
    }
  };

  return (
    <div className="stack near-page">
      <PageHead
        title="NEAR Agent Control Plane"
        sub="NEAR owns agent permissions, cross-chain intent policy and verifiable receipts. Ethereum and Solana are markets TRACE observes."
        action={
          status.data ? <StatusBadge status={status.data.data_status} /> : null
        }
      />

      <section className="near-core" aria-label="NEAR architecture">
        <div><span>1</span><b>Discover</b><small>TRACE finds wallets and explains evidence across chains.</small></div>
        <div><span>2</span><b>Prove</b><small>Shadow tests the strategy with paper money first.</small></div>
        <div><span>3</span><b>Authorize on NEAR</b><small>Your mandate sets assets, limits, slippage and expiry.</small></div>
        <div><span>4</span><b>Execute by intent</b><small>NEAR Intents routes an owner-approved action across chains.</small></div>
      </section>

      {status.status === "error" ? <ErrorState error={status.error} onRetry={status.reload} /> : null}
      <div className="cols-3">
        <Panel title="NEAR contract" sub="Authorization and receipt commitments">
          {status.data ? (
            <div className="stack-sm">
              <div className="kv"><span>Network</span><b>{status.data.network}</b></div>
              <div className="kv"><span>Contract</span><b className="mono ellip">{status.data.contract.account_id ?? "Unavailable"}</b></div>
              <div className="kv"><span>Deployment verified</span><b>{status.data.contract.deployment_verified ? "Yes" : "No"}</b></div>
              <p className="muted">{statusLabel(status.data)}</p>
            </div>
          ) : <SkeletonRows rows={3} h={30} />}
        </Panel>
        <Panel title="Connected owner" sub="The wallet keeps its own keys">
          {walletState ? (
            <div className="stack-sm">
              <strong className="mono ellip">{walletState.accountId}</strong>
              <span className="badge b-live">Wallet connected</span>
              <button type="button" className="btn btn-sm" onClick={disconnect} disabled={walletBusy}>Disconnect</button>
            </div>
          ) : (
            <div className="stack-sm">
              <p className="muted">Connect the NEAR account that will own, pause and revoke the mandate.</p>
              <button type="button" className="btn btn-primary" onClick={connect} disabled={walletBusy || !status.data}>
                {walletBusy ? "Connecting" : "Connect NEAR wallet"}
              </button>
            </div>
          )}
          {walletError ? <p className="err" role="alert">{walletError}</p> : null}
        </Panel>
        <Panel title="TRACE agent" sub="One named agent, never unrestricted authority">
          {status.data ? (
            <div className="stack-sm">
              <strong className="mono ellip">{status.data.agent.account_id ?? "Unavailable"}</strong>
              <span className={`badge ${status.data.agent.delegation_enabled ? "b-partial" : "b-unavailable"}`}>
                Delegation {status.data.agent.delegation_enabled ? "enabled" : "off"}
              </span>
              <p className="muted">Owner approval is the default. Delegation remains off until deployment and security review.</p>
            </div>
          ) : <SkeletonRows rows={3} h={30} />}
        </Panel>
      </div>

      {!appToken ? (
        <EmptyState
          title="Sign in to create a mandate"
          action={<Link className="btn btn-primary" href="/sign-in">Sign in</Link>}
        >Your TRACE account stores drafts and evidence. Your NEAR wallet remains the authority.</EmptyState>
      ) : (
        <div className="cols-2">
          <Panel title="Create an agent mandate" sub="A draft has no authority until your NEAR wallet signs it.">
            <form onSubmit={createMandate} className="stack" noValidate>
              <div className="form-grid">
                <div className="field" style={{ gridColumn: "1 / -1" }}>
                  <label htmlFor="near-strategy">Proven Shadow strategy</label>
                  {shadowIds.length ? (
                    <select id="near-strategy" className="select" value={strategyId} onChange={(event) => setStrategyId(event.target.value)}>
                      {shadowIds.map((id) => <option key={id} value={id}>{id}</option>)}
                    </select>
                  ) : (
                    <div className="state"><p>Create a Shadow strategy first.</p><Link className="btn btn-sm" href="/shadow-trades">Open Shadow</Link></div>
                  )}
                </div>
                <div className="field">
                  <label htmlFor="near-origin">Allowed input asset</label>
                  <select id="near-origin" className="select" value={originAsset} onChange={(event) => setOriginAsset(event.target.value)} disabled={assets.status !== "ok"}>
                    {(assets.data?.items ?? []).map((item) => <option key={item.assetId} value={item.assetId}>{item.symbol} on {item.blockchain}</option>)}
                  </select>
                </div>
                <div className="field">
                  <label htmlFor="near-destination">Allowed output asset</label>
                  <select id="near-destination" className="select" value={destinationAsset} onChange={(event) => setDestinationAsset(event.target.value)} disabled={assets.status !== "ok"}>
                    {(assets.data?.items ?? []).map((item) => <option key={item.assetId} value={item.assetId}>{item.symbol} on {item.blockchain}</option>)}
                  </select>
                </div>
                <div className="field">
                  <label htmlFor="near-trade-limit">Maximum per trade (USD)</label>
                  <input id="near-trade-limit" className="input" inputMode="decimal" value={maxTrade} onChange={(event) => setMaxTrade(event.target.value)} />
                </div>
                <div className="field">
                  <label htmlFor="near-daily-limit">Maximum per day (USD)</label>
                  <input id="near-daily-limit" className="input" inputMode="decimal" value={maxDaily} onChange={(event) => setMaxDaily(event.target.value)} />
                </div>
                <div className="field">
                  <label htmlFor="near-slippage">Maximum slippage (basis points)</label>
                  <input id="near-slippage" className="input" inputMode="numeric" value={slippage} onChange={(event) => setSlippage(event.target.value)} />
                </div>
                <div className="field">
                  <label htmlFor="near-mode">Authority</label>
                  <select id="near-mode" className="select" value={mode} onChange={(event) => setMode(event.target.value as CreateNearMandateBody["mode"])}>
                    <option value="approval_required">Owner approves every action</option>
                    <option value="paper">Paper evidence only</option>
                    <option value="delegated" disabled={!status.data?.agent.delegation_enabled}>Restricted delegation</option>
                  </select>
                </div>
              </div>
              <p className="honesty-note">Expires in 7 days. Both assets, per-trade value, daily value and slippage are enforced by the NEAR mandate. Revocation is permanent.</p>
              {assets.status === "loading" ? <p className="muted">Loading the live NEAR Intents asset list.</p> : null}
              {assets.status === "error" ? <ErrorState error={assets.error} onRetry={assets.reload} /> : null}
              {submitError ? <p className="err" role="alert">{submitError}</p> : null}
              {anchorMessage ? <p className="honesty-note" role="status">{anchorMessage}</p> : null}
              <button type="submit" className="btn btn-primary" disabled={submitBusy || !walletState || !shadowIds.length || !status.data?.agent.configured}>
                {submitBusy ? "Preparing" : "Prepare NEAR mandate"}
              </button>
              {created ? (
                <div className="state">
                  <h4>Draft prepared</h4>
                  <p>{created.note}</p>
                  <span className="reqid">Commitment: {created.commitment}</span>
                  <button type="button" className="btn btn-primary" onClick={signMandate} disabled={submitBusy || !created.near_transaction}>
                    Review and sign in NEAR wallet
                  </button>
                </div>
              ) : null}
            </form>
          </Panel>

          <Panel title="Your mandates" sub="Only verified anchors are active authority.">
            {mandates.status === "loading" ? <SkeletonRows rows={4} /> : null}
            {mandates.status === "error" ? <ErrorState error={mandates.error} onRetry={mandates.reload} /> : null}
            {mandates.data?.items.length === 0 ? <EmptyState title="No mandates yet">Prepare one after testing a Shadow strategy.</EmptyState> : null}
            <ul className="mandate-list">
              {(mandates.data?.items ?? []).map((mandate) => (
                <li key={mandate.id} className="mandate-card">
                  <div className="toolbar">
                    <span className={`badge ${mandate.status === "anchored" ? "b-live" : "b-partial"}`}>{mandate.status}</span>
                    <span className="badge b-neutral">{mandate.mode.replaceAll("_", " ")}</span>
                  </div>
                  <b className="mono ellip">{mandate.agent_near_account}</b>
                  <div className="kv"><span>Per trade</span><b>${mandate.max_trade_usd}</b></div>
                  <div className="kv"><span>Per day</span><b>${mandate.max_daily_usd}</b></div>
                  <div className="kv"><span>Slippage</span><b>{mandate.max_slippage_bps} bps</b></div>
                  <small className="muted">Expires {new Date(mandate.expires_at).toLocaleString()}</small>
                  {mandate.anchor_tx_hash ? <span className="reqid">NEAR tx: {mandate.anchor_tx_hash}</span> : <span className="reqid">Not anchored</span>}
                </li>
              ))}
            </ul>
          </Panel>
        </div>
      )}
    </div>
  );
}

