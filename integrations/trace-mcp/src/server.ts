import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { TraceAgentClient, TraceAgentError, type JsonObject } from "./client.js";

const apiUrl = process.env.TRACE_API_URL;
const agentToken = process.env.TRACE_AGENT_TOKEN;
if (!apiUrl || !agentToken) {
  throw new Error("TRACE_API_URL and TRACE_AGENT_TOKEN are required");
}

const client = new TraceAgentClient({ baseUrl: apiUrl, token: agentToken });
const server = new McpServer({ name: "trace-near-agent", version: "0.1.0" });

function content(value: JsonObject) {
  return { content: [{ type: "text" as const, text: JSON.stringify(value, null, 2) }] };
}

function failure(error: unknown) {
  const message = error instanceof TraceAgentError
    ? `${error.message}${error.requestId ? ` Request id: ${error.requestId}` : ""}`
    : "TRACE agent request failed";
  return {
    isError: true,
    content: [{ type: "text" as const, text: message }],
  };
}

server.registerTool(
  "trace_context",
  {
    title: "Read TRACE agent context",
    description: "Read watched wallets, labelled alerts, and active NEAR mandate limits. Never returns keys or execution authority.",
    inputSchema: {},
    annotations: {
      readOnlyHint: true,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: true,
    },
  },
  async () => {
    try {
      return content(await client.context());
    } catch (error) {
      return failure(error);
    }
  },
);

server.registerTool(
  "trace_preview_intent",
  {
    title: "Prepare a NEAR-governed intent proposal",
    description: "Request a dry NEAR Intents quote inside an active anchored mandate. This cannot sign, deposit, submit, or execute a transaction.",
    inputSchema: {
      mandate_id: z.string().uuid(),
      origin_asset: z.string().min(1),
      destination_asset: z.string().min(1),
      amount: z.string().regex(/^[1-9][0-9]*$/, "Use positive integer base units"),
      slippage_tolerance: z.number().int().min(0).max(2_000),
      recipient: z.string().min(1),
      refund_to: z.string().min(1),
      deadline: z.string().datetime({ offset: true }),
    },
    annotations: {
      readOnlyHint: false,
      destructiveHint: false,
      idempotentHint: false,
      openWorldHint: true,
    },
  },
  async (input) => {
    try {
      return content(await client.previewIntent({
        mandate_id: input.mandate_id,
        quote: {
          originAsset: input.origin_asset,
          destinationAsset: input.destination_asset,
          amount: input.amount,
          slippageTolerance: input.slippage_tolerance,
          recipient: input.recipient,
          refundTo: input.refund_to,
          deadline: input.deadline,
        },
      }));
    } catch (error) {
      return failure(error);
    }
  },
);

await server.connect(new StdioServerTransport());
