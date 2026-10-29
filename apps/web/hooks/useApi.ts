"use client";

import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from "react";
import { isDemoActive, demoReason, subscribeDemo } from "@/lib/api/demo-state";
import { getToken, subscribeToken } from "@/lib/api/token";

export interface ApiState<T> {
  status: "loading" | "ok" | "error";
  data: T | null;
  error: unknown;
  reload: () => void;
}

/**
 * Run an async fetcher whenever `deps` change. `enabled=false` keeps the state in loading.
 * The fetcher identity is intentionally excluded from the dependency list: callers pass deps.
 */
export function useApi<T>(fetcher: () => Promise<T>, deps: readonly unknown[], enabled = true): ApiState<T> {
  const [state, setState] = useState<{ status: ApiState<T>["status"]; data: T | null; error: unknown }>({
    status: "loading",
    data: null,
    error: null,
  });
  const [tick, setTick] = useState(0);
  const fetcherRef = useRef(fetcher);
  fetcherRef.current = fetcher;

  useEffect(() => {
    if (!enabled) {
      setState({ status: "loading", data: null, error: null });
      return;
    }
    let cancelled = false;
    setState((s) => ({ status: "loading", data: s.data, error: null }));
    fetcherRef
      .current()
      .then((data) => {
        if (!cancelled) setState({ status: "ok", data, error: null });
      })
      .catch((error: unknown) => {
        if (!cancelled) setState({ status: "error", data: null, error });
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, enabled, tick]);

  const reload = useCallback(() => setTick((t) => t + 1), []);
  return { ...state, reload };
}

export function useDemoActive(): { active: boolean; reason: string | null } {
  const active = useSyncExternalStore(subscribeDemo, isDemoActive, () => false);
  const reason = useSyncExternalStore(subscribeDemo, demoReason, () => null);
  return { active, reason };
}

export function useAuthToken(): string | null {
  return useSyncExternalStore(subscribeToken, getToken, () => null);
}
