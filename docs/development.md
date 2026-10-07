# Development and contributing guide

This document covers local setup, build steps, testing suites, and architectural invariants for SAM contributors.

---

## Toolchain requirements

Ensure your machine has the following tools installed:

- **Rust**: 1.96.0 or newer with `cargo` and `rustc`.
- **Node.js**: 22.0.0 or newer (v26 recommended).
- **pnpm**: 10.0.0 or newer (v12 recommended).
- **Platform Webview dependencies** (Linux only): `webkit2gtk-4.1` or `webkit2gtk-4.0`.

---

## Initial setup and build

SAM embeds the compiled frontend into the Tauri binary at compile time. Because of this, the frontend must be built before compiling the desktop shell.

1. Clone the repository and enter the directory:
   ```bash
   git clone https://github.com/<repo>/SAM.git
   cd SAM
   ```

2. Install frontend dependencies and compile the UI bundle:
   ```bash
   pnpm -C ui install
   pnpm -C ui build
   ```

3. Build the Cargo workspace:
   ```bash
   cargo build --workspace
   ```

---

## Development workflows

### Running the desktop application

Launch Tauri with live recompilation enabled:
```bash
cargo tauri dev
```

### Running the frontend with the test runner

For rapid UI iteration without launching the Tauri desktop wrapper:
1. Start the IPC runner against a target plan:
   ```bash
   node tools/web-ipc.mjs --port 4399 --plan /tmp/test-plan
   ```
2. Start the Vite development server in another terminal:
   ```bash
   pnpm -C ui dev
   ```
   Open `http://localhost:5173` in a browser.

### Using the CLI directly

Run the command-line dispatcher during development:
```bash
cargo run -p sam-cli -- --help
cargo run -p sam-cli -- paths --json
```

---

## Verification suites and quality gates

SAM enforces quality and consistency through automated tools:

### 1. Workspace test runner

Runs all unit, integration, and spec parity tests:
```bash
cargo test --workspace
```

Key invariants tested:
- **`no_tauri.rs`**: Guarantees that `crates/sam-core` and `crates/sam-cli` never link Tauri or windowing crates.
- **Strict JSON**: Prohibits duplicate keys, trailing commas, BOMs, and arbitrary type coercions.
- **FSRS-6 parity**: Replays reference state calculations against pinned mathematical outputs.
- **Atomic rollback**: Verifies file integrity across simulated crashes and process terminations.

### 2. Design token consistency

Ensures `design/tokens.css`, `design/tokens.json`, and `design/tailwind.css` remain in agreement:
```bash
node design/tools/tokens.mjs
```

### 3. Design token linter

Scans UI stylesheets across `ui/` to ensure no raw literal colors, arbitrary border radii, or untracked animation durations bypass the token system:
```bash
node tools/tokenlint.mjs
```

### 4. Specification validator

Validates that all section, phase, and appendix cross-references in `SAM_PLAN.md` resolve accurately:
```bash
node tools/plancheck.mjs
```

### 5. Presets consistency

Verifies that bundled presets under `Sources/SAMCore/Resources/presets/` load and match current schema definitions:
```bash
node tools/presets.mjs
```

### 6. Full browser and accessibility gate suite

Drives automated browser sessions over 58 application states in both light and dark modes:
```bash
node tools/gates.mjs
```
The suite verifies:
- Zero unnamed or unfocusable interactive elements.
- Strict WCAG 2.1 AA contrast compliance (minimum 4.5:1 ratio) on every text node.
- Schema census checks ensuring internal technical names never leak into user-facing text.
- Four-surface parity across all declared capabilities.

---

## Architectural rules

When submitting contributions, follow these rules:

1. **Four surfaces per capability**: If you add a persistent mutation or domain feature, it must provide a command ID in `command_registry.rs`, a documented JSON path, a desktop UI control, and CLI dispatch. Never add a capability accessible only via the palette or only via a click.
2. **Bounded mechanism, unbounded content**: Do not hardcode specific subjects, school grades, or proprietary study workflows into the engine. Extend the schema primitives instead.
3. **No Tauri in engine crates**: Keep `crates/sam-core` and `crates/sam-cli` completely free of UI, windowing, and webview dependencies.
4. **Sentence case headings**: Use sentence case for documentation headings (e.g. `## Core concepts` instead of `## Core Concepts`).
