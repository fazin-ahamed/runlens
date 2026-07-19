# contributing

## getting started

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```

## code style

- run `cargo fmt` before pushing. settings in rustfmt.toml.
- clippy lints are in root Cargo.toml. dont fight them.
- `#![forbid(unsafe_code)]` on every crate. if you need unsafe, document why.
- `thiserror` for public errors, `anyhow` for internal propagation.
- dont re-export internal types unless a sibling crate needs them.

## pr checklist

1. `cargo test --workspace` — all pass
2. `cargo clippy --workspace --all-targets` — no warnings
3. `cargo fmt --check` — no formatting diffs
4. add a changelog entry in the relevant crate's CHANGELOG.md
5. open a PR against main

## layout

crate layout is in README.md. architecture is in docs/ARCHITECTURE.md.

## license

mit or apache-2.0, your pick.
