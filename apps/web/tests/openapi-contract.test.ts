import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const CLIENT_OPERATIONS: ReadonlyArray<readonly [string, string]> = [
  ["/v1/auth/google", "post"],
  ["/v1/auth/logout", "post"],
  ["/v1/auth/logout-all", "post"],
  ["/v1/auth/sessions", "get"],
  ["/v1/auth/sessions/{id}", "delete"],
  ["/v1/me", "get"],
  ["/v1/me", "delete"],
  ["/v1/me/wallets", "get"],
  ["/v1/watchlists", "get"],
  ["/v1/watchlists", "post"],
  ["/v1/watchlists/{id}", "patch"],
  ["/v1/watchlists/{id}", "delete"],
  ["/v1/alert-preferences", "get"],
  ["/v1/alert-preferences", "patch"],
  ["/v1/alerts", "get"],
  ["/v1/discovery/movers", "get"],
  ["/v1/discovery/wallets", "get"],
  ["/v1/wallets/{chain}/{address}", "get"],
  ["/v1/wallets/{chain}/{address}/trades", "get"],
  ["/v1/wallets/{chain}/{address}/metrics", "get"],
  ["/v1/wallets/{chain}/{address}/dna", "get"],
  ["/v1/tokens/{chain}/{address}", "get"],
  ["/v1/tokens/{chain}/{address}/timeline", "get"],
  ["/v1/tokens/compare", "get"],
  ["/v1/shadow-strategies", "post"],
  ["/v1/shadow-strategies/{id}/results", "get"],
  ["/v1/briefings", "get"],
  ["/v1/chains", "get"],
];

describe("OpenAPI client contract", () => {
  it("contains every endpoint and method used by the web client", () => {
    const file = resolve(__dirname, "../../../docs/openapi.snapshot.json");
    const document = JSON.parse(readFileSync(file, "utf8")) as {
      paths?: Record<string, Record<string, unknown>>;
    };
    expect(document.paths).toBeDefined();
    for (const [path, method] of CLIENT_OPERATIONS) {
      expect(document.paths?.[path], `missing ${path}`).toBeDefined();
      expect(document.paths?.[path]?.[method], `missing ${method.toUpperCase()} ${path}`).toBeDefined();
    }
  });
});
