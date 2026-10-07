//! Path resolution (§4.1) and bundled resource access (`SAM paths --json`).
//!
//! Resolves design tokens from `design/tokens.json` (§1.4.A), supporting `$value`,
//! `$dark` overlays, and recursive `{path}` alias interpolation.

use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::Serialize;

use crate::json_value::{JSONValue, scalar_text, scalar_text_opt};

pub const APP: &str = "SAM";

/// Environment variable override for the resource directory during development or testing.
/// Package runtime passes resource roots via `paths_with_resources`.
pub const RESOURCES_ENV: &str = "SAM_RESOURCES";

/// A development/test override for the **derived** index directory, the same
/// kind of seam as [`RESOURCES_ENV`]: it points the disposable cache somewhere
/// else so a test never litters the user's data directory. Nothing in a shipped
/// app sets it, and the index is never authoritative (D12), so a wrong value
/// costs a rebuild — never source bytes.
pub const INDEXES_ENV: &str = "SAM_INDEX_DIR";

/// The resolved locations (§4.1). A plan root is `--plan` or the data
/// directory's single plan (§4.1 scope 2); the derived index is Phase 6's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Paths {
    #[serde(rename = "dataDir")]
    pub data_dir: PathBuf,
    #[serde(rename = "plansDir")]
    pub plans_dir: PathBuf,
    #[serde(rename = "indexesDir")]
    pub indexes_dir: PathBuf,
    #[serde(rename = "resourcesDir")]
    pub resources_dir: PathBuf,
}

/// Resolve every location from the platform's own conventions (§4.1):
/// macOS `~/Library/Application Support/SAM/…`, Windows `%APPDATA%\SAM\…`,
/// Linux `$XDG_DATA_HOME/SAM/…`.
pub fn paths() -> Result<Paths, String> {
    let exe =
        env::current_exe().map_err(|error| format!("cannot locate the executable: {error}"))?;
    let override_dir = env::var_os(RESOURCES_ENV).map(PathBuf::from);
    paths_with_resources(resources_dir_for(&exe, override_dir))
}

/// The shell's half of §4.1 (Phase 0B). Tauri resolves the bundle's resource
/// directory — the only part of this that a crate outside the shell cannot know
/// — and hands it here, so `src-tauri` and `sam-cli` return the same `Paths` by
/// construction rather than by review.
pub fn paths_with_resources(resources_dir: PathBuf) -> Result<Paths, String> {
    let project = ProjectDirs::from_path(PathBuf::from(APP))
        .ok_or_else(|| format!("no platform data directory is available for {APP}"))?;
    let data_dir = project.data_dir().to_path_buf();

    let indexes_dir = env::var_os(INDEXES_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("indexes"));
    Ok(Paths {
        plans_dir: data_dir.join("plans"),
        indexes_dir,
        data_dir,
        resources_dir,
    })
}

/// The platform's resource-directory shape, from the executable's path:
///
/// - macOS: `SAM.app/Contents/MacOS/sam` → `SAM.app/Contents/Resources`
/// - Windows: `<install dir>\sam.exe` → `<install dir>\resources`
/// - Linux: `<install dir>/sam` → `<install dir>/resources`
///
/// On macOS this is the same answer Tauri's `resource_dir` gives; the shell
/// will supply it directly in Phase 0B and the CLI keeps resolving it here,
/// because §4.1 requires both doors to agree.
pub fn resources_dir_for(exe: &Path, override_dir: Option<PathBuf>) -> PathBuf {
    if let Some(dir) = override_dir {
        return dir;
    }
    let bin = exe.parent().unwrap_or_else(|| Path::new("."));
    if cfg!(target_os = "macos") {
        // Contents/MacOS/sam → Contents/Resources
        bin.parent()
            .map(|contents| contents.join("Resources"))
            .unwrap_or_else(|| bin.join("resources"))
    } else {
        bin.join("resources")
    }
}

// ── the shell's runtime note ─────────────────────────────────────────────────

/// The file the shell writes one fact into, for a bug report to carry it:
/// which WebKitGTK renderer path this launch took (§9, D21's mitigation). It is
/// shell runtime state, not plan data — outside the plan, invisible to the
/// revision, and disposable.
pub const RUNTIME_FILE: &str = "runtime.json";

