import { AsyncLocalStorage } from "async_hooks";

export interface SpanContext {
  trace_id: string;
  span_id: string;
  parent_span_id?: string;
}

const als = new AsyncLocalStorage<SpanContext>();

let spanCounter = 0;

export function generateId(): string {
  return `${Date.now().toString(36)}-${(++spanCounter).toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

export function runWithSpan<T>(handler: () => T): T {
  const parent = als.getStore();
  const span: SpanContext = {
    trace_id: parent?.trace_id ?? generateId(),
    span_id: generateId(),
    parent_span_id: parent?.span_id,
  };
  return als.run(span, handler);
}

export function currentSpan(): SpanContext | undefined {
  return als.getStore();
}
