"use client";

import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { readLocal, writeLocal } from "@/lib/local";

export interface Prefs {
  bt: string;
  cur: string;
  lang: string;
  sb: boolean;
  lim: number;
  playing: boolean;
}

interface Persisted {
  star: Record<string, boolean>;
  prefs: Prefs;
  shadow: { w: string; d: number; a: number };
  copy: { mode: "auto" | "alerts"; amt: number; g: boolean[] };
}

const DEFAULTS: Persisted = {
  star: { NEURA: true },
  prefs: { bt: "08:00", cur: "USD", lang: "English", sb: false, lim: 250, playing: false },
  shadow: { w: "0xSun", d: 30, a: 100 },
  copy: { mode: "alerts", amt: 100, g: [true, true, true, true] },
};

const KEY = "trace.app_state.v1";

interface Ctx extends Persisted {
  toggleStar: (k: string) => boolean;
  setPrefs: (p: Partial<Prefs>) => void;
  setShadow: (p: Partial<Persisted["shadow"]>) => void;
  setCopy: (p: Partial<Persisted["copy"]>) => void;
  toast: (m: string) => void;
  toastMsg: string;
  toastOn: boolean;
}

const AppCtx = createContext<Ctx | null>(null);

export function AppStateProvider({ children }: { children: ReactNode }) {
  const [st, setSt] = useState<Persisted>(DEFAULTS);
  const loaded = useRef(false);
  useEffect(() => {
    const saved = readLocal<Partial<Persisted>>(KEY, {});
    setSt((s) => ({ ...s, ...saved, prefs: { ...s.prefs, ...(saved.prefs ?? {}), playing: false } }));
    loaded.current = true;
  }, []);
  useEffect(() => {
    if (loaded.current) writeLocal(KEY, st);
  }, [st]);

  const [toastMsg, setToastMsg] = useState("");
  const [toastOn, setToastOn] = useState(false);
  const tt = useRef<ReturnType<typeof setTimeout> | null>(null);
  const toast = useCallback((m: string) => {
    setToastMsg(m);
    setToastOn(true);
    if (tt.current) clearTimeout(tt.current);
    tt.current = setTimeout(() => setToastOn(false), 2400);
  }, []);

  const toggleStar = useCallback(
    (k: string) => {
      const now = !st.star[k];
      setSt((s) => ({ ...s, star: { ...s.star, [k]: now } }));
      return now;
    },
    [st.star],
  );

  const value = useMemo<Ctx>(
    () => ({
      ...st,
      toggleStar,
      setPrefs: (p) => setSt((s) => ({ ...s, prefs: { ...s.prefs, ...p } })),
      setShadow: (p) => setSt((s) => ({ ...s, shadow: { ...s.shadow, ...p } })),
      setCopy: (p) => setSt((s) => ({ ...s, copy: { ...s.copy, ...p } })),
      toast,
      toastMsg,
      toastOn,
    }),
    [st, toggleStar, toast, toastMsg, toastOn],
  );
  return <AppCtx.Provider value={value}>{children}</AppCtx.Provider>;
}

export function useApp(): Ctx {
  const c = useContext(AppCtx);
  if (!c) throw new Error("useApp outside AppStateProvider");
  return c;
}
