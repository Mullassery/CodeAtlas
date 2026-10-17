//! CodeAtlas core: deterministic codebase architecture analysis.
//!
//! This crate is pure Rust with no CPython, network, or LLM dependencies so the
//! analysis pipeline can be tested without API keys or an internet connection.
//!
//! See `docs/architecture/ARCHITECTURE.md` for the full module breakdown.

pub mod analysis;
pub mod clustering;
pub mod discovery;
pub mod extract;
pub mod fingerprint;
pub mod model;
pub mod pipeline;

#[cfg(test)]
mod testutil;

/// The engine version string, reported by the Python binding.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
