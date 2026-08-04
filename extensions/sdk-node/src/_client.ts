// Generated — do not edit
import WebSocket from "ws";
import { EventV2, DAEMON_URL } from "./types.js";

export class DaemonClient {
  private ws: WebSocket | null = null;
  private pending = new Map<string, { resolve: (v: any) => void; reject: (e: any) => void }>();
  private id = 0;

  constructor(private url: string = DAEMON_URL) {}

  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.url);
      let settled = false;
      this.ws.on("open", () => { settled = true; resolve(); });
      this.ws.on("message", (data) => this.handleMessage(data));
      this.ws.on("error", (err) => { if (!settled) reject(err); });
      this.ws.on("close", () => { if (!settled) reject(new Error("Connection closed")); });
    });
  }

  async call(method: string, params?: any): Promise<any> {
    if (!this.ws) throw new Error("Not connected");
    const id = ++this.id;
    return new Promise((resolve, reject) => {
      this.pending.set(String(id), { resolve, reject });
      this.ws!.send(JSON.stringify({ jsonrpc: "2.0", id, method, params }));
    });
  }

  async emit(event: EventV2): Promise<void> {
    if (!this.ws) throw new Error("Not connected");
    this.ws.send(JSON.stringify({ jsonrpc: "2.0", method: "event.emit", params: { event } }));
  }

  async emitBatch(events: EventV2[]): Promise<void> {
    if (!this.ws) throw new Error("Not connected");
    this.ws.send(JSON.stringify({ jsonrpc: "2.0", method: "event.emit_batch", params: { events } }));
  }

  disconnect() {
    this.ws?.close();
    this.ws = null;
  }

  private handleMessage(data: WebSocket.Data) {
    let msg: any;
    try { msg = JSON.parse(data.toString()); } catch { return; }
    if (msg.id != null) {
      const p = this.pending.get(String(msg.id));
      if (p) {
        this.pending.delete(String(msg.id));
        msg.error ? p.reject(msg.error) : p.resolve(msg.result);
      }
    }
  }
}