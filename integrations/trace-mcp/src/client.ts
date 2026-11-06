export type JsonObject = Record<string, unknown>;

export interface TraceAgentClientOptions {
  baseUrl: string;
  token: string;
  fetchImpl?: typeof fetch;
}

export class TraceAgentError extends Error {
  constructor(
    message: string,
    readonly code: string,
    readonly status: number,
    readonly requestId: string | null,
  ) {
    super(message);
    this.name = "TraceAgentError";
  }
}

export class TraceAgentClient {
  readonly baseUrl: string;
  private readonly token: string;
  private readonly fetchImpl: typeof fetch;

  constructor(options: TraceAgentClientOptions) {
    const url = new URL(options.baseUrl);
    if (!['http:', 'https:'].includes(url.protocol)) {
      throw new Error("TRACE_API_URL must be an HTTP or HTTPS URL");
    }
    if (!options.token.startsWith("trca_") || options.token.length > 200) {
      throw new Error("TRACE_AGENT_TOKEN must be a scoped TRACE agent credential");
    }
    this.baseUrl = url.toString().replace(/\/$/, "");
    this.token = options.token;
    this.fetchImpl = options.fetchImpl ?? fetch;
  }

  context(): Promise<JsonObject> {
    return this.request("GET", "/v1/agent/context");
  }

  previewIntent(input: JsonObject): Promise<JsonObject> {
    return this.request("POST", "/v1/agent/intents/preview", input);
  }

  private async request(method: string, path: string, body?: JsonObject): Promise<JsonObject> {
    const response = await this.fetchImpl(`${this.baseUrl}${path}`, {
      method,
      headers: {
        authorization: `Bearer ${this.token}`,
        ...(body ? { "content-type": "application/json" } : {}),
      },
      body: body ? JSON.stringify(body) : undefined,
      signal: AbortSignal.timeout(15_000),
    });
    const parsed = await response.json().catch(() => null) as JsonObject | null;
    if (!response.ok) {
      throw new TraceAgentError(
        typeof parsed?.message === "string" ? parsed.message : `TRACE returned HTTP ${response.status}`,
        typeof parsed?.code === "string" ? parsed.code : `http_${response.status}`,
        response.status,
        typeof parsed?.request_id === "string" ? parsed.request_id : null,
      );
    }
    if (!parsed || Array.isArray(parsed)) {
      throw new TraceAgentError("TRACE returned an invalid response", "invalid_response", response.status, null);
    }
    return parsed;
  }
}
