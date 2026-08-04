import { describe, it, mock } from "node:test";
import assert from "node:assert";
import { createExpressMiddleware } from "../express.js";
import { wrapFetch } from "../fetch.js";

describe("node middleware", () => {
  it("express middleware returns a function", () => {
    const client = { emit: mock.fn() } as any;
    const mw = createExpressMiddleware(client, "s1", "p1");
    assert.strictEqual(typeof mw, "function");
    assert.strictEqual(mw.length, 3);
  });

  it("fetch wrapper wraps global fetch", async () => {
    const client = { emit: mock.fn() } as any;
    const wrapped = wrapFetch(client, "s1", "p1");
    assert.strictEqual(typeof wrapped, "function");
  });
});