/// Record the renderer path the shell applied. Writer: `src-tauri` at startup.
pub fn note_renderer_path(paths: &Paths, label: &str) -> Result<(), String> {
    update_runtime(paths, |document| {
        document.insert("rendererPath".into(), serde_json::json!(label));
    })
}

/// Record the last file the OS handed this app by its association (macOS's
/// open event, or a Windows/Linux double-click's argument) and where it came
/// from — "what actually happens", the one thing S7 refuses to guess. A bug
/// report carries it; `SAM paths --json` prints it.
pub fn note_open(paths: &Paths, path: &str, source: &str) -> Result<(), String> {
    update_runtime(paths, |document| {
        document.insert("lastOpen".into(), serde_json::json!(path));
        document.insert("lastOpenSource".into(), serde_json::json!(source));
        document.insert(
            "lastOpenAt".into(),
            serde_json::json!(crate::transaction::iso_now()),
        );
    })
}

/// The renderer path a running shell reported, if any. Reader: `SAM paths
/// --json` — one command that tells a bug report what the app did without the
/// user being asked to guess.
pub fn renderer_path(paths: &Paths) -> Option<String> {
    runtime_field(paths, "rendererPath")
}

/// The last association launch this machine saw, if any.
pub fn last_open(paths: &Paths) -> Option<JSONValue> {
    let text = std::fs::read_to_string(paths.data_dir.join(RUNTIME_FILE)).ok()?;
    let value: JSONValue = serde_json::from_str(&text).ok()?;
    let path = value.get("lastOpen")?.clone();
    Some(serde_json::json!({
        "path": path,
        "source": value.get("lastOpenSource").cloned(),
        "at": value.get("lastOpenAt").cloned(),
    }))
}

/// Record that the page asked for something: the shell's breadcrumb of the
/// last command it ran, and when the pending association path was taken. Small,
/// private to the machine's data directory, and the difference between "the app
/// did nothing" and "the app was never asked".
pub fn note_page_call(paths: &Paths, key: &str, detail: &str) -> Result<(), String> {
    update_runtime(paths, |document| {
        document.insert(key.to_string(), serde_json::json!(detail));
        document.insert(
            format!("{key}At"),
            serde_json::json!(crate::transaction::iso_now()),
        );
    })
}

fn runtime_field(paths: &Paths, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(paths.data_dir.join(RUNTIME_FILE)).ok()?;
    let value: JSONValue = serde_json::from_str(&text).ok()?;
    value
        .get(key)
        .and_then(JSONValue::as_str)
        .map(str::to_string)
}

/// Read-modify-write the shell's runtime note: one small file, one writer (the
/// shell), merged so a later fact never drops an earlier one.
fn update_runtime(
    paths: &Paths,
    edit: impl FnOnce(&mut serde_json::Map<String, JSONValue>),
) -> Result<(), String> {
    let path = paths.data_dir.join(RUNTIME_FILE);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let mut document = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str::<JSONValue>(&text).ok())
        .and_then(|value| match value {
            JSONValue::Object(object) => Some(object),
            _ => None,
        })
        .unwrap_or_else(|| {
            let mut object = serde_json::Map::new();
            object.insert("schemaVersion".into(), serde_json::json!(1));
            object
        });
    edit(&mut document);
    let text = crate::doc_edit::doc_json::pretty(&JSONValue::Object(document));
    std::fs::write(&path, text).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

// ── the token register ───────────────────────────────────────────────────────

/// One token: a machine-readable twin of `tokens.css` that `--tokens --json`
/// reports and the design doctor reads (§1.4.A, §5). Contrast metadata
/// describes the default palette only and is never the source of the palette.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Token {
    pub path: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// The unresolved value — alias references intact.
    pub raw: JSONValue,
    /// The value after every alias resolves.
    pub value: JSONValue,
    /// The `$extensions.hex` mirror.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hex: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenRegister {
    pub version: String,
    pub tokens: BTreeMap<String, Token>,
}

/// A token before alias resolution: its path travels beside its value.
#[derive(Debug, Clone)]
struct Entry {
    path: String,
    type_: Option<String>,
    value: JSONValue,
    hex: Option<String>,
}

/// One cached bundled register and the file identity it was parsed from.
struct CachedRegister {
    key: (PathBuf, u64, u128),
    register: TokenRegister,
}

impl TokenRegister {
    pub fn get(&self, path: &str) -> Option<&Token> {
        self.tokens.get(path)
    }

