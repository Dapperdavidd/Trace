"use client";

import { useRouter } from "next/navigation";
import { useEffect, useRef, useState } from "react";
import { api, describeError } from "@/lib/api/client";

interface GsiId {
  initialize(cfg: { client_id: string; callback: (r: { credential: string }) => void }): void;
  renderButton(el: HTMLElement, opts: Record<string, string | number>): void;
}
declare global {
  interface Window {
    google?: { accounts: { id: GsiId } };
  }
}

const CLIENT_ID = process.env.NEXT_PUBLIC_GOOGLE_CLIENT_ID ?? "";
const DEV_AUTH = process.env.NEXT_PUBLIC_TRACE_DEV_AUTH === "1";
const SRC = "https://accounts.google.com/gsi/client";

export function GoogleSignIn() {
  const holder = useRef<HTMLDivElement>(null);
  const router = useRouter();
  const [state, setState] = useState<"loading" | "ready" | "failed">("loading");
  const [error, setError] = useState<string | null>(null);
  const [devBusy, setDevBusy] = useState(false);

  const devSignIn = async () => {
    setDevBusy(true);
    setError(null);
    try {
      await api.authGoogle("dev:web-local@gifted.test");
      router.push("/");
    } catch (e) {
      setError(describeError(e).message);
    } finally {
      setDevBusy(false);
    }
  };

  useEffect(() => {
    if (!CLIENT_ID) return;
    let cancelled = false;
    const init = () => {
      if (cancelled || !window.google || !holder.current) return;
      window.google.accounts.id.initialize({
        client_id: CLIENT_ID,
        callback: async (resp) => {
          setError(null);
          try {
            await api.authGoogle(resp.credential);
            router.push("/");
          } catch (e) {
            setError(describeError(e).message);
          }
        },
      });
      window.google.accounts.id.renderButton(holder.current, { theme: "filled_black", size: "large", text: "signin_with", shape: "pill", width: 280 });
      setState("ready");
    };
    if (window.google) {
      init();
    } else {
      let script = document.querySelector<HTMLScriptElement>(`script[src="${SRC}"]`);
      if (!script) {
        script = document.createElement("script");
        script.src = SRC;
        script.async = true;
        script.defer = true;
        document.head.appendChild(script);
      }
      script.addEventListener("load", init);
      script.addEventListener("error", () => !cancelled && setState("failed"));
    }
    return () => {
      cancelled = true;
    };
  }, [router]);

  if (!CLIENT_ID && !DEV_AUTH) {
    return (
      <div className="state" role="status">
        <h4>Google sign-in not configured</h4>
        <p>
          This build has no Google client id, so sign-in is switched off. Set <span className="mono">NEXT_PUBLIC_GOOGLE_CLIENT_ID</span> and rebuild to enable it. You can still browse research without an account.
        </p>
        <button type="button" className="btn" disabled aria-disabled="true" style={{ marginTop: 12 }}>Sign in with Google (not configured)</button>
      </div>
    );
  }
  return (
    <div>
      {CLIENT_ID ? <div ref={holder} aria-label="Sign in with Google" /> : null}
      {CLIENT_ID && state === "loading" ? <p className="muted">Loading Google sign-in</p> : null}
      {CLIENT_ID && state === "failed" ? (
        <p role="alert" className="down">Google sign-in could not load. Check your connection or content blocker and try again.</p>
      ) : null}
      {DEV_AUTH ? (
        <div className="state" style={{ marginTop: CLIENT_ID ? 14 : 0 }}>
          <h4>Local development sign-in</h4>
          <p>This option appears only when this web build and the local API both have development authentication enabled.</p>
          <button type="button" className="btn btn-sm" onClick={devSignIn} disabled={devBusy}>
            {devBusy ? "Signing in" : "Sign in locally"}
          </button>
        </div>
      ) : null}
      {error ? <p role="alert" className="down" style={{ marginTop: 8 }}>{error}</p> : null}
    </div>
  );
}
