//! Agent integration — AI agent coding hooks and intercepts.
//!
//! This module provides the machinery that forces AI agents through apmw's
//! install-on-use hook for adding dependencies and running tools. It
//! implements both a **hard intercept** (PATH shims that wrap package manager
//! binaries) and a **soft convention** (instructions for agents to use
//! `apmw add` instead of calling package managers directly).
//!
//! - **Hard intercept** is opt-in via `apmw --install --intercept`.
//! - **Soft convention** is the default (no intercept installed).
//!
//! Hooks are governance-aware: intercepted calls respect the active governance
//! policy (`prefer`/`force`/`block`/`eject`) from the governance engine.

pub mod hooks;

pub use hooks::{
  default_shim_dir, generate_soft_convention_instructions, HookManager, InterceptAction,
  InterceptDecision, InterceptResult, ShimSpec, SHIM_MARKER, SHIM_TARGETS,
};
