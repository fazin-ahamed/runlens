import { describe, it, mock } from "node:test";
import assert from "node:assert";
import { wrapPg, wrapMysql2 } from "../db.js";

describe("db adapters", () => {
  const mockClient = { emit: mock.fn((_: any) => Promise.resolve()) };

  it("wrapPg returns wrapped client with query method", () => {
    const client = { query: mock.fn() };
    const wrapped = wrapPg(client, "sid", "pid", { daemonClient: mockClient as any });
    assert.ok(wrapped);
    assert.strictEqual(typeof wrapped.query, "function");
  });

  it("wrapPg wrapped query calls original", async () => {
    let called = false;
    const client = { query: mock.fn((_text: string) => { called = true; }) };
    const wrapped = wrapPg(client, "sid", "pid", { daemonClient: mockClient as any });
    await wrapped.query("SELECT 1");
    assert.ok(called);
  });

  it("wrapMysql2 returns wrapped pool with execute method", () => {
    const pool = { execute: mock.fn() };
    const wrapped = wrapMysql2(pool, "sid", "pid", { daemonClient: mockClient as any });
    assert.ok(wrapped);
    assert.strictEqual(typeof wrapped.execute, "function");
  });

  it("wrapMysql2 wrapped execute calls original", async () => {
    let called = false;
    const pool = { execute: mock.fn((_sql: string) => { called = true; return Promise.resolve([[], []]); }) };
    const wrapped = wrapMysql2(pool, "sid", "pid", { daemonClient: mockClient as any });
    await wrapped.execute("SELECT 1");
    assert.ok(called);
  });
});
