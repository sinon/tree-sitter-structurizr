#![warn(missing_docs)]
//! Thin, testable LSP server scaffolding for Structurizr DSL editor features.

pub mod capabilities;
pub mod convert;
pub mod documents;
pub mod handlers;
pub mod server;
pub mod state;

pub use server::Backend;
