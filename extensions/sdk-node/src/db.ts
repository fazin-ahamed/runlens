import type { DaemonClient } from "./_client.js";
import { currentSpan, generateId } from "./context.js";
import type { EventV2 } from "./types.js";

function makeDbEvent(
  client: DaemonClient,
  sessionId: string,
  projectId: string,
  sql: string,
  durationNs: number,
  kind: "query" | "execute",
): void {
  const span = currentSpan();
  const normalized = sql.replace(/'[^']*'/g, "?").replace(/\b\d+\b/g, "?").replace(/\?+/g, "?").replace(/\s+/g, " ").trim();
  const ev: EventV2 = {
    eventId: generateId(),
    sessionId,
    projectId,
    sequence: 0,
    source: "sdk",
    kind: "db.query",
    severity: "info",
    utcTimestamp: new Date().toISOString(),
    monotonicNs: Date.now() * 1_000_000,
    durationNs,
    traceId: span?.trace_id,
    spanId: span?.span_id,
    parentSpanId: span?.parent_span_id,
    spanKind: "client",
    payload: { sql, sql_normalized: normalized, kind },
    classification: "public",
  };
  client.emit(ev).catch(() => {});
}

export function wrapPg(
  client: any,
  sessionId: string,
  projectId: string,
  opts?: { daemonClient?: DaemonClient },
): any {
  const dc = opts?.daemonClient!;
  const origQuery = client.query.bind(client);
  client.query = async function wrappedQuery(text: string, params?: any[], callback?: Function) {
    const start = process.hrtime.bigint();
    try {
      return await origQuery(text, params, callback);
    } finally {
      const dur = Number(process.hrtime.bigint() - start);
      makeDbEvent(dc, sessionId, projectId, text, dur, "query");
    }
  };
  return client;
}

export function wrapMysql2(
  pool: any,
  sessionId: string,
  projectId: string,
  opts?: { daemonClient?: DaemonClient },
): any {
  const dc = opts?.daemonClient!;
  const origExecute = pool.execute.bind(pool);
  pool.execute = async function wrappedExecute(sql: string, params?: any[], callback?: Function) {
    const start = process.hrtime.bigint();
    try {
      return await origExecute(sql, params, callback);
    } finally {
      const dur = Number(process.hrtime.bigint() - start);
      makeDbEvent(dc, sessionId, projectId, sql, dur, "execute");
    }
  };
  return pool;
}
