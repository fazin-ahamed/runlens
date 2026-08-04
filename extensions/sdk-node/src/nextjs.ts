import { runWithSpan, currentSpan, generateId, type SpanContext } from "./context.js";
import type { DaemonClient } from "./_client.js";

interface NextHandler {
  (req: any, res: any): Promise<any>;
}

export function wrapNextApiHandler(client: DaemonClient, sessionId: string, projectId: string, handler: NextHandler): NextHandler {
  return async (req: any, res: any) => {
    const spanId = generateId();
    const start = Date.now();
    let span: SpanContext | undefined;
    try {
      await runWithSpan(() => {
        span = currentSpan();
        return handler(req, res);
      });
    } finally {
      client.emit({
        eventId: spanId,
        sessionId: sessionId,
        projectId: projectId,
        sequence: 0,
        source: "sdk",
        kind: "next.api",
        severity: (res.statusCode || 200) >= 500 ? "error" : (res.statusCode || 200) >= 400 ? "warning" : "info",
        utcTimestamp: new Date().toISOString(),
        monotonicNs: Date.now() * 1_000_000,
        durationNs: (Date.now() - start) * 1_000_000,
        traceId: span?.trace_id,
        spanId: span?.span_id,
        parentSpanId: span?.parent_span_id,
        spanKind: "server",
        payload: { url: req.url, method: req.method, status: res.statusCode },
        classification: "public",
      }).catch(() => {});
    }
  };
}
