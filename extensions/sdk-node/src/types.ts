// Generated — do not edit. Regen via: cd extensions/sdk-protocol && python generate.py

export interface EventV2 {

  eventId: string;

  sessionId: string;

  projectId: string;

  sequence: number;

  kind: string;

  severity: string;

  utcTimestamp: string;

  monotonicNs: number;

  payload: Record<string, unknown>;

  classification: string;


  source?: string;

  durationNs?: number;

  threadId?: string;

  traceId?: string;

  spanId?: string;

  parentSpanId?: string;

  spanKind?: string;

  correlationId?: string;

  parentEventId?: string;

  correlationIds?: unknown[];

  envelopeVersion?: number;

  schemaVersion?: number;

}

export const METHODS = {

  SESSION_START: "session.start",

  SESSION_STOP: "session.stop",

  EVENT_EMIT: "event.emit",

  EVENT_EMIT_BATCH: "event.emit_batch",

  DAEMON_STATUS: "daemon.status",

} as const;

export const DAEMON_URL = "ws://127.0.0.1:9876";