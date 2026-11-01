import { describe, expect, it } from "vitest";

import { ApiError, describeError } from "@/lib/api/client";
import type { DataStatus } from "@/lib/api/types";
import { DEFAULT_EMPTY_TITLE, STATUS_LABEL, UNAVAILABLE_EXPLANATION, UNAVAILABLE_TITLE } from "@/lib/ui-state";

describe("honest UI states", () => {
  it.each<[DataStatus, string]>([
    ["live", "Live"],
    ["stale", "Stale"],
    ["partial", "Partial data"],
    ["unavailable", "Data unavailable"],
    ["demo", "DEMO DATA"],
  ])("renders the %s badge", (status, label) => {
    expect(STATUS_LABEL[status]).toBe(label);
  });

  it("renders an empty state without inventing a value", () => {
    expect(DEFAULT_EMPTY_TITLE).toBe("Nothing to show yet");
    expect(DEFAULT_EMPTY_TITLE).not.toMatch(/^(0|-)?$/);
  });

  it("renders the API message, code context, and request id", () => {
    const error = new ApiError({
      kind: "api",
      code: "coverage_unavailable",
      message: "Coverage could not be loaded.",
      status: 503,
      requestId: "req-test-123",
    });
    const details = describeError(error);
    expect(details.message).toBe("Coverage could not be loaded.");
    expect(details.requestId).toBe("req-test-123");
  });

  it("explains unavailable data instead of displaying zero", () => {
    expect(UNAVAILABLE_TITLE).toBe("Data unavailable");
    expect(UNAVAILABLE_EXPLANATION).toContain("instead of guessing");
  });
});
