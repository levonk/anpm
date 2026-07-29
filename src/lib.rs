//! apmw — All Package Manager Wrapper
//!
//! Abstracts every package installer into one intelligent surface.
//! Detects the correct package manager, installs tools with
//! install-on-use semantics, and runs security scanning before install.

pub mod error;

pub use error::{ApmwError, Result};

/// Returns the version of the apmw library.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
