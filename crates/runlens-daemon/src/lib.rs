#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

#![allow(
    clippy::doc_markdown,
    clippy::cast_lossless,
)]

pub mod discovery;
pub mod ipc;
pub mod pipeline;
pub mod state;
pub mod subscription;
pub mod ws;
