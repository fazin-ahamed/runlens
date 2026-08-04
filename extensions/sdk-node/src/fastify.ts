import { runWithSpan, currentSpan, generateId } from "./context.js";
import type { DaemonClient } from "./_client.js";

export function createFastifyPlugin(client: DaemonClient, sessionId: string, projectId: string) {
  return (fastify: any, _opts: unknown, done: () => void) => {
    fastify.addHook("onRequest", (req: any, _reply: any, done: () => void) => {
      req.spanContext = { trace_id: generateId(), span_id: generateId() };
      req.startTime = Date.now();
      done();
    });
    fastify.addHook("onResponse", (req: any, reply: any, done: () => void) => {
      const span = req.spanContext;
      if (span) {
        client.emit({
          eventId: span.span_id,
          sessionId: sessionId,
          projectId: projectId,
          sequence: 0,
          source: "sdk",
          kind: "http.server.request",
          severity: reply.statusCode >= 500 ? "error" : reply.statusCode >= 400 ? "warning" : "info",
          utcTimestamp: new Date().toISOString(),
          monotonicNs: Date.now() * 1_000_000,
          durationNs: (Date.now() - (req.startTime || Date.now())) * 1_000_000,
          traceId: span.trace_id,
          spanId: span.span_id,
          spanKind: "server",
          payload: { method: req.method, url: req.url, status: reply.statusCode },
          classification: "public",
        }).catch(() => {});
      }
      done();
    });
    done();
  };
}
