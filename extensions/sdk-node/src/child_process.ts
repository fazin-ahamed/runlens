import { spawn, exec, type ChildProcess, type ExecOptions } from "child_process";
import { runWithSpan, currentSpan, generateId } from "./context.js";
import type { DaemonClient } from "./_client.js";

export function wrapSpawn(client: DaemonClient, sessionId: string, projectId: string) {
  return (command: string, args?: readonly string[], options?: any): ChildProcess => {
    const spanId = generateId();
    const start = Date.now();
    const child = spawn(command, args, options);
    const emitDone = (code: number | null) => {
      client.emit({
        eventId: spanId,
        sessionId: sessionId,
        projectId: projectId,
        sequence: 0,
        source: "sdk",
        kind: "process.spawn",
        severity: code === 0 ? "info" : "warning",
        utcTimestamp: new Date().toISOString(),
        monotonicNs: Date.now() * 1_000_000,
        durationNs: (Date.now() - start) * 1_000_000,
        payload: { command, args, exit_code: code },
        classification: "public",
      }).catch(() => {});
    };
    child.on("exit", emitDone);
    child.on("error", () => emitDone(null));
    return child;
  };
}

export function wrapExec(client: DaemonClient, sessionId: string, projectId: string) {
  return (command: string, options?: ExecOptions, callback?: (...args: any[]) => void): ChildProcess => {
    const spanId = generateId();
    const start = Date.now();
    const child = exec(command, options, (error, stdout, stderr) => {
      client.emit({
        eventId: spanId,
        sessionId: sessionId,
        projectId: projectId,
        sequence: 0,
        source: "sdk",
        kind: "process.exec",
        severity: error ? "error" : "info",
        utcTimestamp: new Date().toISOString(),
        monotonicNs: Date.now() * 1_000_000,
        durationNs: (Date.now() - start) * 1_000_000,
        payload: { command, exit_code: error?.code || 0, stdout_len: stdout.length },
        classification: "public",
      }).catch(() => {});
      if (callback) callback(error, stdout, stderr);
    });
    return child;
  };
}
