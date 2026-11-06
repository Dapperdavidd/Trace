"use client";

import {
  browserSupportsWebAuthn,
  startAuthentication,
  startRegistration,
  type PublicKeyCredentialCreationOptionsJSON,
  type PublicKeyCredentialRequestOptionsJSON,
} from "@simplewebauthn/browser";
import { useRouter } from "next/navigation";
import { useState, type FormEvent } from "react";
import { api, describeError } from "@/lib/api/client";

type Mode = "sign-in" | "create";

export function PasskeySignIn() {
  const router = useRouter();
  const [mode, setMode] = useState<Mode>("sign-in");
  const [username, setUsername] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const supported = typeof window === "undefined" || browserSupportsWebAuthn();

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setError(null);
    setBusy(true);
    try {
      if (mode === "create") {
        const started = await api.passkeyRegisterOptions<PublicKeyCredentialCreationOptionsJSON>(
          username,
          displayName,
        );
        const credential = await startRegistration({ optionsJSON: started.options });
        await api.passkeyRegisterFinish(started.ceremony_id, credential);
      } else {
        const started = await api.passkeyAuthenticateOptions<PublicKeyCredentialRequestOptionsJSON>(username);
        const credential = await startAuthentication({ optionsJSON: started.options });
        await api.passkeyAuthenticateFinish(started.ceremony_id, credential);
      }
      router.push("/");
    } catch (caught) {
      const described = describeError(caught);
      setError(`${described.message}${described.requestId ? ` Request id: ${described.requestId}` : ""}`);
    } finally {
      setBusy(false);
    }
  };

  if (!supported) {
    return (
      <div className="state" role="status">
        <h4>Passkeys unavailable</h4>
        <p>This browser does not support passkeys. Use the Google fallback or update your browser.</p>
      </div>
    );
  }

  return (
    <div className="stack" style={{ gap: 14 }}>
      <div className="tabs" aria-label="Passkey account action">
        <button type="button" className={mode === "sign-in" ? "active" : ""} onClick={() => setMode("sign-in")}>
          Sign in
        </button>
        <button type="button" className={mode === "create" ? "active" : ""} onClick={() => setMode("create")}>
          Create account
        </button>
      </div>
      <form onSubmit={submit} className="stack" style={{ gap: 12 }}>
        {mode === "create" ? (
          <label className="field">
            <span>Display name</span>
            <input value={displayName} onChange={(event) => setDisplayName(event.target.value)} autoComplete="name" maxLength={80} required />
          </label>
        ) : null}
        <label className="field">
          <span>Username</span>
          <input
            value={username}
            onChange={(event) => setUsername(event.target.value.toLowerCase())}
            autoComplete="username webauthn"
            minLength={3}
            maxLength={32}
            pattern="[a-z0-9_]+"
            required
          />
        </label>
        <button type="submit" className="btn" disabled={busy}>
          {busy ? "Waiting for your device" : mode === "create" ? "Create account with passkey" : "Sign in with passkey"}
        </button>
      </form>
      <p className="muted" style={{ fontSize: 12.5 }}>
        Your fingerprint or face stays on your device. TRACE stores only a public key.
      </p>
      {error ? <p role="alert" className="down">{error}</p> : null}
    </div>
  );
}
