import { describe, it } from "node:test";
import assert from "node:assert";
import { runWithSpan, currentSpan } from "../context.js";

describe("context", () => {
  it("creates trace and span ids inside runWithSpan", () => {
    runWithSpan(() => {
      const span = currentSpan();
      assert.ok(span);
      assert.ok(span!.trace_id);
      assert.ok(span!.span_id);
    });
  });

  it("nests spans with parent_span_id", () => {
    runWithSpan(() => {
      const outer = currentSpan()!;
      runWithSpan(() => {
        const inner = currentSpan()!;
        assert.strictEqual(inner.trace_id, outer.trace_id);
        assert.strictEqual(inner.parent_span_id, outer.span_id);
      });
    });
  });
});
