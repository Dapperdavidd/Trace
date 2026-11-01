"use client";

import { useEffect, useState } from "react";
import { api, describeError } from "@/lib/api/client";
import type { Chain, DiscoveredWallet, WalletTab } from "@/lib/api/types";
import { CHAINS } from "@/lib/api/types";
import { CHAIN_LABEL } from "@/lib/address";
import { useApi } from "@/hooks/useApi";
import { WalletList } from "@/components/WalletList";
import { EmptyState, HonestyNote, PageHead, Panel, Remote, Tabs } from "@/components/ui";

const TABS: ReadonlyArray<{ id: WalletTab; label: string }> = [
  { id: "top", label: "Top performers" },
  { id: "new", label: "New wallets" },
  { id: "most_followed", label: "Most followed" },
];

export default function SmartWalletsPage() {
  const [tab, setTab] = useState<WalletTab>("top");
  const [chain, setChain] = useState<Chain | "">("");
  const [more, setMore] = useState<DiscoveredWallet[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [loadingMore, setLoadingMore] = useState(false);
  const [moreError, setMoreError] = useState<string | null>(null);
  const state = useApi(() => api.discoveryWallets({ tab, chain: chain || undefined }), [tab, chain]);

  useEffect(() => {
    setMore([]);
    setCursor(state.data?.next_cursor ?? null);
    setMoreError(null);
  }, [state.data]);

  const loadMore = async () => {
    if (!cursor) return;
    setLoadingMore(true);
    setMoreError(null);
    try {
      const res = await api.discoveryWallets({ tab, chain: chain || undefined, cursor });
      setMore((m) => [...m, ...res.items]);
      setCursor(res.next_cursor);
    } catch (e) {
      setMoreError(describeError(e).message);
    } finally {
      setLoadingMore(false);
    }
  };

  return (
    <div className="stack">
      <PageHead title="Smart Wallets" sub="Wallets that GiFTED! found by watching real trades, ranked with the evidence behind them." />
      <HonestyNote>
        Return figures are estimates from observed trades. The confidence label says how much to trust them, and wallets with few trades are marked as such. Past results do not guarantee future results. Shadow lets you test a wallet with paper money. GiFTED! cannot copy trades with real money.
      </HonestyNote>
      <Panel>
        <div className="toolbar">
          <Tabs label="Smart wallet category" value={tab} onChange={setTab} tabs={TABS} />
          <div className="field" style={{ flexDirection: "row", alignItems: "center", gap: 8 }}>
            <label htmlFor="w-chain">Chain</label>
            <select id="w-chain" className="select" style={{ width: 150, height: 34 }} value={chain} onChange={(e) => setChain(e.target.value as Chain | "")}>
              <option value="">All chains</option>
              {CHAINS.map((c) => <option key={c} value={c}>{CHAIN_LABEL[c]}</option>)}
            </select>
          </div>
        </div>
        <Remote
          state={state}
          skeletonRows={5}
          skeletonH={72}
          isEmpty={(d) => d.items.length === 0}
          empty={<EmptyState title="No wallets in this view">No wallets passed the evidence checks for this tab and chain.</EmptyState>}
        >
          {(d) => (
            <>
              <WalletList items={[...d.items, ...more]} />
              {cursor ? (
                <div style={{ marginTop: 14 }}>
                  <button type="button" className="btn" onClick={loadMore} disabled={loadingMore}>
                    {loadingMore ? "Loading" : "Load more wallets"}
                  </button>
                  {moreError ? <p className="muted" role="alert" style={{ marginTop: 6 }}>{moreError}</p> : null}
                </div>
              ) : null}
            </>
          )}
        </Remote>
      </Panel>
    </div>
  );
}
