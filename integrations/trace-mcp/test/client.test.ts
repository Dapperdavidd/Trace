import assert from "node:assert/strict";
import test from "node:test";
import { TraceAgentClient, TraceAgentError } from "../src/client.js";

test("reads context with the scoped agent token", async () => {
  let request: Request | undefined;
  const client = new TraceAgentClient({
    baseUrl: "https://trace.example/",
    token: `trca_${"a".repeat(64)}`,
    fetchImpl: async (input, init) => {
      request = new Request(input, init);
      return Response.json({ near_is_control_plane: true, capabilities: { execute: false } });
    },
  });
  const result = await client.context();
  assert.equal(result.near_is_control_plane, true);
  assert.equal(request?.url, "https://trace.example/v1/agent/context");
  assert.equal(request?.headers.get("authorization"), `Bearer trca_${"a".repeat(64)}`);
});

test("posts a proposal and preserves exact string amounts", async () => {
  let body = "";
  const client = new TraceAgentClient({
    baseUrl: "https://trace.example",
    token: `trca_${"b".repeat(64)}`,
    fetchImpl: async (_input, init) => {
      body = String(init?.body);
      return Response.json({ execution: "not_submitted" });
    },
  });
  await client.previewIntent({ mandate_id: "m", quote: { amount: "1000000000000000000000000" } });
  assert.equal(JSON.parse(body).quote.amount, "1000000000000000000000000");
});

test("surfaces the TRACE error envelope without leaking the token", async () => {
  const token = `trca_${"c".repeat(64)}`;
  const client = new TraceAgentClient({
    baseUrl: "https://trace.example",
    token,
    fetchImpl: async () => Response.json({
      code: "forbidden",
      message: "scope missing",
      request_id: "req-1",
    }, { status: 403 }),
  });
  await assert.rejects(
    client.context(),
    (error: unknown) => error instanceof TraceAgentError
      && error.code === "forbidden"
      && error.requestId === "req-1"
      && !error.message.includes(token),
  );
});

test("rejects app sessions and non-http URLs", () => {
  assert.throws(() => new TraceAgentClient({ baseUrl: "https://trace.example", token: "trc_session" }));
  assert.throws(() => new TraceAgentClient({ baseUrl: "file:///tmp/trace", token: `trca_${"d".repeat(64)}` }));
});
