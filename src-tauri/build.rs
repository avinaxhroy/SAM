//! Tauri build script. Embeds `ui/dist/` into the shell binary at compile time.

fn main() {
    tauri_build::build()
}
