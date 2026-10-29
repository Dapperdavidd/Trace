import { getToken, setToken, clearToken } from "./token";
import { markDemoActive } from "./demo-state";
import { demoResponse } from "./demo";
import {
  guardAlerts,
  guardAny,
  guardAuth,
  guardBriefing,
  guardCompare,
  guardDiscoveryWallets,
  guardDna,
  guardIdCreated,
  guardMe,
  guardMovers,
  guardShadowResults,
  guardTimeline,
  guardToken,
  guardWalletDetail,
  guardWalletMetrics,
  guardWalletTrades,
  guardWatchlist,
  hasEnvelope,
  isRecord,
  type Guard,
} from "./guards";
import type {
  AddWatchlistBody,
  AlertPreferences,
  AlertsResponse,
  ApiErrorBody,
  AuthSession,
  BriefingResponse,
  Chain,
  ChainsResponse,
  CompareResponse,
  DiscoveryWalletsResponse,
  Me,
  MoverTab,
  MoverWindow,
  MoversResponse,
  MyWallets,
  PatchWatchlistBody,
  ShadowResults,
  ShadowStrategyBody,
  ShadowStrategyCreated,
  TimelineResponse,
  TokenDetail,
  WalletDetail,
  WalletDna,
  WalletMetricsResponse,
  WalletTab,
  WalletTradesResponse,
  WatchlistItem,
  WatchlistResponse,
} from "./types";

export type ApiErrorKind = "api" | "network" | "timeout" | "invalid_response" | "unauthorized";

export class ApiError extends Error {
  readonly kind: ApiErrorKind;
  readonly code: string;
  readonly status: number | null;
  readonly requestId: string | null;
  readonly details: unknown;

  constructor(init: {
    kind: ApiErrorKind;
    code: string;
    message: string;
    status?: number | null;
    requestId?: string | null;
    details?: unknown;
  }) {
    super(init.message);
    this.name = "ApiError";
    this.kind = init.kind;
    this.code = init.code;
    this.status = init.status ?? null;
    this.requestId = init.requestId ?? null;
    this.details = init.details;
  }
}

export interface ClientOptions {
  baseUrl?: string;
  /** Always serve demo fixtures and never call the network. */
  demoMode?: boolean;
  fetchImpl?: typeof fetch;
  timeoutMs?: number;
  now?: () => Date;
}

interface RequestSpec<T> {
  method: "GET" | "POST" | "PATCH" | "DELETE";
  path: string;
  query?: Record<string, string | number | undefined | null>;
  body?: unknown;
  guard: Guard<T>;
  /** Only GET and the demo-safe writes fall back to fixtures when the API is unreachable. */
  allowDemo?: boolean;
}

const DEFAULT_BASE = "http://localhost:8080";
const API_PREFIX = "/v1";

function envBase(): string {
  const fromEnv = process.env.NEXT_PUBLIC_TRACE_API_URL;
  return (fromEnv && fromEnv.trim()) || DEFAULT_BASE;
}

function envDemo(): boolean {
  return process.env.NEXT_PUBLIC_TRACE_DEMO === "1";
}

