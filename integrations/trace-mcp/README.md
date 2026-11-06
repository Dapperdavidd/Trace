# TRACE agent MCP bridge

This stdio MCP server lets a compatible external agent read TRACE context and
prepare dry NEAR Intents proposals. It cannot execute a trade.

Create an agent connection on the TRACE NEAR page and copy the token when it is
shown. Configure the MCP process with:

```json
{
  "command": "node",
  "args": ["/absolute/path/to/trace/integrations/trace-mcp/dist/server.js"],
  "env": {
    "TRACE_API_URL": "https://your-trace-api.example",
    "TRACE_AGENT_TOKEN": "trca_token_shown_once"
  }
}
```

The bridge exposes:

- `trace_context`: read watched public addresses, labelled alerts and active
  anchored mandate limits;
- `trace_preview_intent`: prepare and retain a dry quote after the API checks
  the anchored mandate's assets, value, slippage and expiry.

The token has no wallet key and no execute scope. Revoke it from TRACE at any
time. A NEAR owner signature or separately reviewed delegated NEAR agent is
still required for any live action.
