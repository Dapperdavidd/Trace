"use client";

import { useState, type FormEvent } from "react";
import { useApi, useAuthToken } from "@/hooks/useApi";
import { api, describeError } from "@/lib/api/client";
import type { AgentConnectionCreated } from "@/lib/api/types";
import { EmptyState, ErrorState, Panel, SkeletonRows } from "./ui";

export function AgentConnections() {
  const appToken = useAuthToken();
  const connections = useApi(() => api.agentConnections(), [appToken], !!appToken);
  const [name, setName] = useState("My research agent");
  const [canPropose, setCanPropose] = useState(true);
  const [created, setCreated] = useState<AgentConnectionCreated | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  if (!appToken) return null;

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setCreated(null);
    try {
      const result = await api.createAgentConnection({
        name,
        scopes: canPropose ? ["observe", "propose"] : ["observe"],
        expires_in_days: 30,
      });
      setCreated(result);
      connections.reload();
    } catch (caught) {
      const detail = describeError(caught);
      setError(`${detail.message}${detail.requestId ? ` Request id: ${detail.requestId}` : ""}`);
    } finally {
      setBusy(false);
    }
  };

  const revoke = async (id: string) => {
    setBusy(true);
    setError(null);
    try {
      await api.revokeAgentConnection(id);
      connections.reload();
    } catch (caught) {
      const detail = describeError(caught);
      setError(`${detail.message}${detail.requestId ? ` Request id: ${detail.requestId}` : ""}`);
    } finally {
      setBusy(false);
    }
  };

  const copyToken = async () => {
    if (!created) return;
    await navigator.clipboard.writeText(created.token);
    setCopied(true);
  };

  return (
    <div className="cols-2">
      <Panel title="Connect your agent" sub="Give an external agent limited access without sharing your TRACE session or wallet key.">
        <form className="stack" onSubmit={submit}>
          <label className="field">
            <span>Connection name</span>
            <input className="input" value={name} onChange={(event) => setName(event.target.value)} maxLength={60} required />
          </label>
          <label className="check-row">
            <input type="checkbox" checked={canPropose} onChange={(event) => setCanPropose(event.target.checked)} />
            <span><b>Allow proposals</b><small>The agent may request a dry NEAR Intents quote only inside an anchored mandate.</small></span>
          </label>
          <p className="honesty-note">Observe lets the agent read watched wallets, alerts and active mandate limits. There is no execute permission.</p>
          <button className="btn btn-primary" type="submit" disabled={busy}>{busy ? "Creating" : "Create agent connection"}</button>
          {error ? <p className="err" role="alert">{error}</p> : null}
          {created ? (
            <div className="state">
              <h4>Copy this token now</h4>
              <p>It is shown once. TRACE stores only a hash.</p>
              <code className="reqid" style={{ overflowWrap: "anywhere" }}>{created.token}</code>
              <button className="btn btn-sm" type="button" onClick={copyToken}>{copied ? "Copied" : "Copy token"}</button>
            </div>
          ) : null}
        </form>
      </Panel>
      <Panel title="Agent connections" sub="Each connection expires and can be revoked without affecting your passkey.">
        {connections.status === "loading" ? <SkeletonRows rows={3} /> : null}
        {connections.status === "error" ? <ErrorState error={connections.error} onRetry={connections.reload} /> : null}
        {connections.data?.items.length === 0 ? <EmptyState title="No connected agents">Create one when you are ready to connect an agent.</EmptyState> : null}
        <ul className="mandate-list">
          {(connections.data?.items ?? []).map((connection) => (
            <li className="mandate-card" key={connection.id}>
              <div className="toolbar">
                <b>{connection.name}</b>
                <span className={`badge ${connection.status === "active" ? "b-live" : "b-unavailable"}`}>{connection.status}</span>
              </div>
              <span className="muted">Scopes: {connection.scopes.join(", ")}</span>
              <span className="reqid">Token ending {connection.token_hint}</span>
              <small className="muted">Expires {new Date(connection.expires_at).toLocaleString()}</small>
              {connection.status === "active" ? <button className="btn btn-sm" type="button" disabled={busy} onClick={() => revoke(connection.id)}>Revoke</button> : null}
            </li>
          ))}
        </ul>
      </Panel>
    </div>
  );
}
