"use client";

import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useState } from "react";
import { api } from "@/lib/api/client";
import type { Chain } from "@/lib/api/types";
import { CHAIN_LABEL, candidateChains } from "@/lib/address";
import { useApi } from "@/hooks/useApi";
import { tokenHref } from "./MoversTable";
import { walletHref } from "./WalletList";
import { Avatar, ComingLaterBadge, EmptyState, PageHead, Panel, Remote } from "./ui";
import { tokenLabel, walletLabel } from "@/lib/format";

export function SearchResults() {
  const sp = useSearchParams();
  const q = (sp?.get("q") ?? "").trim();
  const chains = candidateChains(q);
  const isAddress = chains.length > 0;
  const [chain, setChain] = useState<Chain | null>(null);
  const active = chain && chains.includes(chain) ? chain : chains[0];

  const movers = useApi(() => api.movers({ tab: "trending", window: "24h" }), [], !isAddress && !!q);
  const wallets = useApi(() => api.discoveryWallets({ tab: "top" }), [], !isAddress && !!q);
  const needle = q.toLowerCase();

  if (!q) {
    return (
      <div className="stack">
        <PageHead title="Search" />
        <EmptyState title="Type something to search">Search a coin symbol, a wallet alias, or paste a wallet or token address.</EmptyState>
      </div>
    );
  }

  if (isAddress && active) {
    return (
      <div className="stack">
        <PageHead title="Open this address" sub={<span className="mono" style={{ overflowWrap: "anywhere" }}>{q}</span>} />
        <Panel>
          {chains.length > 1 ? (
            <div className="field" style={{ maxWidth: 260, marginBottom: 14 }}>
              <label htmlFor="s-chain">This address works on several chains. Which one?</label>
              <select id="s-chain" className="select" value={active} onChange={(e) => setChain(e.target.value as Chain)}>
                {chains.map((c) => <option key={c} value={c}>{CHAIN_LABEL[c]}</option>)}
              </select>
            </div>
          ) : (
            <p className="muted" style={{ marginBottom: 14 }}>Format matches {CHAIN_LABEL[active]}.</p>
          )}
          <p className="muted" style={{ marginBottom: 10 }}>GiFTED! cannot tell from the address alone whether this is a wallet or a token, so pick one.</p>
          <div className="hero-cta" style={{ margin: 0 }}>
            <Link className="btn btn-primary" href={`/wallets/${active}/${encodeURIComponent(q)}`}>Open as wallet</Link>
            <Link className="btn" href={`/tokens/${active}/${encodeURIComponent(q)}`}>Open as token</Link>
          </div>
        </Panel>
      </div>
    );
  }

  const looksLikeQuestion = /\s/.test(q) && q.split(/\s+/).length >= 3;
  return (
    <div className="stack">
      <PageHead title={`Results for "${q}"`} sub="Searches the coins and wallets GiFTED! is currently listing." />
      {looksLikeQuestion ? (
        <Panel title="Ask GiFTED!" action={<ComingLaterBadge />}>
          <p className="muted">Answering questions in plain language is coming later. Meanwhile, matching coins and wallets are listed below.</p>
        </Panel>
      ) : null}
      <div className="cols-2">
        <Panel title="Coins">
          <Remote
            state={movers}
            skeletonRows={3}
            isEmpty={(d) => !d.items.some((m) => `${m.token.symbol} ${m.token.name}`.toLowerCase().includes(needle))}
            empty={<EmptyState title="No matching coins">Try a symbol such as NEURA, or paste a token address.</EmptyState>}
          >
            {(d) => (
              <ul>
                {d.items.filter((m) => `${m.token.symbol} ${m.token.name}`.toLowerCase().includes(needle)).map((m) => (
                  <li key={`${m.token.chain}:${m.token.address}`} className="list-row">
                    <Avatar seed={m.token.address} label={tokenLabel(m.token)} />
                    <Link className="grow" href={tokenHref(m.token)}><b>{tokenLabel(m.token)}</b> <span className="muted">{m.token.name}</span></Link>
                  </li>
                ))}
              </ul>
            )}
          </Remote>
        </Panel>
        <Panel title="Wallets">
          <Remote
            state={wallets}
            skeletonRows={3}
            isEmpty={(d) => !d.items.some((w) => (w.wallet.alias ?? "").toLowerCase().includes(needle))}
            empty={<EmptyState title="No matching wallets">Search by alias, or paste a full wallet address.</EmptyState>}
          >
            {(d) => (
              <ul>
                {d.items.filter((w) => (w.wallet.alias ?? "").toLowerCase().includes(needle)).map((w) => (
                  <li key={`${w.wallet.chain}:${w.wallet.address}`} className="list-row">
                    <Avatar seed={w.wallet.address} label={walletLabel(w.wallet)} />
                    <Link className="grow" href={walletHref(w.wallet)}><b>{walletLabel(w.wallet)}</b></Link>
                  </li>
                ))}
              </ul>
            )}
          </Remote>
        </Panel>
      </div>
    </div>
  );
}
