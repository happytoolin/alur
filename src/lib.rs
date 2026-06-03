//! HNI - ni-compatible package manager command router with node shim.
//!
//! This crate provides a unified interface for working with multiple package managers
//! (npm, yarn, pnpm, bun) using ni-compatible commands.

pub mod app;
pub mod core;
pub mod features;
pub mod platform;
