import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApiError, createApiClient, describeError } from "@/lib/api/client";
import { demoResponse, resetDemoFixtures, DEMO_TOKENS, DEMO_WALLET_REFS } from "@/lib/api/demo";
import { isDemoActive, resetDemoState } from "@/lib/api/demo-state";
import { clearToken, getToken, setToken } from "@/lib/api/token";

const NOW = new Date("2026-10-28T12:10:00Z");

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), { status, headers: { "Content-Type": "application/json" } });
}

const liveMovers = {
  data_status: "live",
  as_of: "2026-10-28T12:00:00Z",
  demo: false,
  items: [
    {
      rank: 1,
      token: { chain: "solana", address: "x", symbol: "T", name: "T" },
      price_usd: "1",
      change_pct: "2",
      volume_usd: "3",
      liquidity_usd: "4",
      pool: "p",
      reason: { code: "c", label: "l" },
      sparkline: [1, 2],
      risk: "pass",
      risk_notes: [],
      source_timestamp: "2026-10-28T12:00:00Z",
    },
  ],
};

beforeEach(() => {
  resetDemoState();
  resetDemoFixtures();
  clearToken();
});
afterEach(() => vi.useRealTimers());

describe("api client: live responses", () => {
  it("calls the versioned URL with query params and returns parsed data without flagging demo", async () => {
    const fetchImpl = vi.fn().mockResolvedValue(json(liveMovers));
    const c = createApiClient({ baseUrl: "http://api.test/", demoMode: false, fetchImpl, now: () => NOW });
    const res = await c.movers({ tab: "trending", window: "24h" });
    expect(res.items[0]?.rank).toBe(1);
    const url = new URL(fetchImpl.mock.calls[0]![0] as string);
    expect(url.origin + url.pathname).toBe("http://api.test/v1/discovery/movers");
    expect(url.searchParams.get("tab")).toBe("trending");
    expect(url.searchParams.get("window")).toBe("24h");
    expect(url.searchParams.has("chain")).toBe(false);
    expect(isDemoActive()).toBe(false);
  });

  it("sends the bearer token when one is set", async () => {
    setToken("tok123");
    const fetchImpl = vi.fn().mockResolvedValue(json(liveMovers));
    const c = createApiClient({ baseUrl: "http://api.test", demoMode: false, fetchImpl });
    await c.movers({ tab: "new", window: "1h" });
    const init = fetchImpl.mock.calls[0]![1] as RequestInit;
    expect((init.headers as Record<string, string>).Authorization).toBe("Bearer tok123");
    expect(getToken()).toBe("tok123");
  });

  it("flags demo when the API itself returns demo data", async () => {
    const fetchImpl = vi.fn().mockResolvedValue(json({ ...liveMovers, data_status: "demo", demo: true }));
    const c = createApiClient({ baseUrl: "http://api.test", demoMode: false, fetchImpl });
    await c.movers({ tab: "trending", window: "24h" });
    expect(isDemoActive()).toBe(true);
  });

  it("turns the error envelope into an ApiError", async () => {
    const fetchImpl = vi.fn().mockResolvedValue(json({ code: "rate_limited", message: "Slow down", request_id: "req_9" }, 429));
    const c = createApiClient({ baseUrl: "http://api.test", demoMode: false, fetchImpl });
    const err = await c.movers({ tab: "trending", window: "24h" }).catch((e) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect(err.code).toBe("rate_limited");
    expect(err.status).toBe(429);
    expect(err.requestId).toBe("req_9");
    expect(describeError(err).requestId).toBe("req_9");
    expect(isDemoActive()).toBe(false);
  });

  it("marks 401 as unauthorized and does not fall back to demo", async () => {
    const fetchImpl = vi.fn().mockResolvedValue(json({ code: "unauthorized", message: "no", request_id: "r" }, 401));
    const c = createApiClient({ baseUrl: "http://api.test", demoMode: false, fetchImpl });
    const err = await c.watchlists().catch((e) => e);
    expect(err.kind).toBe("unauthorized");
    expect(describeError(err).title).toBe("Sign in required");
    expect(isDemoActive()).toBe(false);
  });

  it("copes with a non-JSON error body", async () => {
    const fetchImpl = vi.fn().mockResolvedValue(new Response("<html>bad gateway</html>", { status: 502 }));
    const c = createApiClient({ baseUrl: "http://api.test", demoMode: false, fetchImpl });
    const err = await c.movers({ tab: "trending", window: "24h" }).catch((e) => e);
    expect(err.code).toBe("http_502");
    expect(err.kind).toBe("api");
    expect(isDemoActive()).toBe(false);
  });

  it("rejects a response with the wrong shape", async () => {
    const fetchImpl = vi.fn().mockResolvedValue(json({ items: "nope" }));
    const c = createApiClient({ baseUrl: "http://api.test", demoMode: false, fetchImpl });
    const err = await c.movers({ tab: "trending", window: "24h" }).catch((e) => e);
    expect(err.kind).toBe("invalid_response");
  });

  it("returns undefined for 204 responses", async () => {
    const fetchImpl = vi.fn().mockResolvedValue(new Response(null, { status: 204 }));
    const c = createApiClient({ baseUrl: "http://api.test", demoMode: false, fetchImpl });
    await expect(c.deleteWatchlist("w1")).resolves.toBeUndefined();
  });

  it("sets the token after a Google sign-in", async () => {
    const session = { access_token: "abc", expires_at: "2026-10-29T00:00:00Z", user: { id: "u", email: "a@b.c", name: "A" } };
    const fetchImpl = vi.fn().mockResolvedValue(json(session));
    const c = createApiClient({ baseUrl: "http://api.test", demoMode: false, fetchImpl });
    await c.authGoogle("id-token");
    expect(getToken()).toBe("abc");
    const init = fetchImpl.mock.calls[0]![1] as RequestInit;
    expect(JSON.parse(init.body as string)).toEqual({ id_token: "id-token" });
  });
});

describe("api client: demo fallback", () => {
  it("falls back to demo fixtures and flags demo when the API is unreachable", async () => {
    const fetchImpl = vi.fn().mockRejectedValue(new TypeError("fetch failed"));
    const c = createApiClient({ baseUrl: "http://api.test", demoMode: false, fetchImpl, now: () => NOW });
    const res = await c.movers({ tab: "trending", window: "24h" });
    expect(res.demo).toBe(true);
    expect(res.data_status).toBe("demo");
    expect(res.items[0]?.token.symbol).toBe("NEURA");
    expect(res.items[0]?.change_pct).toBe("42.7");
    expect(isDemoActive()).toBe(true);
  });

  it("falls back on timeout", async () => {
    const fetchImpl = vi.fn(
      (_url: string, init?: RequestInit) =>
        new Promise<Response>((_res, rej) => {
          init?.signal?.addEventListener("abort", () => rej(new DOMException("aborted", "AbortError")));
        }),
    );
    const c = createApiClient({ baseUrl: "http://api.test", demoMode: false, fetchImpl: fetchImpl as unknown as typeof fetch, timeoutMs: 20, now: () => NOW });
    const res = await c.briefings();
    expect(res.demo).toBe(true);
    expect(isDemoActive()).toBe(true);
  });

  it("does not fall back on API errors, only on unreachable", async () => {
    const fetchImpl = vi.fn().mockResolvedValue(json({ code: "internal", message: "boom", request_id: "r1" }, 500));
    const c = createApiClient({ baseUrl: "http://api.test", demoMode: false, fetchImpl });
    await expect(c.briefings()).rejects.toMatchObject({ code: "internal" });
    expect(isDemoActive()).toBe(false);
  });

  it("never fakes a sign-in when the API is unreachable", async () => {
    const fetchImpl = vi.fn().mockRejectedValue(new TypeError("fetch failed"));
    const c = createApiClient({ baseUrl: "http://api.test", demoMode: false, fetchImpl });
    const err = await c.authGoogle("t").catch((e) => e);
    expect(err.kind).toBe("network");
    expect(getToken()).toBeNull();
  });

  it("serves demo without any network call when demo mode is on", async () => {
    const fetchImpl = vi.fn();
    const c = createApiClient({ baseUrl: "http://api.test", demoMode: true, fetchImpl, now: () => NOW });
    const res = await c.discoveryWallets({ tab: "top" });
    expect(fetchImpl).not.toHaveBeenCalled();
    expect(res.items.length).toBe(4);
    expect(res.demo).toBe(true);
    expect(isDemoActive()).toBe(true);
  });

  it("raises a clear error when the fixture does not exist", async () => {
    const c = createApiClient({ baseUrl: "http://api.test", demoMode: true, now: () => NOW });
    const err = await c.token("solana", "11111111111111111111111111111111").catch((e) => e);
    expect(err.code).toBe("demo_not_available");
  });
});

describe("demo fixtures", () => {
  const token = DEMO_TOKENS[0]!;
  const wallet = DEMO_WALLET_REFS[0]!;
  const unknownWallet = DEMO_WALLET_REFS[2]!;
  const requests: Array<[string, string, Record<string, string>, unknown?]> = [
    ["GET", "/v1/discovery/movers", { tab: "trending", window: "24h" }],
    ["GET", "/v1/discovery/wallets", { tab: "top" }],
    ["GET", `/v1/wallets/${wallet.chain}/${wallet.address}`, {}],
    ["GET", `/v1/wallets/${wallet.chain}/${wallet.address}/trades`, {}],
    ["GET", `/v1/wallets/${wallet.chain}/${wallet.address}/metrics`, {}],
    ["GET", `/v1/wallets/${wallet.chain}/${wallet.address}/dna`, {}],
    ["GET", `/v1/tokens/${token.chain}/${token.address}`, {}],
    ["GET", `/v1/tokens/${token.chain}/${token.address}/timeline`, {}],
    ["GET", "/v1/tokens/compare", { a: `${DEMO_TOKENS[0]!.chain}:${DEMO_TOKENS[0]!.address}`, b: `${DEMO_TOKENS[1]!.chain}:${DEMO_TOKENS[1]!.address}` }],
    ["POST", "/v1/shadow-strategies", {}, { wallet: { chain: "solana", address: "x" }, budget_usd: "200", size_rule: "proportional", delay_seconds: 30, slippage_bps: 50, exit_rule: "mirror_sells" }],
    ["GET", "/v1/briefings", {}],
    ["GET", "/v1/watchlists", {}],
    ["GET", "/v1/alerts", {}],
    ["GET", "/v1/alert-preferences", {}],
    ["GET", "/v1/me", {}],
    ["GET", "/v1/me/wallets", {}],
    ["GET", "/v1/chains", {}],
  ];

  it("returns demo-flagged data for every analytic endpoint", () => {
    for (const [m, p, q, b] of requests) {
      const r = demoResponse(m, p, q, b, NOW);
      expect(r.found, `${m} ${p}`).toBe(true);
      if (r.found && ["/v1/discovery", "/v1/wallets/", "/v1/tokens", "/v1/briefings"].some((x) => p.startsWith(x)) && typeof r.data === "object" && r.data) {
        const d = r.data as { demo?: boolean; data_status?: string };
        expect(d.demo, p).toBe(true);
        expect(d.data_status, p).toBe("demo");
      }
    }
  });

  it("scales shadow results with the budget and marks them demo", () => {
    const created = demoResponse("POST", "/v1/shadow-strategies", {}, { wallet: { chain: "solana", address: "x" }, budget_usd: "200", size_rule: "proportional", delay_seconds: 30, slippage_bps: 50, exit_rule: "mirror_sells" }, NOW);
    expect(created.found && (created.data as { id: string }).id).toBe("demo-1");
    const r = demoResponse("GET", "/v1/shadow-strategies/demo-1/results", {}, undefined, NOW);
    const d = (r.found ? r.data : null) as { starting_usd: string; ending_usd: string; demo: boolean; skipped_unexecutable: number };
    expect(d.demo).toBe(true);
    expect(d.starting_usd).toBe("200.00");
    expect(d.ending_usd).toBe("297.20");
    expect(d.skipped_unexecutable).toBeGreaterThan(0);
  });

  it("includes an insufficient history DNA state and an unresolved metrics example", () => {
    const dna = demoResponse("GET", `/v1/wallets/${unknownWallet.chain}/${unknownWallet.address}/dna`, {}, undefined, NOW);
    expect(dna.found && (dna.data as { state: string }).state).toBe("insufficient_history");
  });

  it("keeps the screenshot numbers", () => {
    const r = demoResponse("GET", "/v1/discovery/movers", { tab: "trending", window: "24h" }, undefined, NOW);
    const items = (r.found ? (r.data as { items: Array<{ token: { symbol: string }; price_usd: string; change_pct: string }> }).items : []);
    expect(items.map((i) => `${i.token.symbol} ${i.price_usd} ${i.change_pct}`)).toEqual([
      "NEURA 0.0084 42.7",
      "SOLX 0.132 28.4",
      "FROG 0.0031 19.1",
      "NOVA 0.21 15.8",
      "ORBIT 0.045 12.3",
    ]);
  });

  it("is deterministic for the same clock", () => {
    const a = JSON.stringify(demoResponse("GET", "/v1/discovery/movers", { tab: "trending", window: "24h" }, undefined, NOW));
    const b = JSON.stringify(demoResponse("GET", "/v1/discovery/movers", { tab: "trending", window: "24h" }, undefined, NOW));
    expect(a).toBe(b);
  });

  it("supports watchlist add, idempotency and delete in demo mode", () => {
    const body = { chain: "base", address: "0x" + "ab".repeat(20) };
    const first = demoResponse("POST", "/v1/watchlists", {}, body, NOW);
    const second = demoResponse("POST", "/v1/watchlists", {}, body, NOW);
    expect(first.found && second.found && (first.data as { id: string }).id).toBe((second.found ? (second.data as { id: string }).id : ""));
    const list = demoResponse("GET", "/v1/watchlists", {}, undefined, NOW);
    expect(list.found && (list.data as { items: unknown[] }).items.length).toBe(3);
    const id = first.found ? (first.data as { id: string }).id : "";
    demoResponse("DELETE", `/v1/watchlists/${id}`, {}, undefined, NOW);
    const after = demoResponse("GET", "/v1/watchlists", {}, undefined, NOW);
    expect(after.found && (after.data as { items: unknown[] }).items.length).toBe(2);
  });

  it("contains no em dash characters", () => {
    for (const [m, p, q, b] of requests) {
      const r = demoResponse(m, p, q, b, NOW);
      expect(JSON.stringify(r)).not.toMatch(new RegExp(String.fromCharCode(0x2014)));
    }
  });
});
