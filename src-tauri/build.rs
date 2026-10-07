//! Tauri build script. Embeds `ui/dist/` into the shell binary at compile time.

fn main() {
    // Ensure sidecar placeholder exists so tauri_build does not fail when checking externalBin
    let target_triple = std::env::var("TARGET").unwrap_or_default();
    if !target_triple.is_empty() {
        let binaries_dir = std::path::Path::new("binaries");
        let _ = std::fs::create_dir_all(binaries_dir);
        let ext = if target_triple.contains("windows") { ".exe" } else { "" };
        let sidecar_name = format!("sam-{target_triple}{ext}");
        let sidecar_path = binaries_dir.join(sidecar_name);
        if !sidecar_path.exists() {
            let _ = std::fs::write(&sidecar_path, b"");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&sidecar_path, std::fs::Permissions::from_mode(0o755));
            }
        }
    }

    tauri_build::build()
}
