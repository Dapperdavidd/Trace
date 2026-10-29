const KEY = "trace.access_token";
let memoryToken: string | null = null;
const listeners = new Set<() => void>();

function readStorage(): string | null {
  try {
    if (typeof sessionStorage === "undefined") return null;
    return sessionStorage.getItem(KEY);
  } catch {
    return null;
  }
}

export function getToken(): string | null {
  if (memoryToken) return memoryToken;
  const stored = readStorage();
  if (stored) memoryToken = stored;
  return memoryToken;
}

export function setToken(token: string): void {
  memoryToken = token;
  try {
    if (typeof sessionStorage !== "undefined") sessionStorage.setItem(KEY, token);
  } catch {
    /* storage can be blocked; the in-memory token still works for this tab */
  }
  listeners.forEach((l) => l());
}

export function clearToken(): void {
  memoryToken = null;
  try {
    if (typeof sessionStorage !== "undefined") sessionStorage.removeItem(KEY);
  } catch {
    /* ignore */
  }
  listeners.forEach((l) => l());
}

export function subscribeToken(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
