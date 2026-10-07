//! # sam-core — the engine (§4.2)
//!
//! Content model, resource resolution, validation, transaction engine, and dispatcher.
//! Designed with no windowing or UI dependencies (D18), serving both the desktop shell and the CLI.

pub mod apply;
pub mod command_registry;
pub mod config_store;
pub mod config_watcher;
pub mod decode;
pub mod diagnostic;
pub mod dispatch;
pub mod doc_edit;
pub mod evaluator;
pub mod expression;
pub mod index;
pub mod json_cursor;
pub mod json_value;
pub mod model;
pub mod pipeline;
pub mod plan_lock;
pub mod profile;
pub mod resolved_config;
pub mod resources;
pub mod rules;
pub mod scheduler;
pub mod schema_guide;
pub mod selfcheck;
pub mod strict_json;
pub mod theme;
pub mod today;
pub mod transaction;
pub mod uicheck;
pub mod validator;
pub mod views;

use std::path::Path;

pub use selfcheck::CheckResult;

/// Runs the shipped selfcheck gate suite against the compiled binary (§6, D22).
pub fn run_selfcheck(binary: &Path) -> Vec<CheckResult> {
    selfcheck::run(binary)
}
