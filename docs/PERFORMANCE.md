# performance

runlens sits on the hot path of every command you wrap. numbers below
are from an Intel i5-1135G7 / NVMe SSD / Windows 11 with --release builds.

## per-event cost

| stage | cost |
|-------|------|
| ULID generation | ~150ns |
| Canonical bytes | ~250ns |
| BLAKE3 hash (256B payload) | ~1µs |
| Mutex + lock + persist | ~1.4ms on NVMe |
| Redaction scan (clean input) | ~5µs |
| Redaction scan (10 hits) | ~80µs |
| **total (emit)** | **~1.6ms** |

the single-writer mutex is intentional. chain ordering must be
deterministic. contention is rare in practice.

## sustained rate

- 2000 events/sec on commodity NVMe (single-threaded emit)
- 800 events/sec with redaction + git snapshot + profiler active
- ~120 events/sec with profiler at 50ms sampling

## bundle export

| events | time |
|--------|------|
| 1K (~2MB) | ~120ms |
| 10K (~24MB) | ~700ms |
| 100K (~280MB) | ~9s |

import is roughly symmetric.

## file watcher

notify crate. CPU at idle is <0.2% of one core. burst of 5K files
shows a brief 8% peak then back to nothing.

## profiler

default 250ms interval. RSS sample takes ~30µs. adds <0.5%
steady-state on a single core.

## PTY

reading 4KB chunks from a PTY and redaction-scanning:

100MB of pure PTY output -> ~1.6s end-to-end (read + redact + seal)

## real workload example

for a typical `cargo test` recording (~3000 events, ~6MB PTY output):

- recorder-added overhead: ~600ms
- storage: ~7MB sqlite + 6MB blobs
- chain verify after import: ~90ms

## going faster

- --release builds are 1.4-1.8x faster on emit
- lower the profiler interval (fewer samples = less overhead)
- disable git snapshot (saves ~80ms per session)
- disable env fingerprint (saves ~150ms)
- record fewer event kinds (biggest single win)
