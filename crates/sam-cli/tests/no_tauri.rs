//! Verifies absence of windowing frameworks in `sam-cli` dependency graph (D18).
//!
//! Enforces engine/presentation separation by checking `cargo tree` resolution.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Names that mean "a windowing framework", not only `tauri` itself: Phase 0B
/// links `tauri`, `tauri-build`, `tauri-runtime*` and friends into the shell.
fn is_windowing_framework(crate_name: &str) -> bool {
    crate_name == "tauri" || crate_name.starts_with("tauri-")
}

#[test]
fn sam_cli_links_no_windowing_framework() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/sam-cli sits two levels under the workspace root")
        .to_path_buf();

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = Command::new(cargo)
        .current_dir(&workspace)
        .args([
            "tree",
            "--package",
            "sam-cli",
            "--edges",
            "normal,build",
            "--prefix",
            "none",
            "--locked",
        ])
        .output()
        .expect("cargo tree runs wherever the tests run");

    assert!(
        output.status.success(),
        "cargo tree failed in {}: {}",
        workspace.display(),
        String::from_utf8_lossy(&output.stderr)
    );

    let graph = String::from_utf8_lossy(&output.stdout);
    let offenders: Vec<&str> = graph
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| is_windowing_framework(name))
        .collect();

    assert!(
        offenders.is_empty(),
        "sam-cli must not link a windowing framework (D18): {offenders:?}\n\
         The engine and the CLI are consumers of sam-core; only src-tauri links tauri."
    );
}

/// The same rule, stated for the workspace as a whole, so that a windowing
/// framework slipping into `sam-core` fails too — the CLI's graph would then
/// contain it transitively and the test above would already fail, but the
/// message would point at the wrong crate.
#[test]
fn sam_core_links_no_windowing_framework() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/sam-cli sits two levels under the workspace root")
        .to_path_buf();

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = Command::new(cargo)
        .current_dir(&workspace)
        .args([
            "tree",
            "--package",
            "sam-core",
            "--edges",
            "normal,build",
            "--prefix",
            "none",
            "--locked",
        ])
        .output()
        .expect("cargo tree runs wherever the tests run");

    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let graph = String::from_utf8_lossy(&output.stdout);
    let offenders: Vec<&str> = graph
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| is_windowing_framework(name))
        .collect();

    assert!(
        offenders.is_empty(),
        "sam-core must compile with no windowing framework (§4.5): {offenders:?}"
    );
}
