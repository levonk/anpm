//! apmw-core — reusable package manager detection, ecosystem mapping, and
//! version resolution.
//!
//! This crate provides the core detection, ecosystem mapping, and version
//! resolution logic extracted from the `apmw` binary. It is designed to be
//! reusable by downstream projects that need package-manager detection without
//! the full CLI/daemon surface.

pub mod custom_types;
pub mod detect;
pub mod ecosystem;
pub mod error;
pub mod version;
pub mod version_info;
pub mod workspace;

pub use error::{CoreError, Result};
