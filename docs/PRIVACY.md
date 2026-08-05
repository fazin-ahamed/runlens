# privacy

runlens is local. it never uploads anything. unlike a telemetry tool, it
is meant to keep recorded data on your machine.

## redaction happens by default

every event payload goes through the redaction pipeline before the
BLAKE3 chain seals it and before it touches sqlite. order matters.

we scan string values with these patterns:

| what | example |
|------|---------|
| AWS key | `AKIA...` |
| GitHub personal token | `ghp_...` |
| Slack token | `xox[bp]-...` |
| Stripe key | `sk_...` |
| Google API key | `AIza...` |
| JWT-like strings | 3-segment base64 strings |
| Auth headers | `Authorization: ...` |
| Private key blocks | PEM BEGIN/END blocks |
| DB URLs | `postgres://user:pass@...` |
| Email addresses | user@domain |
| Private IPs | `10.x.x.x`, `192.168.x.x` |
| Home paths | `/home/you/...` to `~/...` |
| High-entropy strings | base64-looking blobs >= 40 chars |

findings are reported with byte offsets into the original input.
detection and mutation are separate. you can review what got caught.

**the on-disk payload is the redacted one.** the sensitive form is
not written on the matched path.

redaction is best-effort regex detection, not a guarantee of secrecy.
a secret that matches no pattern (a novel token format, a password in
a path attribute, a value split across two payloads) can pass. treat the
redactor as a safety net, not a confidentiality boundary, and do not rely
on it to protect credentials you must never leak.

## path masking

`mask_absolute_path` collapses `/home/...`, `/Users/...`,
`C:\Users\...` to `~`. prevents leaking the username.

## env fingerprint

we take a caller-supplied env map. we do not call `std::env::vars()`
ourselves. then we walk the map through an allow-list:

- PATH, OS, OSTYPE, MSYSTEM, PROCESSOR_ARCHITECTURE
- HOME, HOMEDRIVE, USERPROFILE, TMPDIR, TMP, TEMP
- LANG, LC_ALL, TZ
- toolchain vars (JAVA_HOME, PYTHON_VERSION, NODE_VERSION, etc)
- CI vars (GITHUB_ACTIONS, CI, etc)
- any `RUNLENS_*` key

everything else is marked `Excluded` with the value never serialized.
we store:
- BLAKE3 hash of the value
- a short preview (only if safe to show)
- category: os/lang/ci/custom/excluded

this lets you detect drift (my rust version changed between runs)
without leaking the actual value.

## failure signature normalization

`FailureSignature` strips UUIDs, PIDs, file:line refs, absolute paths,
IPs, ISO timestamps. if two failures have the same signature after
normalization, theyre probably the same bug.

we never claim root cause. `compare_sessions` says "two sessions
diverged in event-kind X", not "session B crashed because of A".

## reviewing findings

`runlens redactions <session_id>` lists every finding. findings are
tagged with `reviewed` in schema. interactive review command coming
later.

## threat model

see the [threat model](THREAT-MODEL.md).
