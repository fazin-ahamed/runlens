import type { Request, Response, NextFunction } from "express";
import { runWithSpan, currentSpan } from "./context.js";
import type { DaemonClient } from "./_client.js";

export function createExpressMiddleware(client: DaemonClient, sessionId: string, projectId: string) {
  return (req: Request, res: Response, next: NextFunction) => {
    try {
      runWithSpan(() => {
        const span = currentSpan()!;
        const start = Date.now();
        res.on("finish", () => {
          client.emit({
            eventId: span.span_id,
            sessionId: sessionId,
            projectId: projectId,
            sequence: 0,
            source: "sdk",
            kind: "http.server.request",
            severity: res.statusCode >= 500 ? "error" : res.statusCode >= 400 ? "warning" : "info",
            utcTimestamp: new Date().toISOString(),
            monotonicNs: Date.now() * 1_000_000,
            durationNs: (Date.now() - start) * 1_000_000,
            traceId: span.trace_id,
            spanId: span.span_id,
            parentSpanId: span.parent_span_id,
            spanKind: "server",
            payload: { method: req.method, path: req.path, status: res.statusCode },
            classification: "public",
          }).catch(() => {});
        });
        next();
      });
    } catch (e) {
      next(e);
    }
  };
}
