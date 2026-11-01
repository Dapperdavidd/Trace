"use client";

import Link from "next/link";
import { useState, type FormEvent } from "react";
import { api, describeError } from "@/lib/api/client";
import type { Chain } from "@/lib/api/types";
import { CHAINS } from "@/lib/api/types";
import { CHAIN_LABEL, validateAddress } from "@/lib/address";
import { useApi } from "@/hooks/useApi";
import { WatchRow } from "@/components/Watch";
import { EmptyState, HonestyNote, PageHead, Panel, Remote } from "@/components/ui";

export default function WatchlistPage() {
  const list = useApi(() => api.watchlists(), []);
  const [chain, setChain] = useState<Chain>("solana");
  const [address, setAddress] = useState("");
  const [alias, setAlias] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null);
  const [busy, setBusy] = useState(false);

  const add = async (e: FormEvent) => {
    e.preventDefault();
    setMsg(null);
    const v = validateAddress(chain, address);
    if (!v.ok) {
      setErr(v.reason);
      return;
    }
    setErr(null);
    setBusy(true);
    try {
      await api.addWatchlist({ chain, address: address.trim(), alias: alias.trim() || undefined });
      setAddress("");
      setAlias("");
      setMsg({ ok: true, text: "Added. GiFTED! will check this address for new trades." });
      list.reload();
    } catch (e2) {
      setMsg({ ok: false, text: describeError(e2).message });
    } finally {
      setBusy(false);
    }
  };

  const act = async (fn: () => Promise<unknown>) => {
    setMsg(null);
    try {
      await fn();
      list.reload();
    } catch (e2) {
      setMsg({ ok: false, text: describeError(e2).message });
    }
  };

  return (
    <div className="stack">
      <PageHead title="Watchlist" sub="Watch any public wallet address. No signature, funds or private keys are needed." />
      <HonestyNote>Watching an address only reads public activity. Names are your own labels, not verified identities.</HonestyNote>
      <div className="cols-2">
        <Panel title="Add a wallet">
          <form onSubmit={add} noValidate>
            <div className="form-grid">
              <div className="field">
                <label htmlFor="wl-chain">Chain</label>
                <select id="wl-chain" className="select" value={chain} onChange={(e) => { setChain(e.target.value as Chain); setErr(null); }}>
                  {CHAINS.map((c) => <option key={c} value={c}>{CHAIN_LABEL[c]}</option>)}
                </select>
              </div>
              <div className="field">
                <label htmlFor="wl-alias">Label (optional)</label>
                <input id="wl-alias" className="input" value={alias} maxLength={40} onChange={(e) => setAlias(e.target.value)} />
              </div>
              <div className="field" style={{ gridColumn: "1 / -1" }}>
                <label htmlFor="wl-addr">Address</label>
                <input
                  id="wl-addr"
                  className="input mono"
                  value={address}
                  onChange={(e) => { setAddress(e.target.value); setErr(null); }}
                  onBlur={() => { if (address.trim()) { const v = validateAddress(chain, address); setErr(v.ok ? null : v.reason); } }}
                  aria-invalid={!!err}
                  aria-describedby={err ? "wl-addr-err" : "wl-addr-hint"}
                  autoComplete="off"
                  spellCheck={false}
                  placeholder={chain === "solana" ? "Solana address (32 to 44 characters)" : "0x followed by 40 hex characters"}
                />
                <span id="wl-addr-hint" className="hint">Format is checked here. GiFTED! confirms activity after you add it.</span>
                {err ? <span id="wl-addr-err" className="err" role="alert">{err}</span> : null}
              </div>
            </div>
            <button className="btn btn-primary" type="submit" disabled={busy} style={{ marginTop: 14 }}>{busy ? "Adding" : "Add to watchlist"}</button>
            {msg ? <p role="status" style={{ marginTop: 10 }} className={msg.ok ? "up" : "down"}>{msg.text}</p> : null}
          </form>
        </Panel>
        <Panel title="Watched wallets" action={<Link className="link" href="/alerts">Alert settings</Link>}>
          <Remote
            state={list}
            skeletonRows={3}
            isEmpty={(d) => d.items.length === 0}
            empty={<EmptyState title="No watched wallets yet">Add an address on the left to start.</EmptyState>}
          >
            {(d) => (
              <ul>
                {d.items.map((i) => (
                  <WatchRow
                    key={i.id}
                    item={i}
                    right={
                      <span style={{ display: "flex", gap: 6, flexDirection: "column" }}>
                        <button type="button" className="btn btn-sm" onClick={() => act(() => api.patchWatchlist(i.id, { status: i.status === "paused" ? "active" : "paused" }))}>
                          {i.status === "paused" ? "Resume" : "Pause"}
                        </button>
                        <button type="button" className="btn btn-sm" aria-label={`Remove ${i.wallet.alias ?? i.wallet.address} from watchlist`} onClick={() => act(() => api.deleteWatchlist(i.id))}>
                          Remove
                        </button>
                      </span>
                    }
                  />
                ))}
              </ul>
            )}
          </Remote>
        </Panel>
      </div>
    </div>
  );
}
