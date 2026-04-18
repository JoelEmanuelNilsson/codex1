//! Codex1 V2 — skills-first harness backed by a small CLI contract kernel.
//!
//! Public surface is intentionally tight: only `cli::{Cli, run}` are
//! re-exported. All other modules are module-scoped or `pub(crate)`.
//! Keeping the surface narrow is a direct reaction to V1's `codex1-core`
//! exporting ~70 types and becoming a god-library.

pub(crate) mod envelope;
pub(crate) mod error;

pub(crate) mod fs_atomic;
pub(crate) mod events;
pub(crate) mod state;

pub(crate) mod mission;
pub(crate) mod blueprint;
pub(crate) mod graph;
pub(crate) mod proof;
pub(crate) mod binding;
pub(crate) mod replan;
pub(crate) mod review;
pub(crate) mod mission_close;
pub(crate) mod advisor;
pub(crate) mod status;

pub mod cli;

pub use cli::{run, Cli};