    /// Read `<resources>/tokens.json` as data. The bundled file is read-only
    /// for the process's lifetime, so it is parsed once and cached by path,
    /// length and mtime — the CLI (a process per command) pays nothing, the
    /// shell pays one parse.
    pub fn bundled(resources_dir: &Path) -> Result<Self, String> {
        let path = resources_dir.join("tokens.json");
        static CACHE: std::sync::Mutex<Option<CachedRegister>> = std::sync::Mutex::new(None);
        let metadata = std::fs::metadata(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let key = (
            path.clone(),
            metadata.len(),
            metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_nanos())
                .unwrap_or(0),
        );
        let mut cache = CACHE.lock().expect("the token cache lock is not poisoned");
        if let Some(cached) = cache.as_ref()
            && cached.key == key
        {
            return Ok(cached.register.clone());
        }
        let text = std::fs::read(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let register = Self::parse(&text)?;
        *cache = Some(CachedRegister {
            key,
            register: register.clone(),
        });
        Ok(register)
    }

    pub fn parse(data: &[u8]) -> Result<Self, String> {
        let root: JSONValue = serde_json::from_slice(data)
            .map_err(|error| format!("tokens.json is not JSON: {error}"))?;
        let JSONValue::Object(root) = root else {
            return Err("tokens.json is not a JSON object".into());
        };
        let version = match root.get("$version") {
            Some(JSONValue::String(text)) => text.clone(),
            _ => "unknown".into(),
        };

        let mut collected = Vec::new();
        collect(&root, "", None, &mut collected);
        build(version, &collected)
    }

    /// Deep-merge `overrides` over this register: objects descend, arrays and
    /// scalars replace, an override key must address an existing token or
    /// group. Aliases re-resolve after the merge so a reference picks up an
    /// overridden value. `null` is rejected — the schema permits no
    /// null-valued token.
    pub fn merged(&self, overrides: &JSONValue) -> Result<TokenRegister, String> {
        let JSONValue::Object(groups) = overrides else {
            return Err("appearance overrides must be an object".into());
        };
        let mut entries: BTreeMap<String, Entry> = self
            .tokens
            .values()
            .map(|token| {
                (
                    token.path.clone(),
                    Entry {
                        path: token.path.clone(),
                        type_: token.type_.clone(),
                        value: token.raw.clone(),
                        hex: token.hex.clone(),
                    },
                )
            })
            .collect();
        merge_groups(groups, "", &mut entries)?;
        build(
            self.version.clone(),
            &entries.into_values().collect::<Vec<_>>(),
        )
    }
}

/// Resolve aliases over collected entries and sort by path.
fn build(version: String, collected: &[Entry]) -> Result<TokenRegister, String> {
    let by_path: BTreeMap<String, Entry> = collected
        .iter()
        .map(|entry| (entry.path.clone(), entry.clone()))
        .collect();
    let mut tokens = BTreeMap::new();
    for entry in collected {
        let value = resolve_aliases(&entry.value, &entry.path, &by_path, &mut Vec::new())?;
        tokens.insert(
            entry.path.clone(),
            Token {
                path: entry.path.clone(),
                type_: entry.type_.clone(),
                raw: entry.value.clone(),
                value,
                hex: entry.hex.clone(),
            },
        );
    }
    Ok(TokenRegister { version, tokens })
}

fn merge_groups(
    groups: &serde_json::Map<String, JSONValue>,
    prefix: &str,
    entries: &mut BTreeMap<String, Entry>,
) -> Result<(), String> {
    for (key, value) in groups {
        if key.starts_with('$') {
            return Err(format!(
                "override key '{prefix}{key}' addresses register metadata"
            ));
        }
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            JSONValue::Object(children) => {
                if entries.contains_key(&path) {
                    return Err(format!("override '{path}' replaces a token with a group"));
                }
                merge_groups(children, &path, entries)?;
            }
            _ => match entries.get_mut(&path) {
                Some(existing) => {
                    if value.is_null() {
                        return Err(format!(
                            "override '{path}' is null — the token schema permits no null values"
                        ));
                    }
                    existing.value = value.clone();
                }
                None => {
                    return Err(format!(
                        "override '{path}' addresses no existing token or group"
                    ));
                }
            },
        }
    }
    Ok(())
}