export function createApiClient(options: ClientOptions = {}) {
  const baseUrl = (options.baseUrl ?? envBase()).replace(/\/+$/, "");
  const demoMode = options.demoMode ?? envDemo();
  const timeoutMs = options.timeoutMs ?? 8000;
  const now = options.now ?? (() => new Date());

  function serveDemo<T>(spec: RequestSpec<T>, why: string): T {
    const res = demoResponse(spec.method, spec.path, spec.query ?? {}, spec.body, now());
    if (!res.found) {
      throw new ApiError({
        kind: "api",
        code: "demo_not_available",
        message: "No demo data exists for this request.",
        status: 404,
      });
    }
    markDemoActive(why);
    return res.data as T;
  }

  function buildUrl(spec: RequestSpec<unknown>): string {
    const url = new URL(`${baseUrl}${API_PREFIX}${spec.path}`);
    for (const [k, v] of Object.entries(spec.query ?? {})) {
      if (v !== undefined && v !== null && v !== "") url.searchParams.set(k, String(v));
    }
    return url.toString();
  }

  async function request<T>(spec: RequestSpec<T>): Promise<T> {
    const allowDemo = spec.allowDemo ?? true;
    if (demoMode && allowDemo) {
      return serveDemo(spec, "Demo mode is on (NEXT_PUBLIC_TRACE_DEMO=1).");
    }

    const fetchFn = options.fetchImpl ?? fetch;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    const headers: Record<string, string> = { Accept: "application/json" };
    const token = getToken();
    if (token) headers.Authorization = `Bearer ${token}`;
    if (spec.body !== undefined) headers["Content-Type"] = "application/json";

    let res: Response;
    try {
      res = await fetchFn(buildUrl(spec), {
        method: spec.method,
        headers,
        body: spec.body !== undefined ? JSON.stringify(spec.body) : undefined,
        signal: controller.signal,
      });
    } catch (err) {
      clearTimeout(timer);
      const timedOut = controller.signal.aborted;
      const apiErr = new ApiError({
        kind: timedOut ? "timeout" : "network",
        code: timedOut ? "timeout" : "network_error",
        message: timedOut
          ? "The TRACE API took too long to respond."
          : "The TRACE API could not be reached.",
        details: err instanceof Error ? err.message : undefined,
      });
      if (allowDemo) {
        const demoOk = demoResponse(spec.method, spec.path, spec.query ?? {}, spec.body, now()).found;
        if (demoOk) return serveDemo(spec, "The TRACE API is unreachable, so demo fixtures are shown.");
      }
      throw apiErr;
    }
    clearTimeout(timer);

    const text = await res.text().catch(() => "");
    let parsed: unknown = undefined;
    if (text) {
      try {
        parsed = JSON.parse(text);
      } catch {
        parsed = undefined;
      }
    }

    if (!res.ok) {
      const body = isRecord(parsed) ? (parsed as Partial<ApiErrorBody>) : {};
      throw new ApiError({
        kind: res.status === 401 ? "unauthorized" : "api",
        code: typeof body.code === "string" ? body.code : `http_${res.status}`,
        message: typeof body.message === "string" ? body.message : `Request failed with status ${res.status}.`,
        status: res.status,
        requestId: typeof body.request_id === "string" ? body.request_id : null,
        details: body.details,
      });
    }

    if (res.status === 204 || text === "") {
      return undefined as T;
    }

    if (!spec.guard(parsed)) {
      throw new ApiError({
        kind: "invalid_response",
        code: "invalid_response",
        message: "The TRACE API returned data in an unexpected shape.",
        status: res.status,
      });
    }

    if (hasEnvelope(parsed) && (parsed.demo || parsed.data_status === "demo")) {
      markDemoActive("The API marked this data as demo.");
    }
    return parsed;
  }

  const enc = encodeURIComponent;

  return {
    baseUrl,
    request,

    /* auth and account */
    async authGoogle(idToken: string): Promise<AuthSession> {
      const s = await request<AuthSession>({
        method: "POST",
        path: "/auth/google",
        body: { id_token: idToken },
        guard: guardAuth as Guard<AuthSession>,
        allowDemo: false,
      });
      setToken(s.access_token);
      return s;
    },
    async logout(): Promise<void> {
      try {
        await request<void>({ method: "POST", path: "/auth/logout", guard: guardAny as Guard<void>, allowDemo: false });
      } finally {
        clearToken();
      }
    },
    async deleteAccount(): Promise<void> {
      await request<void>({ method: "DELETE", path: "/me", guard: guardAny as Guard<void>, allowDemo: false });
      clearToken();
    },
    me: () => request<Me>({ method: "GET", path: "/me", guard: guardMe as Guard<Me> }),
    myWallets: () => request<MyWallets>({ method: "GET", path: "/me/wallets", guard: guardAny as Guard<MyWallets> }),

    /* watchlist and alerts */
    watchlists: () =>
      request<WatchlistResponse>({ method: "GET", path: "/watchlists", guard: guardWatchlist as Guard<WatchlistResponse> }),
    addWatchlist: (body: AddWatchlistBody) =>
      request<WatchlistItem>({
        method: "POST",
        path: "/watchlists",
        body,
        guard: guardIdCreated as unknown as Guard<WatchlistItem>,
      }),
    patchWatchlist: (id: string, body: PatchWatchlistBody) =>
      request<WatchlistItem>({
        method: "PATCH",
        path: `/watchlists/${enc(id)}`,
        body,
        guard: guardIdCreated as unknown as Guard<WatchlistItem>,
      }),
    deleteWatchlist: (id: string) =>
      request<void>({ method: "DELETE", path: `/watchlists/${enc(id)}`, guard: guardAny as Guard<void> }),
    alertPreferences: () =>
      request<AlertPreferences>({ method: "GET", path: "/alert-preferences", guard: guardAny as Guard<AlertPreferences> }),
    patchAlertPreferences: (body: Partial<AlertPreferences>) =>
      request<AlertPreferences>({
        method: "PATCH",
        path: "/alert-preferences",
        body,
        guard: guardAny as Guard<AlertPreferences>,
      }),
    alerts: (cursor?: string) =>
      request<AlertsResponse>({
        method: "GET",
        path: "/alerts",
        query: { cursor },
        guard: guardAlerts as Guard<AlertsResponse>,
      }),

    /* discovery */
    movers: (p: { tab: MoverTab; window: MoverWindow; chain?: Chain }) =>
      request<MoversResponse>({
        method: "GET",
        path: "/discovery/movers",
        query: { tab: p.tab, window: p.window, chain: p.chain },
        guard: guardMovers as Guard<MoversResponse>,
      }),
    discoveryWallets: (p: { tab: WalletTab; chain?: Chain; cursor?: string }) =>
      request<DiscoveryWalletsResponse>({
        method: "GET",
        path: "/discovery/wallets",
        query: { tab: p.tab, chain: p.chain, cursor: p.cursor },
        guard: guardDiscoveryWallets as Guard<DiscoveryWalletsResponse>,
      }),

    /* wallet research */
    wallet: (chain: Chain, address: string) =>
      request<WalletDetail>({
        method: "GET",
        path: `/wallets/${chain}/${enc(address)}`,
        guard: guardWalletDetail as Guard<WalletDetail>,
      }),
    walletTrades: (chain: Chain, address: string, cursor?: string) =>
      request<WalletTradesResponse>({
        method: "GET",
        path: `/wallets/${chain}/${enc(address)}/trades`,
        query: { cursor },
        guard: guardWalletTrades as Guard<WalletTradesResponse>,
      }),
    walletMetrics: (chain: Chain, address: string) =>
      request<WalletMetricsResponse>({
        method: "GET",
        path: `/wallets/${chain}/${enc(address)}/metrics`,
        guard: guardWalletMetrics as Guard<WalletMetricsResponse>,
      }),
    walletDna: (chain: Chain, address: string) =>
      request<WalletDna>({
        method: "GET",
        path: `/wallets/${chain}/${enc(address)}/dna`,
        guard: guardDna as Guard<WalletDna>,
      }),

    /* tokens */
    token: (chain: Chain, address: string) =>
      request<TokenDetail>({
        method: "GET",
        path: `/tokens/${chain}/${enc(address)}`,
        guard: guardToken as Guard<TokenDetail>,
      }),
    tokenTimeline: (chain: Chain, address: string) =>
      request<TimelineResponse>({
        method: "GET",
        path: `/tokens/${chain}/${enc(address)}/timeline`,
        guard: guardTimeline as Guard<TimelineResponse>,
      }),
    compare: (a: string, b: string) =>
      request<CompareResponse>({
        method: "GET",
        path: "/tokens/compare",
        query: { a, b },
        guard: guardCompare as Guard<CompareResponse>,
      }),

    /* shadow (paper) strategies */
    createShadowStrategy: (body: ShadowStrategyBody) =>
      request<ShadowStrategyCreated>({
        method: "POST",
        path: "/shadow-strategies",
        body,
        guard: guardIdCreated as Guard<ShadowStrategyCreated>,
      }),
    shadowResults: (id: string) =>
      request<ShadowResults>({
        method: "GET",
        path: `/shadow-strategies/${enc(id)}/results`,
        guard: guardShadowResults as Guard<ShadowResults>,
      }),

    /* briefings and coverage */
    briefings: () =>
      request<BriefingResponse>({ method: "GET", path: "/briefings", guard: guardBriefing as Guard<BriefingResponse> }),
    chains: () => request<ChainsResponse>({ method: "GET", path: "/chains", guard: guardAny as Guard<ChainsResponse> }),
  };
}

export type ApiClient = ReturnType<typeof createApiClient>;

/** Shared browser instance. */
export const api: ApiClient = createApiClient();

export function describeError(err: unknown): { title: string; message: string; requestId: string | null } {
  if (err instanceof ApiError) {
    switch (err.kind) {
      case "network":
        return { title: "Cannot reach TRACE", message: err.message, requestId: null };
      case "timeout":
        return { title: "TRACE is slow to respond", message: err.message, requestId: null };
      case "unauthorized":
        return { title: "Sign in required", message: "Sign in with Google to use this part of TRACE.", requestId: err.requestId };
      case "invalid_response":
        return { title: "Unexpected response", message: err.message, requestId: err.requestId };
      default:
        return { title: "Request failed", message: err.message, requestId: err.requestId };
    }
  }
  return { title: "Something went wrong", message: err instanceof Error ? err.message : "Unknown error.", requestId: null };
}

export function isDemoEnv(): boolean {
  return envDemo();
}
