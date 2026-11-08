"use client";

import {
  browserSupportsWebAuthn,
  startAuthentication,
  startRegistration,
  type PublicKeyCredentialCreationOptionsJSON,
  type PublicKeyCredentialRequestOptionsJSON,
} from "@simplewebauthn/browser";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useState, type FormEvent } from "react";
import { useAuthToken } from "@/hooks/useApi";
import { api } from "@/lib/api/client";
import type { FieldConfig } from "@/lib/trace/engine";
import { errText } from "@/lib/trace/near";
import { GoogleButton } from "./GoogleButton";
import { FieldView } from "./Three";
import { Logo } from "./ui";

/* The design's WELCOME() field: the lens wanders, then opens block A to show the trade inside. */
const WELCOME_FIELD = (): FieldConfig => ({
  seed: 11,
  cols: 9,
  rows: 9,
  fov: 42,
  lensR: 1.1,
  fogNear: 30,
  fogFar: 80,
  distMul: 1.4,
  compact: true,
  cardLift: 2.1,
  cardTop: 96,
  wanderDur: 1.6,
  keys: { A: [4, 4] },
  hist: [],
  wash: [],
  scenes: [
    { wander: 1, focus: "center", dist: 1.2 },
    { card: "A", focus: "A", dist: 0.9, lookY: 4.4 },
  ],
  at: () => ((performance.now() / 1000) % 11 < 5.5 ? 0 : 1),
  smooth: 2.6,
});

type Mode = "start" | "create" | "signin";

export function Welcome() {
  const router = useRouter();
  const params = useSearchParams();
  const next = params.get("next") || "/app/today/";
  const token = useAuthToken();
  const [mode, setMode] = useState<Mode>("start");
  const [username, setUsername] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [supported, setSupported] = useState(true);
  useEffect(() => setSupported(browserSupportsWebAuthn()), []);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      if (mode === "create") {
        const started = await api.passkeyRegisterOptions<PublicKeyCredentialCreationOptionsJSON>(username, displayName || username);
        const credential = await startRegistration({ optionsJSON: started.options });
        await api.passkeyRegisterFinish(started.ceremony_id, credential);
      } else {
        const started = await api.passkeyAuthenticateOptions<PublicKeyCredentialRequestOptionsJSON>(username);
        const credential = await startAuthentication({ optionsJSON: started.options });
        await api.passkeyAuthenticateFinish(started.ceremony_id, credential);
      }
      router.push(next);
    } catch (err) {
      const msg = err instanceof Error && err.name === "NotAllowedError" ? "The passkey prompt was closed or timed out. Try again." : errText(err);
      setError(msg);
    } finally {
      setBusy(false);
    }
  };

  return (
    <main className="wel-page">
      <div className="scr s-wel s-wel1">
        <FieldView className="wel3d" config={WELCOME_FIELD} />
        <Link href="/" aria-label="trace home">
          <Logo className="logo wlogo" />
        </Link>
        <Link className="back-home" href="/app/today/">
          Explore first
        </Link>
        <div className="welb">
          {mode === "start" ? (
            <>
              <span className="pill">
                <i />
                On-chain research, made simple
              </span>
              <h3>
                See who&apos;s buying <span>before you buy.</span>
              </h3>
              <p>trace reads every trade on the chain and tells you who is buying, in plain words.</p>
              <div className="bset">
                {token ? (
                  <button className="bfill" type="button" onClick={() => router.push(next)}>
                    Continue
                  </button>
                ) : (
                  <>
                    <button className="bfill" type="button" onClick={() => setMode("create")}>
                      Create an account
                    </button>
                    <button className="bout" type="button" onClick={() => setMode("signin")}>
                      I already have one
                    </button>
                    <GoogleButton onSignedIn={() => router.push(next)} />
                  </>
                )}
              </div>
            </>
          ) : (
            <form onSubmit={submit} style={{ display: "grid", gap: 12 }}>
              <span className="pill">
                <i />
                {mode === "create" ? "No password. Just a passkey." : "Welcome back"}
              </span>
              <h3>
                {mode === "create" ? "Create your account" : "Sign in"}
                <span>{mode === "create" ? "in about 20 seconds." : "with your passkey."}</span>
              </h3>
              {!supported ? (
                <p className="err">This browser does not support passkeys. Try Safari, Chrome or Edge on a recent phone or computer.</p>
              ) : null}
              {mode === "create" ? (
                <label className="fld">
                  <span>Your name</span>
                  <input value={displayName} onChange={(e) => setDisplayName(e.target.value)} autoComplete="name" maxLength={80} placeholder="Ayo Bello" />
                </label>
              ) : null}
              <label className="fld">
                <span>Username</span>
                <input
                  value={username}
                  onChange={(e) => setUsername(e.target.value.toLowerCase().replace(/[^a-z0-9_]/g, ""))}
                  autoComplete="username webauthn"
                  minLength={3}
                  maxLength={32}
                  pattern="[a-z0-9_]+"
                  placeholder="ayo"
                  required
                />
              </label>
              {error ? <p className="err" role="alert">{error}</p> : null}
              <div className="bset">
                <button className="bfill" type="submit" disabled={busy || !supported || username.length < 3}>
                  {busy ? "Waiting for your device" : mode === "create" ? "Create with passkey" : "Sign in with passkey"}
                </button>
                <button className="bout" type="button" onClick={() => setMode(mode === "create" ? "signin" : "create")}>
                  {mode === "create" ? "I already have an account" : "Create an account instead"}
                </button>
              </div>
              <GoogleButton onSignedIn={() => router.push(next)} />
              <p className="sub" style={{ margin: 0, whiteSpace: "normal" }}>
                Your face or fingerprint stays on your device. trace stores only a public key, and never holds your wallet keys.
              </p>
            </form>
          )}
        </div>
      </div>
    </main>
  );
}
