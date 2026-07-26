# runlens-mcp

Model Context Protocol server exposing RunLens recordings as read-only tools for LLM agents and editors.

## Transports

| Mode | Usage |
|------|-------|
| **stdio** | Claude Code / Continue.dev direct integration |
| **HTTP** | Loopback-only for browser tools and manual probes |

## Exposed Tools

All tools are read-only — they walk stored sessions and return structured information. No tool deletes data or invokes the recorder.
