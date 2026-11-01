"use client";

import { useEffect, useState } from "react";
import { api, describeError } from "@/lib/api/client";
import type { AlertItem, AlertPreferences, AlertType } from "@/lib/api/types";
import { useApi } from "@/hooks/useApi";
import { formatRelativeTime } from "@/lib/format";
import { EmptyState, HonestyNote, PageHead, Panel, Remote } from "@/components/ui";

const TYPE_LABEL: Record<AlertType, string> = {
  wallet_buy: "Wallet bought",
  wallet_sell: "Wallet sold",
  risk_event: "Risk event",
  signal: "Research signal",
};

function Prefs() {
  const state = useApi(() => api.alertPreferences(), []);
  const [prefs, setPrefs] = useState<AlertPreferences | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  useEffect(() => { if (state.data) setPrefs(state.data); }, [state.data]);
  const toggle = async (k: keyof AlertPreferences) => {
    if (!prefs) return;
    const next = { ...prefs, [k]: !prefs[k] };
    setPrefs(next);
    setMsg(null);
    try {
      await api.patchAlertPreferences({ [k]: next[k] });
    } catch (e) {
      setPrefs(prefs);
      setMsg(describeError(e).message);
    }
  };
  return (
    <Panel title="Alert settings" sub="Choose what you want to hear about. Alerts appear here in the app. Push delivery is not promised.">
      <Remote state={state} skeletonRows={4} skeletonH={36} hideStatus>
        {() => (
          <fieldset style={{ border: 0, padding: 0, margin: 0 }}>
            <legend className="sr-only">Alert types</legend>
            {(Object.keys(TYPE_LABEL) as AlertType[]).map((k) => (
              <label key={k} className="list-row" style={{ cursor: "pointer" }}>
                <input type="checkbox" checked={!!prefs?.[k]} onChange={() => toggle(k)} style={{ width: 18, height: 18, accentColor: "#4be3a6" }} />
                <span className="grow">{TYPE_LABEL[k]}</span>
              </label>
            ))}
          </fieldset>
        )}
      </Remote>
      {msg ? <p role="alert" className="down" style={{ marginTop: 8 }}>{msg}</p> : null}
    </Panel>
  );
}

export default function AlertsPage() {
  const state = useApi(() => api.alerts(), []);
  const [more, setMore] = useState<AlertItem[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [moreErr, setMoreErr] = useState<string | null>(null);

  useEffect(() => {
    setMore([]);
    setCursor(state.data?.next_cursor ?? null);
  }, [state.data]);

  const loadMore = async () => {
    if (!cursor) return;
    setLoading(true);
    setMoreErr(null);
    try {
      const r = await api.alerts(cursor);
      setMore((m) => [...m, ...r.items]);
      setCursor(r.next_cursor);
    } catch (e) {
      setMoreErr(describeError(e).message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="stack">
      <PageHead title="Alerts" sub="Events from the wallets and coins you follow." />
      <HonestyNote>Alerts report observed events. Each one shows when it happened and when GiFTED! published it, and links to transaction evidence when it has some.</HonestyNote>
      <div className="cols-2">
        <Panel title="Latest alerts">
          <Remote
            state={state}
            skeletonRows={4}
            isEmpty={(d) => d.items.length === 0}
            empty={<EmptyState title="No alerts yet">Watch a wallet and alerts will appear here when it trades.</EmptyState>}
          >
            {(d) => (
              <>
                <ul>
                  {[...d.items, ...more].map((a) => (
                    <li key={a.id} className="evidence">
                      <span className="badge b-neutral">{TYPE_LABEL[a.type]}</span>
                      <div style={{ minWidth: 0 }}>
                        <b>{a.title}</b>
                        <p className="muted" style={{ fontSize: 13 }}>{a.body}</p>
                        <small className="muted">
                          Happened {formatRelativeTime(a.event_time)}, published {formatRelativeTime(a.published_at)}.
                          {a.evidence_tx.length > 0 ? ` Evidence: ${a.evidence_tx.length} transaction${a.evidence_tx.length === 1 ? "" : "s"}.` : ""}
                        </small>
                      </div>
                    </li>
                  ))}
                </ul>
                {cursor ? <button type="button" className="btn" onClick={loadMore} disabled={loading}>{loading ? "Loading" : "Load more"}</button> : null}
                {moreErr ? <p role="alert" className="down">{moreErr}</p> : null}
              </>
            )}
          </Remote>
        </Panel>
        <Prefs />
      </div>
    </div>
  );
}
