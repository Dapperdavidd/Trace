"use client";

import { useSearchParams } from "next/navigation";
import { useEffect, useState, type FormEvent } from "react";
import { api, describeError } from "@/lib/api/client";
import type { Chain } from "@/lib/api/types";
import { CHAINS } from "@/lib/api/types";
import { CHAIN_LABEL, isValidChain, validateAddress } from "@/lib/address";
import { useApi } from "@/hooks/useApi";
import { readLocal, SHADOW_IDS_KEY, writeLocal } from "@/lib/local";
import { walletLabel } from "@/lib/format";
import { ShadowResultView } from "./ShadowResultView";
import { EmptyState, HonestyNote, PageHead, Panel, Remote } from "./ui";

const SIZE_RULES = [
  { id: "proportional", label: "Match their size, scaled to my budget" },
  { id: "fixed_pct", label: "Fixed 5% of my budget per trade" },
  { id: "fixed_usd", label: "Equal dollar amount per trade" },
];
const EXIT_RULES = [
  { id: "mirror_sells", label: "Sell when they sell" },
  { id: "time_24h", label: "Sell after 24 hours" },
  { id: "stop_loss_20", label: "Sell if down 20%" },
];
const DELAYS = [
  { v: 0, label: "No delay (best case, rarely realistic)" },
  { v: 30, label: "30 seconds" },
  { v: 60, label: "1 minute" },
  { v: 300, label: "5 minutes" },
];
const PRESETS = ["50", "100", "250"];

