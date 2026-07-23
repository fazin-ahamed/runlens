#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]
#![allow(
    clippy::doc_markdown,
    clippy::str_to_string,
    clippy::single_char_lifetime_names,
)]

pub mod ast;
pub mod error;
pub mod executor;
pub mod lexer;
pub mod parser;

pub use ast::Query;
pub use error::RqlError;
pub use executor::{execute, explain};
pub use parser::parse;

pub fn run_query(conn: &rusqlite::Connection, rql: &str) -> Result<Vec<serde_json::Value>, RqlError> {
    let query = parse(rql)?;
    executor::execute(conn, &query)
}

pub fn run_explain(conn: &rusqlite::Connection, rql: &str) -> Result<Vec<serde_json::Value>, RqlError> {
    let query = parse(rql)?;
    executor::explain(conn, &query)
}
