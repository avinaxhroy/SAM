# SAM — Configurable Study OS

Product codebase, not a study-plan advising context. Tauri desktop app: a Rust engine
behind a Svelte frontend, built on the usual JS bundler and utility-first CSS toolchain.
Targets macOS, Windows and Linux equally, and only those — no platform-native toolchain
(Xcode, Swift, Dart, JVM).

The workspace splits into: the core engine crate (framework-free — never link the
desktop shell here) · the CLI crate · the desktop shell (the only crate that may depend
on the app framework) · the web UI (not a cargo member) · the dev/gate tooling · a
superseded legacy engine kept purely as a parity reference, never ported from. An
integration test enforces the engine/shell split. Keep it passing.

## Rules

1. Four surfaces per capability: **click, name, text, terminal**. Persistent edits get a
   command id, a documented JSON location, and CLI dispatch. Nothing palette-only.
2. Greenfield. Build from the reference design. No compat shim, no legacy-state
   migration, no pixel-identity gate.
3. Bounded mechanism, unbounded content. The vocabulary is small and fixed; user content
   goes inside it. Don't grow the mechanism.
4. The workspace test run is the runner. The standalone self-checks stay because they
   gate a packaged binary where no harness exists.

## Finding code — search first

Reach for code search before `grep`/`glob`/`read`:

```js
// in `execute` (Code Mode). repo defaults to the workspace; don't hardcode a path.
await tools.semble.search({ repo: ".", query: "<what it does, not what it's called>", top_k: 5 })
await tools.semble.find_related({ repo: ".", file_path: "<from search>", line: <start_line> })
```

Read the returned file at that line — don't re-grep what the search already gave you.
Use `grep` only when you need every literal occurrence (all callers of a symbol, a
string appearing in docs and code alike); scope prose and Markdown searches to
`content: "docs"`. CLI fallback: `semble search "<query>" .`

## Specs are binding

The plan, the composer notes, the UI/UX specs, the design docs and the build log. Don't
re-derive intent from code. When code and spec disagree the spec wins and the code is
the bug — or the spec is stale, which is a finding to report, not to code around.
