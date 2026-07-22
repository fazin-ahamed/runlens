# threat model

runlens records stuff. its job is to make a faithful, verifiable,
export-safe trace of what happened during a command without becoming
a side-channel for secrets.

## adversaries we consider

| adversary | capability |
|-----------|------------|
| **local user** | owns the machine, may share it with others |
| **bundle consumer** | receives a .runlens file via slack/paste/git |
| **AI assistant** | has tool access, reads redacted strings |
| **rogue dependency** | writes to stdout or env that we record |
| **machine attacker** | reads files in our repo post-mortem |

we do not defend against someone with kernel-level access during
recording. if an attacker can run arbitrary code while the recorder
is alive, the recorder is the least of your problems.

## guarantees we make

| property | how |
|----------|-----|
| events cant be modified without detection | BLAKE3 chain links each event to its predecessor. verify_chain regenerates every hash. |
| bundle events cant be modified without detection | manifest carries invariants.head_hash. import re-verifies the chain. |
| bundle entries cant escape disk | is_path_unsafe rejects .., absolute drives, null bytes before hash check. |
| bundle size mismatch detected | header-declared size vs actual bytes-read. mismatch aborts. |
| bundle version must be known | COMPATIBLE_VERSIONS accepts runlens.bundle@1.x.y and rejects everything else. |
| secrets never recorded in clear | default redaction runs before sealing. |
| secrets dont leak via paths | mask_absolute_path collapses to ~. |
| secrets dont leak via env | RUNLENS_* is the only auto-recorded prefix. everything else is allow-listed or marked Excluded (value dropped). |
| AI tools see read-only data | MCP surface is list, get, find, compare, verify. no writes. |
| MCP binds locally | 127.0.0.1 only. never reachable from outside the host. |

## properties we do not claim

- **confidentiality at rest.** SQLite isnt encrypted. if your
  filesystem is compromised, the redacted payloads are still visible.
- **forward-integrity during a crash.** SQLite WAL gives
  crash-recovery for committed transactions. an event mid-flight at
  SIGKILL may be lost.
- **tamper-resistance against someone who controls the writer.** a
  malicious script using our mutating APIs can produce a chain that
  verifies but contains false data. we are not a trust anchor.

## bundles

.runlens bundles are the trust boundary between the recorder and
offline consumers. on import we:

1. open the file as a gzipped tar
2. reject any entry with path traversal or absolute drive before
   reading content
3. compare header-declared size to actual bytes-read
4. decode the manifest, reject if bundle_format_version is not in
   COMPATIBLE_VERSIONS
5. decode every event
6. re-run chain::verify_chain on the decoded events
7. only then persist events to SQLite

if any of those steps fails, import aborts and the source bundle
stays untouched on disk.

## MCP

the MCP server is read-only by construction:

- tools only call into Repository for list/get/verify/compare. no
  write path exists.
- axum server binds to 127.0.0.1 only. bind failure = abort.
- stdio transport does no file writes. only emits JSON lines on
  stdout.

undefined tool returns `method not found` (-32601). bad params
returns `invalid params` (-32602).

## keeping this current

any change to redaction patterns, default env allow-list, or bundle
version compatibility is security-relevant. add a PR link to the
audit log below.

### audit log

| date | change | PR |
|------|--------|----|
| 2026-07-17 | initial threat model with allow-list and redaction defaults | bootstrap |
