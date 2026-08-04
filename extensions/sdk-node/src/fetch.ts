import { runWithSpan, currentSpan, generateId } from "./context.js";
import type { DaemonClient } from "./_client.js";

export function wrapFetch(client: DaemonClient, sessionId: string, projectId: string): typeof globalThis.fetch {
  const orig = globalThis.fetch;
  return async (input: RequestInfo | URL, init?: RequestInit) => {
    const spanId = generateId();
    const start = Date.now();
    try {
      const response = await runWithSpan(() => orig(input, init));
      const url = typeof input === "string" ? input : input instanceof URL ? input.href : input.url;
      client.emit({
        eventId: spanId,
        sessionId: sessionId,
        projectId: projectId,
        sequence: 0,
        source: "sdk",
        kind: "http.client.request",
        severity: response.status >= 500 ? "error" : response.status >= 400 ? "warning" : "info",
        utcTimestamp: new Date().toISOString(),
        monotonicNs: Date.now() * 1_000_000,
        durationNs: (Date.now() - start) * 1_000_000,
        spanKind: "client",
        payload: { url, method: (init?.method || "GET"), status: response.status },
        classification: "public",
      });
      return response;
    } catch (err) {
      const url = typeof input === "string" ? input : input instanceof URL ? input.href : input.url;
      client.emit({
        eventId: spanId,
        sessionId: sessionId,
        projectId: projectId,
        sequence: 0,
        source: "sdk",
        kind: "http.client.error",
        severity: "error",
        utcTimestamp: new Date().toISOString(),
        monotonicNs: Date.now() * 1_000_000,
        durationNs: (Date.now() - start) * 1_000_000,
        spanKind: "client",
        payload: { url, method: (init?.method || "GET"), error: String(err) },
        classification: "sensitive",
      });
      throw err;
    }
  };
}