export function ShadowTrades() {
  const sp = useSearchParams();
  const [chain, setChain] = useState<Chain>("solana");
  const [address, setAddress] = useState("");
  const [budget, setBudget] = useState("100");
  const [sizeRule, setSizeRule] = useState(SIZE_RULES[0]!.id);
  const [delay, setDelay] = useState(30);
  const [slippage, setSlippage] = useState("50");
  const [exitRule, setExitRule] = useState(EXIT_RULES[0]!.id);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [ids, setIds] = useState<string[]>([]);

  const wallets = useApi(() => api.discoveryWallets({ tab: "top" }), []);

  useEffect(() => {
    setIds(readLocal<string[]>(SHADOW_IDS_KEY, []));
    const c = sp?.get("chain");
    const a = sp?.get("address");
    const b = sp?.get("budget");
    if (c && isValidChain(c)) setChain(c);
    if (a) setAddress(a);
    if (b && Number(b) > 0) setBudget(b);
  }, [sp]);

  const results = useApi(() => api.shadowResults(activeId as string), [activeId], !!activeId);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    const errs: Record<string, string> = {};
    const v = validateAddress(chain, address);
    if (!v.ok) errs.address = v.reason;
    const b = Number(budget);
    if (!Number.isFinite(b) || b <= 0) errs.budget = "Enter an amount above 0.";
    else if (b > 1_000_000) errs.budget = "Paper budgets are limited to $1,000,000.";
    const s = Number(slippage);
    if (!Number.isFinite(s) || s < 0 || s > 1000) errs.slippage = "Enter 0 to 1000 basis points (100 = 1%).";
    setErrors(errs);
    setSubmitError(null);
    if (Object.keys(errs).length > 0) return;
    setSubmitting(true);
    try {
      const created = await api.createShadowStrategy({
        wallet: { chain, address: address.trim() },
        budget_usd: String(b),
        size_rule: sizeRule,
        delay_seconds: delay,
        slippage_bps: Math.round(s),
        exit_rule: exitRule,
      });
      const next = [created.id, ...ids.filter((i) => i !== created.id)].slice(0, 10);
      setIds(next);
      writeLocal(SHADOW_IDS_KEY, next);
      setActiveId(created.id);
    } catch (err) {
      setSubmitError(describeError(err).message);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="stack">
      <PageHead title="Shadow Trades" sub="See what would have happened if you followed a wallet, using paper money." />
      <HonestyNote>
        This is a simulation. Nothing is bought, signed or sent. TRACE uses the price a follower could have got after the delay you choose, adds fees and slippage, and skips trades a follower could not have copied. Simulated results are not a prediction or a promise.
      </HonestyNote>
      <div className="cols-2">
        <Panel title="Set up a simulation">
          <form onSubmit={submit} noValidate>
            <div className="form-grid">
              <div className="field" style={{ gridColumn: "1 / -1" }}>
                <label htmlFor="sw-pick">Pick a smart wallet</label>
                <select
                  id="sw-pick"
                  className="select"
                  value=""
                  onChange={(e) => {
                    const [c, ...rest] = e.target.value.split(":");
                    if (c && isValidChain(c)) {
                      setChain(c);
                      setAddress(rest.join(":"));
                    }
                  }}
                >
                  <option value="">{wallets.status === "loading" ? "Loading wallets" : "Choose from discovered wallets"}</option>
                  {(wallets.data?.items ?? []).map((w) => (
                    <option key={`${w.wallet.chain}:${w.wallet.address}`} value={`${w.wallet.chain}:${w.wallet.address}`}>
                      {walletLabel(w.wallet)} ({w.wallet.chain})
                    </option>
                  ))}
                </select>
                <span className="hint">Or enter any public address below.</span>
              </div>
              <div className="field">
                <label htmlFor="sw-chain">Chain</label>
                <select id="sw-chain" className="select" value={chain} onChange={(e) => setChain(e.target.value as Chain)}>
                  {CHAINS.map((c) => <option key={c} value={c}>{CHAIN_LABEL[c]}</option>)}
                </select>
              </div>
              <div className="field">
                <label htmlFor="sw-addr">Wallet address</label>
                <input id="sw-addr" className="input mono" value={address} onChange={(e) => setAddress(e.target.value)} aria-invalid={!!errors.address} aria-describedby={errors.address ? "sw-addr-err" : undefined} autoComplete="off" spellCheck={false} placeholder={chain === "solana" ? "Solana address" : "0x..."} />
                {errors.address ? <span id="sw-addr-err" className="err" role="alert">{errors.address}</span> : null}
              </div>
              <div className="field">
                <label htmlFor="sw-budget">Starting paper budget (USD)</label>
                <input id="sw-budget" className="input" inputMode="decimal" value={budget} onChange={(e) => setBudget(e.target.value)} aria-invalid={!!errors.budget} />
                <div className="tabs sm" role="group" aria-label="Budget presets">
                  {PRESETS.map((p) => (
                    <button key={p} type="button" className="tab" aria-pressed={budget === p} onClick={() => setBudget(p)}>${p}</button>
                  ))}
                </div>
                {errors.budget ? <span className="err" role="alert">{errors.budget}</span> : null}
              </div>
              <div className="field">
                <label htmlFor="sw-size">Position size rule</label>
                <select id="sw-size" className="select" value={sizeRule} onChange={(e) => setSizeRule(e.target.value)}>
                  {SIZE_RULES.map((r) => <option key={r.id} value={r.id}>{r.label}</option>)}
                </select>
              </div>
              <div className="field">
                <label htmlFor="sw-delay">Delay before you could copy</label>
                <select id="sw-delay" className="select" value={delay} onChange={(e) => setDelay(Number(e.target.value))}>
                  {DELAYS.map((d) => <option key={d.v} value={d.v}>{d.label}</option>)}
                </select>
                <span className="hint">Longer delays usually mean worse prices.</span>
              </div>
              <div className="field">
                <label htmlFor="sw-slip">Slippage (basis points)</label>
                <input id="sw-slip" className="input" inputMode="numeric" value={slippage} onChange={(e) => setSlippage(e.target.value)} aria-invalid={!!errors.slippage} />
                <span className="hint">100 basis points = 1%.</span>
                {errors.slippage ? <span className="err" role="alert">{errors.slippage}</span> : null}
              </div>
              <div className="field">
                <label htmlFor="sw-exit">Exit rule</label>
                <select id="sw-exit" className="select" value={exitRule} onChange={(e) => setExitRule(e.target.value)}>
                  {EXIT_RULES.map((r) => <option key={r.id} value={r.id}>{r.label}</option>)}
                </select>
              </div>
            </div>
            {submitError ? <p className="err" role="alert" style={{ color: "#ffb0b0", marginTop: 12 }}>{submitError}</p> : null}
            <button type="submit" className="btn btn-primary" disabled={submitting} style={{ marginTop: 16 }}>
              {submitting ? "Starting" : "Start Shadowing"}
            </button>
          </form>
        </Panel>

        <div className="stack">
          <Panel title="Results" sub="Simulated with paper money.">
            {!activeId ? (
              <EmptyState title="No simulation selected">Start a simulation, or open a saved one below.</EmptyState>
            ) : (
              <Remote state={results} skeletonRows={4} skeletonH={36}>{(r) => <ShadowResultView r={r} />}</Remote>
            )}
          </Panel>
          <Panel title="Saved on this device" sub="Simulation ids are remembered in this browser only.">
            {ids.length === 0 ? (
              <p className="muted">No saved simulations yet.</p>
            ) : (
              <ul>
                {ids.map((id) => (
                  <li key={id} className="list-row">
                    <span className="mono grow ellip">{id}</span>
                    <button type="button" className="btn btn-sm" onClick={() => setActiveId(id)} aria-pressed={activeId === id}>View</button>
                  </li>
                ))}
              </ul>
            )}
          </Panel>
        </div>
      </div>
    </div>
  );
}
