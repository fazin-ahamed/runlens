//! RunLens core: pure domain logic, canonical serialization, hash chain,
//! failure signatures, privacy, run comparison.
//!
//! This crate has no I/O (no DB, no fs, no network). All adapters live in
//! `runlens-storage`, `runlens-recorder`, `runlens-bundle`, etc.

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

// Style allowances for pre-existing code patterns
#![allow(
    clippy::doc_markdown,
    clippy::str_to_string,
    clippy::missing_const_for_fn,
    clippy::let_and_return,
    clippy::too_many_arguments,
    clippy::items_after_test_module,
    clippy::cast_lossless,
    clippy::unusual_byte_groupings,
    clippy::single_char_add_str,
    clippy::option_if_let_else,
    clippy::manual_pattern_char_comparison,
    clippy::explicit_counter_loop,
    clippy::op_ref,
    clippy::needless_borrow,
    clippy::unnecessary_sort_by,
    clippy::needless_borrows_for_generic_args,
    clippy::if_same_then_else,
)]

pub mod canonical;

pub mod chain;

pub mod compare;

pub mod event_v2;

pub mod identifier;

pub mod model;

pub mod privacy;

pub mod protocol;

pub mod signatures;
