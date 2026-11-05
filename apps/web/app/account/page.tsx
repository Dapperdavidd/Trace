"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { useApi, useAuthToken } from "@/hooks/useApi";
import { api } from "@/lib/api/client";
import type { AccountSession } from "@/lib/api/types";
import { EmptyState, ErrorState, PageHead, Panel, SkeletonRows } from "@/components/ui";

function dateLabel(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "Unavailable";
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

function SessionRow({ session, onRevoke, busy }: {
  session: AccountSession;
  onRevoke: (id: string) => void;
  busy: boolean;
}) {
  return (
    <div className="session-row">
      <div className="session-details">
        <strong>{session.device_label} {session.current ? <span className="badge b-live">This device</span> : null}</strong>
        <small>Signed in {dateLabel(session.created_at)}</small>
        <small>Expires {dateLabel(session.expires_at)}</small>
      </div>
      {session.current ? (
        <span className="muted">Use Sign out below</span>
      ) : (
        <button className="btn btn-sm btn-danger" type="button" disabled={busy} onClick={() => onRevoke(session.id)}>
          {busy ? "Revoking..." : "Revoke"}
        </button>
      )}
    </div>
  );
}

export default function AccountPage() {
  const token = useAuthToken();
  const router = useRouter();
  const sessions = useApi(() => api.sessions(), [token], !!token);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [actionError, setActionError] = useState<unknown>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleteText, setDeleteText] = useState("");

  const revoke = async (id: string) => {
    setBusyId(id);
    setActionError(null);
    try {
      await api.revokeSession(id);
      sessions.reload();
    } catch (error) {
      setActionError(error);
    } finally {
      setBusyId(null);
    }
  };

  const signOutAll = async () => {
    setBusyId("all");
    setActionError(null);
    try {
      await api.logoutAll();
      router.push("/");
    } catch (error) {
      setActionError(error);
    } finally {
      setBusyId(null);
    }
  };

  const deleteAccount = async () => {
    if (deleteText !== "DELETE") return;
    setBusyId("delete");
    setActionError(null);
    try {
      await api.deleteAccount();
      router.push("/");
    } catch (error) {
      setActionError(error);
    } finally {
      setBusyId(null);
    }
  };

  if (!token) {
    return (
      <div className="stack" style={{ maxWidth: 720 }}>
        <PageHead title="Account" sub="Review and revoke signed-in devices." />
        <Panel>
          <EmptyState title="Sign in required" action={<Link className="btn btn-primary btn-sm" href="/sign-in">Sign in</Link>}>
            Sign in to review active account sessions.
          </EmptyState>
        </Panel>
      </div>
    );
  }

  return (
    <div className="stack" style={{ maxWidth: 720 }}>
      <PageHead title="Account sessions" sub="Review every device signed in to your TRACE account." />
      <Panel title="Signed-in devices" sub="Revoke anything you do not recognise.">
        {sessions.status === "loading" && sessions.data === null ? <SkeletonRows rows={2} h={76} /> : null}
        {sessions.status === "error" ? <ErrorState error={sessions.error} onRetry={sessions.reload} /> : null}
        {sessions.data?.items.length === 0 ? <EmptyState title="No active sessions">Sign in again to create a new session.</EmptyState> : null}
        {sessions.data?.items.length ? (
          <div className="session-list">
            {sessions.data.items.map((session) => (
              <SessionRow key={session.id} session={session} onRevoke={revoke} busy={busyId === session.id} />
            ))}
          </div>
        ) : null}
      </Panel>
      {actionError ? <ErrorState error={actionError} /> : null}
      <Panel title="Sign out everywhere" sub="This immediately revokes every active session, including this device.">
        <button className="btn btn-danger" type="button" disabled={busyId !== null} onClick={signOutAll}>
          {busyId === "all" ? "Signing out..." : "Sign out on all devices"}
        </button>
      </Panel>
      <Panel title="Delete account" sub="Permanently remove your TRACE account and its account-owned data.">
        {!confirmDelete ? (
          <div className="danger-action">
            <p className="muted">This deletes your sessions, watchlist, alert preferences, alerts, and Shadow strategies. Public blockchain data is not part of your account and is not deleted.</p>
            <button className="btn btn-danger" type="button" disabled={busyId !== null} onClick={() => setConfirmDelete(true)}>
              Delete account
            </button>
          </div>
        ) : (
          <div className="delete-confirm" role="group" aria-labelledby="delete-confirm-title">
            <strong id="delete-confirm-title">This cannot be undone</strong>
            <p className="muted">Type DELETE to confirm permanent account deletion.</p>
            <div className="field">
              <label htmlFor="delete-confirmation">Confirmation</label>
              <input
                id="delete-confirmation"
                className="input"
                value={deleteText}
                onChange={(event) => setDeleteText(event.target.value)}
                autoComplete="off"
                spellCheck={false}
              />
            </div>
            <div className="confirm-actions">
              <button
                className="btn"
                type="button"
                disabled={busyId !== null}
                onClick={() => {
                  setConfirmDelete(false);
                  setDeleteText("");
                }}
              >
                Cancel
              </button>
              <button className="btn btn-danger" type="button" disabled={deleteText !== "DELETE" || busyId !== null} onClick={deleteAccount}>
                {busyId === "delete" ? "Deleting..." : "Delete permanently"}
              </button>
            </div>
          </div>
        )}
      </Panel>
    </div>
  );
}
