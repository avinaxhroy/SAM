//! `SAM paths --json` answers on a machine with no plan (BUILDLOG 2026-10-06).
//!
//! The registry marks the read `plan_required: false`, it is the command a bug
//! report runs first, and the first run's own boot asks it before a plan root
//! exists — but the resolver refused with `no plan found`, so the one machine
//! that had opened nothing could not read its own paths. Checked here rather
//! than in `sam-core` because the data directory comes from the platform's
//! conventions: `HOME` redirects it on unix, and Windows' known-folder API has
//! no such seam.

#![cfg(unix)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn paths_answers_with_no_plan_on_the_machine() {
    let home = std::env::temp_dir().join(format!("sam-paths-planless-{}", std::process::id()));
    fs::create_dir_all(home.join("Library/Application Support/SAM/plans"))
        .expect("an empty plans directory");
    let resources =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Sources/SAMCore/Resources");

    let output = Command::new(env!("CARGO_BIN_EXE_sam"))
        .args(["paths", "--json"])
        .env("HOME", &home)
        .env_remove("XDG_DATA_HOME")
        .env("SAM_RESOURCES", &resources)
        .output()
        .expect("sam runs");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        output.status.success(),
        "paths refused a machine with no plan: {stdout}"
    );
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("a JSON envelope");
    let data = &parsed["data"];
    assert_eq!(data["plan"], serde_json::Value::Null, "no plan resolves: {data}");
    assert_eq!(data["layer"], serde_json::Value::Null, "no layer resolves: {data}");
    assert_eq!(data["plans"], serde_json::json!([]), "an empty machine lists no plans: {data}");
    assert_eq!(data["index"], serde_json::Value::Null, "no index exists: {data}");
    assert_eq!(data["indexFresh"], serde_json::json!(false), "nothing can be fresh: {data}");
    assert!(
        data["plansDir"].as_str().is_some_and(|dir| dir.ends_with("plans")),
        "the plans directory is still named: {data}"
    );

    fs::remove_dir_all(&home).ok();
}
