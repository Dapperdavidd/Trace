import { afterEach, describe, expect, it } from "vitest";
import { clearToken, getToken, setToken } from "@/lib/api/token";

afterEach(() => clearToken());

describe("token store", () => {
  it("keeps the token in memory even when sessionStorage is missing", () => {
    expect(typeof sessionStorage).toBe("undefined");
    setToken("abc");
    expect(getToken()).toBe("abc");
    clearToken();
    expect(getToken()).toBeNull();
  });

  it("survives a throwing sessionStorage", () => {
    const g = globalThis as unknown as { sessionStorage?: unknown };
    g.sessionStorage = {
      getItem() { throw new Error("blocked"); },
      setItem() { throw new Error("blocked"); },
      removeItem() { throw new Error("blocked"); },
    };
    try {
      expect(() => setToken("t")).not.toThrow();
      expect(getToken()).toBe("t");
      expect(() => clearToken()).not.toThrow();
    } finally {
      delete g.sessionStorage;
    }
  });
});
