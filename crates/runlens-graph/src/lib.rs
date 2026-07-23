//! Graph modeling for RunLens: event graphs, critical paths, span chains,
//! and before/after graph diffs.

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

pub mod graph;
pub mod critical;
pub mod span;
pub mod diff;