/// Descend group objects; a `$value` node is a token leaf.
fn collect(
    obj: &serde_json::Map<String, JSONValue>,
    path: &str,
    inherited_type: Option<String>,
    out: &mut Vec<Entry>,
) {
    let own_type = obj
        .get("$type")
        .and_then(scalar_text_opt)
        .or(inherited_type);
    for (key, node) in obj {
        // `$dark` is an overlay namespace, so it is a path segment like any
        // other; every other `$`-key is register metadata.
        if key.starts_with('$') && key != "$dark" {
            continue;
        }
        let node_path = if path.is_empty() {
            key.clone()
        } else {
            format!("{path}.{key}")
        };
        let JSONValue::Object(child) = node else {
            continue;
        };
        if let Some(value) = child.get("$value") {
            let hex = child
                .get("$extensions")
                .and_then(|extensions| extensions.get("hex"))
                .and_then(JSONValue::as_str)
                .map(str::to_string);
            // A token may declare its own `$type` or inherit the nearest
            // ancestor group's. `tokens.json` does both, so read the leaf first
            // and fall back — a token with no type anywhere reports none.
            let type_ = child
                .get("$type")
                .and_then(scalar_text_opt)
                .or_else(|| own_type.clone());
            out.push(Entry {
                path: node_path,
                type_,
                value: value.clone(),
                hex,
            });
        } else {
            collect(child, &node_path, own_type.clone(), out);
        }
    }
}

/// Substitute every `{path}` in a string value against `entries`, following
/// chains; cycles and unknown references are errors.
fn resolve_aliases(
    value: &JSONValue,
    path: &str,
    entries: &BTreeMap<String, Entry>,
    visiting: &mut Vec<String>,
) -> Result<JSONValue, String> {
    let JSONValue::String(text) = value else {
        return Ok(value.clone());
    };
    let mut result = String::new();
    let mut cursor = 0;
    for (start, end, reference) in references(text) {
        result.push_str(&text[cursor..start]);
        let Some(target) = entries.get(&reference) else {
            return Err(format!(
                "token {path}: reference {{{reference}}} does not exist"
            ));
        };
        if visiting.contains(&reference) {
            return Err(format!(
                "token {path}: alias cycle {} → {reference}",
                visiting.join(" → ")
            ));
        }
        visiting.push(reference.clone());
        let resolved = resolve_aliases(&target.value, &reference, entries, visiting)?;
        visiting.pop();
        let Some(scalar) = scalar_text_opt(&resolved) else {
            return Err(format!(
                "token {path}: reference {{{reference}}} is not a scalar"
            ));
        };
        result.push_str(&scalar);
        cursor = end;
    }
    result.push_str(&text[cursor..]);
    Ok(JSONValue::String(result))
}

/// Byte ranges of whole `{reference}` groups: `{` … the next `}` with no `{`
/// between (§1.4.A permits several in one string).
fn references(text: &str) -> Vec<(usize, usize, String)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'{' {
            index += 1;
            continue;
        }
        let mut scan = index + 1;
        while scan < bytes.len() && bytes[scan] != b'}' && bytes[scan] != b'{' {
            scan += 1;
        }
        if scan < bytes.len() && bytes[scan] == b'}' {
            let reference = text[index + 1..scan].trim().to_string();
            out.push((index, scan + 1, reference));
            index = scan + 1;
        } else {
            index += 1;
        }
    }
    out
}

/// The scalar text of a token's resolved value, for the human table.
pub fn token_text(token: &Token) -> String {
    scalar_text(&token.value)
}

