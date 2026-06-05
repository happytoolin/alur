//! alur - fast package-manager routing with short commands and an opt-in Node shim.
//!
//! This crate powers the `alur` CLI, including package-manager detection, fast
//! script/local-bin execution, multicall command aliases, and the optional `node`
//! shim.

pub mod app;
pub mod core;
pub mod features;
pub mod platform;