/// Does a resolved value still contain an alias reference? `tokens-load`
/// asserts the register resolves completely, so a leftover brace is a finding.
pub fn contains_reference(value: &JSONValue) -> bool {
    matches!(value, JSONValue::String(text) if !references(text).is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_land_under_the_platform_data_directory() {
        // `index::refresh_tests` points `SAM_INDEX_DIR` at scratch dirs while
        // it runs; this assertion reads that same variable's default, so the
        // two must not interleave.
        let _guard = crate::index::env_test_lock();
        let paths = paths().expect("a data directory resolves on every supported platform");
        assert_eq!(paths.data_dir.file_name().unwrap(), APP);
        assert_eq!(paths.plans_dir, paths.data_dir.join("plans"));
        assert_eq!(paths.indexes_dir, paths.data_dir.join("indexes"));
        assert!(paths.resources_dir.is_absolute());
    }

    #[test]
    fn resources_follow_the_platforms_directory_shape() {
        let exe = Path::new("/Applications/SAM.app/Contents/MacOS/sam");
        let resolved = resources_dir_for(exe, None);
        if cfg!(target_os = "macos") {
            assert_eq!(
                resolved,
                Path::new("/Applications/SAM.app/Contents/Resources")
            );
        }
        assert_eq!(
            resources_dir_for(exe, Some(PathBuf::from("/tmp/r"))),
            Path::new("/tmp/r")
        );
    }

    #[test]
    fn the_shells_resource_directory_replaces_the_executables_answer() {
        let shell = paths_with_resources(PathBuf::from("/Applications/SAM.app/Contents/Resources"))
            .expect("a data directory resolves on every supported platform");
        assert_eq!(
            shell.resources_dir,
            Path::new("/Applications/SAM.app/Contents/Resources")
        );
        assert_eq!(shell.plans_dir, shell.data_dir.join("plans"));
    }

    #[test]
    fn aliases_resolve_and_cycles_are_refused() {
        let register = TokenRegister::parse(
            br#"{"$version":"t",
                 "color":{"ink":{"$type":"color","$value":"oklch(20% 0 0)"},
                          "focus":{"$type":"color","$value":"{color.ink}"}},
                 "shadow":{"$type":"shadow","$value":"0 1px 2px {color.ink}"}}"#,
        )
        .expect("a well-formed register parses");
        assert_eq!(
            register.get("color.focus").unwrap().value,
            JSONValue::String("oklch(20% 0 0)".into())
        );
        assert_eq!(
            register.get("shadow").unwrap().value,
            JSONValue::String("0 1px 2px oklch(20% 0 0)".into()),
            "a reference embedded in a composite string substitutes in place"
        );

        let cyclic = TokenRegister::parse(
            br#"{"$version":"t",
                 "a":{"$type":"color","$value":"{b}"},
                 "b":{"$type":"color","$value":"{a}"}}"#,
        );
        assert!(cyclic.is_err(), "an alias cycle is refused");
    }

    #[test]
    fn overrides_deep_merge_and_re_resolve_aliases() {
        let register = TokenRegister::parse(
            br#"{"$version":"t",
                 "color":{"ink":{"$type":"color","$value":"oklch(20% 0 0)"},
                          "focus":{"$type":"color","$value":"{color.ink}"}}}"#,
        )
        .expect("parses");
        let merged = register
            .merged(&serde_json::json!({"color": {"ink": "#000000"}}))
            .expect("a scalar override replaces");
        assert_eq!(
            merged.get("color.focus").unwrap().value,
            JSONValue::String("#000000".into()),
            "the alias re-resolved onto the overridden value"
        );

        assert!(
            register
                .merged(&serde_json::json!({"color": {"nope": "x"}}))
                .is_err(),
            "an unknown override key is refused"
        );
        assert!(
            register
                .merged(&serde_json::json!({"color": null}))
                .is_err(),
            "a null override is refused"
        );
    }

    #[test]
    fn the_hex_mirror_and_type_are_read_from_the_token_node() {
        let register = TokenRegister::parse(
            br##"{"$version":"4.7","color":{"surface":{"card":{
                 "$type":"color","$value":"oklch(96% 0 0)",
                 "$extensions":{"hex":"#f0f3f7"}}}}}"##,
        )
        .expect("parses");
        let card = register
            .get("color.surface.card")
            .expect("the leaf is a token");
        assert_eq!(card.hex.as_deref(), Some("#f0f3f7"));
        assert_eq!(card.type_.as_deref(), Some("color"));
        assert_eq!(register.tokens.len(), 1, "$-keys are metadata, not tokens");
    }

    #[test]
    fn a_group_type_is_inherited_by_its_tokens() {
        let register = TokenRegister::parse(
            br#"{"$version":"t","space":{"$type":"dimension",
                 "sm":{"$value":"4px"},"md":{"$value":"8px"}}}"#,
        )
        .expect("parses");
        assert_eq!(
            register.get("space.sm").unwrap().type_.as_deref(),
            Some("dimension")
        );
        assert_eq!(
            register.get("space.md").unwrap().type_.as_deref(),
            Some("dimension")
        );
    }
}
