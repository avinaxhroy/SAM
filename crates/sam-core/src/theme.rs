//! Themes and design linting (§6 Phase 7, §5, §1.4.A).
//!
//! Themes in `themes/<id>.json` define token overrides matching the path syntax of
//! `content/appearance.json#/overrides`.
//!
//! Resolution precedence:
//! 1. Bundled register (`design/tokens.json`)
//! 2. Register `$dark` overlay (when dark mode is active)
//! 3. Theme document `tokens`
//! 4. Plan `appearance.json#/overrides`
//! 5. `textScale` applied to `fontSize` tokens
//!
//! `$dark` operates as a CSS-variable namespace (e.g. `color.$dark.mint.ink` maps to `--fg-mint`).
//! Keys with no matching token trigger errors to catch stale token names.
//!
//! Contrast is recomputed dynamically from resolved colors (§1.4.A) via Oklch -> linear sRGB ->
//! WCAG 2.x relative luminance. Unparseable pairs report as uncomputed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::json_value::{JSONValue, scalar_text};
use crate::resolved_config::ResolvedConfig;
use crate::resources::TokenRegister;
use crate::rules::LINT_RULES;

/// The four shipped themes (§6 Phase 7's artifacts) live beside the register.
pub const DEFAULT_THEME: &str = "cadence-light";

/// The modes a theme may declare. `auto` follows the caller's preference (the
/// shell passes the OS/stored mode; a headless caller passes `light`).
pub const THEME_MODES: [&str; 3] = ["light", "dark", "auto"];

/// The text-size preference's allowed range (§6 Phase 7's "text size").
pub const TEXT_SCALE_RANGE: (f64, f64) = (0.8, 2.0);

/// One theme document: `themes/<id>.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeDoc {
    #[serde(
        rename = "schemaVersion",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub schema_version: Option<u64>,
    pub id: String,
    pub name: String,
    /// `light` | `dark` | `auto` — [`THEME_MODES`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Token overrides in token-path space — the shape
    /// [`TokenRegister::merged`] accepts and `appearance.json` already uses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens: Option<JSONValue>,
}

impl ThemeDoc {
    pub fn mode(&self) -> &str {
        self.mode.as_deref().unwrap_or("auto")
    }

    /// The mode a caller's preference resolves to for this theme.
    pub fn resolved_mode(&self, user_mode: &str) -> String {
        match self.mode() {
            "dark" => "dark".into(),
            "light" => "light".into(),
            _ => {
                if user_mode == "dark" {
                    "dark".into()
                } else {
                    "light".into()
                }
            }
        }
    }
}

pub fn themes_dir(resources_dir: &Path) -> PathBuf {
    resources_dir.join("themes")
}

/// Every theme in the resources directory, sorted by id. A malformed theme is
/// an error, never skipped: a theme that silently does not appear is a
/// settings screen that lies about what exists.
pub fn list(resources_dir: &Path) -> Result<Vec<ThemeDoc>, String> {
    let dir = themes_dir(resources_dir);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("cannot read {}: {error}", dir.display())),
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().map(|ext| ext == "json").unwrap_or(false))
        .collect();
    paths.sort();
    let mut themes = Vec::new();
    for path in paths {
        themes.push(load_file(&path)?);
    }
    Ok(themes)
}

/// One theme by id, `None` when no document carries that id.
pub fn load(resources_dir: &Path, id: &str) -> Result<Option<ThemeDoc>, String> {
    Ok(list(resources_dir)?
        .into_iter()
        .find(|theme| theme.id == id))
}

fn load_file(path: &Path) -> Result<ThemeDoc, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let doc: ThemeDoc = serde_json::from_str(&text)
        .map_err(|error| format!("{} is not a theme document: {error}", path.display()))?;
    if doc.id.is_empty() {
        return Err(format!("{} declares no id", path.display()));
    }
    if let Some(mode) = &doc.mode
        && !THEME_MODES.contains(&mode.as_str())
    {
        return Err(format!(
            "{} declares mode \"{mode}\" — one of {}",
            path.display(),
            THEME_MODES.join(", ")
        ));
    }
    Ok(doc)
}

// ── token path ⇄ CSS custom property ─────────────────────────────────────────

/// The design system's own mapping from a token path to the CSS custom
/// property the stylesheets consume (`design/tools/tokens.mjs`'s `mapFor`,
/// inverted). `None` for a path with no custom property — including the
/// `$dark` overlay namespace, which is a set of names, not paths.
pub fn css_name_for(path: &str) -> Option<String> {
    const SURFACE: [(&str, &str); 5] = [
        ("backdrop", "color.surface.backdrop"),
        ("sheet", "color.surface.sheet"),
        ("well", "color.surface.well"),
        ("well-2", "color.surface.well-2"),
        ("card", "color.surface.card"),
    ];
    const INK: [(&str, &str); 7] = [
        ("ink", "color.ink-level.ink"),
        ("ink-2", "color.ink-level.ink-2"),
        ("ink-3", "color.ink-level.ink-3"),
        ("ink-4", "color.ink-level.ink-4"),
        ("ink-inv", "color.ink-level.ink-inv"),
        ("rule", "color.ink-level.rule"),
        ("rule-strong", "color.ink-level.rule-strong"),
    ];
    const MATERIAL: [(&str, &str); 4] = [
        ("pane", "color.material.pane"),
        ("fill-on-ink", "color.material.fill-on-ink"),
        ("fill-on-ink-hi", "color.material.fill-on-ink-hi"),
        ("scrim", "color.material.scrim"),
    ];
    const LAYOUT: [(&str, &str); 13] = [
        ("--titlebar-h", "layout.titlebarHeight"),
        ("--titlebar-inset-mac", "layout.titlebarInset.mac"),
        ("--titlebar-inset-win", "layout.titlebarInset.win"),
        ("--pill-h", "layout.pillHeight"),
        ("--pill-h-lg", "layout.pillHeightLg"),
        ("--chip-h", "layout.chipHeight"),
        ("--gutter", "layout.windowGutter"),
        ("--rail-w", "layout.railWidth"),
        ("--rail-label", "layout.railLabel"),
        ("--disc", "layout.discSize"),
        ("--hit", "layout.hitFloor"),
        ("--canvas-max", "layout.canvasMax"),
        ("--prose-measure", "layout.proseMeasure"),
    ];

    for (name, candidate) in LAYOUT {
        if path == candidate {
            return Some(name.to_string());
        }
    }
    for (name, candidate) in SURFACE {
        if path == candidate {
            return Some(format!("--{name}"));
        }
    }
    for (name, candidate) in INK {
        if path == candidate {
            return Some(format!("--{name}"));
        }
    }
    for (name, candidate) in MATERIAL {
        if path == candidate {
            return Some(format!("--{name}"));
        }
    }
    if path == "color.focus" {
        return Some("--focus".into());
    }
    // `room-lit` and the five glass rungs are `color.material.*` paths whose
    // custom property shares the tail of the path.
    if let Some(rest) = path.strip_prefix("color.material.")
        && (rest == "room-lit" || rest == "glass" || rest.starts_with("glass-"))
    {
        return Some(format!("--{rest}"));
    }
    if let Some(rest) = path.strip_prefix("color.identity.neutral.") {
        return Some(format!("--wash-{rest}"));
    }
    if let Some(rest) = path.strip_prefix("color.identity.") {
        let (name, kind) = rest.split_once('.')?;
        return match kind {
            "surface" => Some(format!("--w-{name}")),
            "ink" => Some(format!("--fg-{name}")),
            _ => None,
        };
    }
    if let Some(rest) = path.strip_prefix("color.state.") {
        let (name, kind) = rest.split_once('.')?;
        return match kind {
            "surface" => Some(format!("--chip-{name}")),
            "ink" => Some(format!("--on-{name}")),
            _ => None,
        };
    }
    if let Some(rest) = path.strip_prefix("fontSize.") {
        return Some(format!("--text-{rest}"));
    }
    if let Some(rest) = path.strip_prefix("tracking.") {
        return Some(format!("--track-{rest}"));
    }
    if let Some(rest) = path.strip_prefix("space.") {
        return Some(format!("--space-{rest}"));
    }
    if let Some(rest) = path.strip_prefix("radius.") {
        return Some(format!("--r-{rest}"));
    }
    if path == "shadow.catch" {
        return Some("--catch".into());
    }
    if path == "shadow.rim-glass" {
        return Some("--rim-glass".into());
    }
    if let Some(rest) = path.strip_prefix("shadow.") {
        return Some(format!("--sh-{rest}"));
    }
    if let Some(rest) = path.strip_prefix("motion.duration.") {
        return Some(format!("--dur-{rest}"));
    }
    if path == "motion.easing.out" {
        return Some("--ease".into());
    }
    if path == "motion.easing.pop" {
        return Some("--ease-pop".into());
    }
    if let Some(rest) = path.strip_prefix("font.weight-") {
        return Some(format!("--weight-{rest}"));
    }
    if path == "font.display" {
        return Some("--font-display".into());
    }
    if path == "font.body" {
        return Some("--font-body".into());
    }
    None
}

/// A `color.$dark.<key>` key as the custom property it overrides:
/// `mint.surface` → `--w-mint`, `mint.ink` → `--fg-mint`, `wash-rose` →
/// `--wash-rose`, `weight-body` → `--weight-body`.
fn dark_css_name(key: &str) -> String {
    if let Some((name, kind)) = key.split_once('.') {
        match kind {
            "surface" => return format!("--w-{name}"),
            "ink" => return format!("--fg-{name}"),
            _ => {}
        }
    }
    format!("--{key}")
}

/// The register's resolved values keyed by custom property.
pub fn css_values(register: &TokenRegister) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (path, token) in &register.tokens {
        if let Some(name) = css_name_for(path) {
            out.insert(name, scalar_text(&token.value));
        }
    }
    out
}

// ── the `$dark` overlay ──────────────────────────────────────────────────────

/// Apply `design/tokens.json`'s own `$dark` group over a register: every
/// `color.$dark.<key>` and `shadow.$dark.<rest>` leaf replaces the token its
/// custom property names. A key that maps to no token is an error — the design
/// system changed shape, and a dropped dark value would be invisible.
pub fn apply_dark_overlay(register: &TokenRegister) -> Result<TokenRegister, String> {
    let mut overrides: Vec<(String, JSONValue)> = Vec::new();
    let by_css: BTreeMap<String, String> = register
        .tokens
        .keys()
        .filter(|path| !path.contains('$'))
        .filter_map(|path| css_name_for(path).map(|name| (name, path.clone())))
        .collect();

    // `color.$dark` is a group, so its values are leaves under it —
    // `color.$dark.card`, `color.$dark.mint.ink`, `color.$dark.wash-rose`.
    let mut unresolved: Vec<String> = Vec::new();
    let mut any = false;
    for (path, token) in &register.tokens {
        let Some(key) = path.strip_prefix("color.$dark.") else {
            continue;
        };
        any = true;
        let css = dark_css_name(key);
        match by_css.get(&css) {
            Some(target) => overrides.push((target.clone(), token.value.clone())),
            None => unresolved.push(format!("{path} (--{css})")),
        }
    }
    for (path, token) in &register.tokens {
        if let Some(rest) = path.strip_prefix("shadow.$dark.") {
            any = true;
            overrides.push((format!("shadow.{rest}"), token.value.clone()));
        }
    }
    if !unresolved.is_empty() {
        return Err(format!(
            "the register's $dark overlay names no token for: {}",
            unresolved.join(", ")
        ));
    }
    if !any {
        // A register without a dark group is legal: the light values stand.
        return Ok(register.clone());
    }
    if overrides.is_empty() {
        return Ok(register.clone());
    }
    register.merged(&override_document(&overrides))
}

/// Turn `(path, value)` pairs into the nested override document
/// [`TokenRegister::merged`] takes.
fn override_document(pairs: &[(String, JSONValue)]) -> JSONValue {
    let mut root = serde_json::Map::new();
    for (path, value) in pairs {
        let mut cursor = &mut root;
        let segments: Vec<&str> = path.split('.').collect();
        for segment in &segments[..segments.len() - 1] {
            cursor = cursor
                .entry((*segment).to_string())
                .or_insert_with(|| JSONValue::Object(serde_json::Map::new()))
                .as_object_mut()
                .expect("the override tree is built from objects only");
        }
        cursor.insert(segments[segments.len() - 1].to_string(), value.clone());
    }
    JSONValue::Object(root)
}

// ── resolution ───────────────────────────────────────────────────────────────

/// One pair of resolved colours, with the WCAG ratio recomputed from them.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContrastPair {
    pub fg: String,
    pub bg: String,
    pub ratio: f64,
    pub required: f64,
    pub ok: bool,
    /// False when a value could not be read as an opaque colour; the pair is
    /// then **not** counted as passing.
    pub computed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The pairs §5's contrast rule judges: normal text on each surface, the
/// identity inks on their washes, and the state inks on their chips.
pub const CONTRAST_PAIRS: [(&str, &str); 20] = [
    ("color.ink-level.ink", "color.surface.card"),
    ("color.ink-level.ink", "color.surface.sheet"),
    ("color.ink-level.ink", "color.surface.well"),
    ("color.ink-level.ink", "color.surface.backdrop"),
    ("color.ink-level.ink-2", "color.surface.card"),
    ("color.ink-level.ink-2", "color.surface.sheet"),
    ("color.ink-level.ink-3", "color.surface.card"),
    ("color.ink-level.ink-3", "color.surface.sheet"),
    ("color.ink-level.ink-inv", "color.identity.mint.ink"),
    ("color.ink-level.ink-inv", "color.identity.lilac.ink"),
    ("color.ink-level.ink-inv", "color.identity.butter.ink"),
    ("color.ink-level.ink-inv", "color.identity.sky.ink"),
    ("color.identity.mint.ink", "color.identity.mint.surface"),
    ("color.identity.lilac.ink", "color.identity.lilac.surface"),
    ("color.identity.butter.ink", "color.identity.butter.surface"),
    ("color.identity.sky.ink", "color.identity.sky.surface"),
    ("color.state.overdue.ink", "color.state.overdue.surface"),
    ("color.state.risk.ink", "color.state.risk.surface"),
    ("color.state.ok.ink", "color.state.ok.surface"),
    ("color.state.info.ink", "color.state.info.surface"),
];

/// Normal text's WCAG 2.x floor (§5: `design.contrast.on-wash ≥ 4.5`).
pub const NORMAL_TEXT_CONTRAST: f64 = 4.5;

/// A fully resolved appearance: the theme's identity, its mode, the scale, the
/// complete CSS custom-property set, and the recomputed contrast table.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedAppearance {
    pub theme: String,
    #[serde(rename = "themeName")]
    pub theme_name: String,
    pub mode: String,
    #[serde(rename = "resolvedMode")]
    pub resolved_mode: String,
    #[serde(rename = "textScale")]
    pub text_scale: f64,
    /// Custom property → resolved value, exactly what the UI applies.
    pub css: BTreeMap<String, String>,
    /// The plan's token overrides, flattened to `path → value` — what the
    /// appearance editor lists beside its Reset controls (§4.1: the file alone
    /// stores them, and the screen never hides their identity).
    pub overrides: BTreeMap<String, String>,
    pub contrast: Vec<ContrastPair>,
    pub warnings: Vec<String>,
}

impl ResolvedAppearance {
    /// The pairs that failed their floor, for the design doctor and the gates.
    pub fn failures(&self) -> impl Iterator<Item = &ContrastPair> {
        self.contrast.iter().filter(|pair| !pair.ok)
    }
}

/// Resolve the register, the theme, the caller's mode, the text scale and the
/// plan's overrides into one appearance. This is the only place the order
/// lives; the UI, the CLI and the doctor all call it.
pub fn resolve(
    register: &TokenRegister,
    theme: Option<&ThemeDoc>,
    user_mode: &str,
    text_scale: f64,
    overrides: Option<&JSONValue>,
) -> Result<ResolvedAppearance, String> {
    let mut warnings = Vec::new();
    let mut resolved_mode = if user_mode == "dark" {
        "dark".to_string()
    } else {
        "light".to_string()
    };
    let mut theme_id = DEFAULT_THEME.to_string();
    let mut theme_name = "Cadence".to_string();
    if let Some(theme) = theme {
        theme_id = theme.id.clone();
        theme_name = theme.name.clone();
        resolved_mode = theme.resolved_mode(user_mode);
    }

    let mut tokens = register.clone();
    if resolved_mode == "dark" {
        tokens = apply_dark_overlay(&tokens)?;
    }
    if let Some(theme) = theme
        && let Some(doc) = &theme.tokens
        && !doc.is_null()
    {
        tokens = tokens.merged(doc)?;
    }
    if let Some(doc) = overrides
        && !doc.is_null()
    {
        tokens = tokens.merged(doc)?;
    }
    let scale = text_scale.clamp(TEXT_SCALE_RANGE.0, TEXT_SCALE_RANGE.1);
    if (scale - text_scale).abs() > f64::EPSILON {
        warnings.push(format!(
            "textScale {text_scale} is outside {}–{}; read as {scale}",
            TEXT_SCALE_RANGE.0, TEXT_SCALE_RANGE.1
        ));
    }
    if (scale - 1.0).abs() > f64::EPSILON {
        tokens = scale_font_sizes(&tokens, scale)?;
    }

    Ok(ResolvedAppearance {
        theme: theme_id,
        theme_name,
        mode: theme
            .map(|theme| theme.mode().to_string())
            .unwrap_or_else(|| "auto".into()),
        resolved_mode,
        text_scale: scale,
        css: css_values(&tokens),
        overrides: overrides.map(flatten_overrides).unwrap_or_default(),
        contrast: contrast_table(&tokens),
        warnings,
    })
}

/// Flatten an override document into `dotted.path → scalar text` pairs — the
/// same paths the file holds, so a row can name what it edits.
fn flatten_overrides(document: &JSONValue) -> BTreeMap<String, String> {
    fn walk(value: &JSONValue, prefix: &str, out: &mut BTreeMap<String, String>) {
        match value {
            JSONValue::Object(members) => {
                for (key, child) in members {
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{prefix}.{key}")
                    };
                    walk(child, &path, out);
                }
            }
            other => {
                out.insert(prefix.to_string(), scalar_text(other));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(document, "", &mut out);
    out
}

/// Resolve a plan's appearance: its pinned theme, the caller's mode, its text
/// scale and its overrides, over the bundled register. The one entry point the
/// shell, the CLI and the doctor share.
pub fn resolve_for(
    config: &ResolvedConfig,
    resources_dir: &Path,
    user_mode: &str,
) -> Result<ResolvedAppearance, String> {
    let register = TokenRegister::bundled(resources_dir)?;
    let theme_id = config
        .appearance
        .as_ref()
        .and_then(|appearance| appearance.theme.clone())
        .unwrap_or_else(|| DEFAULT_THEME.to_string());
    let theme = load(resources_dir, &theme_id)?;
    let scale = config
        .appearance
        .as_ref()
        .and_then(|appearance| appearance.text_scale)
        .unwrap_or(1.0);
    let overrides = config
        .appearance
        .as_ref()
        .and_then(|appearance| appearance.overrides.as_ref());
    let mut resolved = resolve(&register, theme.as_ref(), user_mode, scale, overrides)?;
    if theme.is_none() {
        resolved.warnings.push(format!(
            "theme \"{theme_id}\" is not in {} — resolved as the bundled default",
            themes_dir(resources_dir).display()
        ));
    }
    Ok(resolved)
}

/// Resolve the **bundled** appearance: the register and the default theme, with
/// no plan's theme pin, text scale or overrides. A machine with no plan still
/// reads an appearance — the rail's theme disc is the one control the first run
/// draws, and a control that cannot honour a press is not drawn at all.
pub fn resolve_defaults(
    resources_dir: &Path,
    user_mode: &str,
) -> Result<ResolvedAppearance, String> {
    let register = TokenRegister::bundled(resources_dir)?;
    let theme = load(resources_dir, DEFAULT_THEME)?;
    let mut resolved = resolve(&register, theme.as_ref(), user_mode, 1.0, None)?;
    if theme.is_none() {
        resolved.warnings.push(format!(
            "theme \"{DEFAULT_THEME}\" is not in {} — resolved as the register alone",
            themes_dir(resources_dir).display()
        ));
    }
    Ok(resolved)
}

/// Scale every `fontSize` token by `scale`: the design system's rungs are
/// absolute px stops, so a text-size preference is a multiplication of the
/// whole scale, not one root font size.
fn scale_font_sizes(register: &TokenRegister, scale: f64) -> Result<TokenRegister, String> {
    let mut overrides: Vec<(String, JSONValue)> = Vec::new();
    for (path, token) in &register.tokens {
        if !path.starts_with("fontSize.") {
            continue;
        }
        let JSONValue::String(text) = &token.value else {
            continue;
        };
        let Some(number) = text.strip_suffix("px") else {
            continue;
        };
        let Ok(px) = number.trim().parse::<f64>() else {
            continue;
        };
        overrides.push((path.clone(), JSONValue::String(format_px(px * scale))));
    }
    if overrides.is_empty() {
        return Ok(register.clone());
    }
    register.merged(&override_document(&overrides))
}

/// `13px` × 1.15 → `14.95px`; whole numbers lose their decimal point.
fn format_px(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if (rounded - rounded.round()).abs() < f64::EPSILON {
        format!("{}px", rounded.round() as i64)
    } else {
        format!("{rounded}px")
    }
}

// ── contrast ─────────────────────────────────────────────────────────────────

/// Recompute every pair from resolved colours (§1.4.A).
pub fn contrast_table(register: &TokenRegister) -> Vec<ContrastPair> {
    CONTRAST_PAIRS
        .iter()
        .map(|(fg_path, bg_path)| {
            let fg_text = register
                .tokens
                .get(*fg_path)
                .map(|token| scalar_text(&token.value))
                .or_else(|| {
                    register
                        .tokens
                        .get(*fg_path)
                        .and_then(|token| token.hex.clone())
                });
            let bg_text = register
                .tokens
                .get(*bg_path)
                .map(|token| scalar_text(&token.value))
                .or_else(|| {
                    register
                        .tokens
                        .get(*bg_path)
                        .and_then(|token| token.hex.clone())
                });
            let fg = fg_text.as_deref().and_then(parse_colour);
            let bg = bg_text.as_deref().and_then(parse_colour);
            match (fg, bg) {
                (Some(fg), Some(bg)) => {
                    let ratio = contrast_ratio(fg, bg);
                    ContrastPair {
                        fg: (*fg_path).to_string(),
                        bg: (*bg_path).to_string(),
                        ratio: (ratio * 100.0).round() / 100.0,
                        required: NORMAL_TEXT_CONTRAST,
                        ok: ratio + 1e-9 >= NORMAL_TEXT_CONTRAST,
                        computed: true,
                        note: None,
                    }
                }
                _ => ContrastPair {
                    fg: (*fg_path).to_string(),
                    bg: (*bg_path).to_string(),
                    ratio: 0.0,
                    required: NORMAL_TEXT_CONTRAST,
                    ok: false,
                    computed: false,
                    note: Some(format!(
                        "not computed from {} on {} — a translucent or computed colour",
                        fg_text.unwrap_or_else(|| "(missing)".into()),
                        bg_text.unwrap_or_else(|| "(missing)".into()),
                    )),
                },
            }
        })
        .collect()
}

/// An opaque sRGB colour, 0–1 per channel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

/// `oklch(L% C H)` and `oklch(L% C H / A)`, `#rrggbb`, `rgb(r g b)`. A colour
/// with alpha < 1 returns `None`: compositing needs a third colour, and a
/// guess here would be a contrast claim the engine cannot support.
pub fn parse_colour(text: &str) -> Option<Rgb> {
    let text = text.trim();
    if let Some(inner) = text.strip_prefix('#') {
        if inner.len() < 6 {
            return None;
        }
        if inner.len() > 6 {
            // `#rrggbbaa` — opaque only.
            let alpha = u8::from_str_radix(&inner[6..8], 16).ok()?;
            if alpha != 255 {
                return None;
            }
        }
        let channel = |at: usize| u8::from_str_radix(&inner[at..at + 2], 16).ok();
        return Some(Rgb {
            r: channel(0)? as f64 / 255.0,
            g: channel(2)? as f64 / 255.0,
            b: channel(4)? as f64 / 255.0,
        });
    }
    if let Some(inner) = text
        .strip_prefix("oklch(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let (body, alpha) = match inner.split_once('/') {
            Some((body, alpha)) => (body, Some(alpha.trim())),
            None => (inner, None),
        };
        if let Some(alpha) = alpha
            && alpha
                .parse::<f64>()
                .ok()
                .filter(|value| *value >= 1.0)
                .is_none()
        {
            return None;
        }
        let parts: Vec<&str> = body.split_whitespace().collect();
        if parts.len() < 3 {
            return None;
        }
        let lightness = parts[0].trim_end_matches('%').parse::<f64>().ok()?;
        let lightness = if parts[0].ends_with('%') {
            lightness / 100.0
        } else {
            lightness
        };
        let chroma = parts[1].parse::<f64>().ok()?;
        let hue = parts[2].parse::<f64>().ok()?;
        return Some(oklch_to_srgb(lightness, chroma, hue));
    }
    None
}

/// oklch → linear sRGB (Björn Ottosson's matrices), clamped to the gamut.
pub fn oklch_to_srgb(lightness: f64, chroma: f64, hue_degrees: f64) -> Rgb {
    let hue = hue_degrees.to_radians();
    let a = chroma * hue.cos();
    let b = chroma * hue.sin();
    let l_ = lightness + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = lightness - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = lightness - 0.0894841775 * a - 1.2914855480 * b;
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;
    Rgb {
        r: (4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s).clamp(0.0, 1.0),
        g: (-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s).clamp(0.0, 1.0),
        b: (-0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s).clamp(0.0, 1.0),
    }
}

/// WCAG 2.x relative luminance for a linear-light sRGB triple.
pub fn luminance(colour: Rgb) -> f64 {
    0.2126 * colour.r + 0.7152 * colour.g + 0.0722 * colour.b
}

/// WCAG 2.x contrast ratio.
pub fn contrast_ratio(fg: Rgb, bg: Rgb) -> f64 {
    let a = luminance(fg);
    let b = luminance(bg);
    let (lighter, darker) = if a >= b { (a, b) } else { (b, a) };
    (lighter + 0.05) / (darker + 0.05)
}

// ── the design doctor (§5) ───────────────────────────────────────────────────

/// One finding: the rule, its effective severity, where it is, and what it saw.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Finding {
    pub rule: String,
    pub severity: String,
    pub path: String,
    pub message: String,
    /// True when the plan waived this rule — reported, never hidden.
    pub waived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// One rule's report: whether the engine could look, and what it found.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RuleReport {
    pub id: String,
    pub severity: String,
    pub checked: bool,
    pub note: String,
    pub findings: Vec<Finding>,
}

/// The doctor's output: a severity-level view over §5's rules, plus the
/// resolved appearance the contrast was recomputed from.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DesignReport {
    #[serde(rename = "theme")]
    pub appearance: ResolvedAppearance,
    pub rules: Vec<RuleReport>,
    pub errors: usize,
    pub advisories: usize,
    pub waived: usize,
    pub unobservable: Vec<String>,
}

/// Run §5's lint vocabulary against a loaded plan (§6 Phase 7). The doctor
/// reports; it never blocks a save — `errors` are the release gate's business,
/// and an integrity error would already have failed the load.
pub fn design_check(
    config: &ResolvedConfig,
    resources_dir: &Path,
    user_mode: &str,
) -> Result<DesignReport, String> {
    let register = TokenRegister::bundled(resources_dir)?;
    let appearance = resolve_for(config, resources_dir, user_mode)?;

    let lint = config.rules.lint.as_ref();
    let mut reports: Vec<RuleReport> = Vec::new();
    let mut findings_by_rule: BTreeMap<&str, Vec<Finding>> = BTreeMap::new();

    // -- the contrast table: normal text on resolved colour pairs.
    for pair in appearance.failures() {
        findings_by_rule
            .entry("design.contrast.on-wash")
            .or_default()
            .push(finding(
                "design.contrast.on-wash",
                &pair.fg,
                format!(
                    "{} on {} resolves to {:.2}:1, below the {:.1}:1 floor",
                    short(&pair.fg),
                    short(&pair.bg),
                    pair.ratio,
                    pair.required
                ),
            ));
    }

    // -- a chromatic page background: judged on the **resolved** backdrop (the
    //    one the screen actually paints), not the bundle's default.
    if let Some(text) = appearance.css.get("--backdrop")
        && let Some(chroma) = oklch_chroma(text)
        && chroma > 0.08
    {
        findings_by_rule
            .entry("design.no.chromatic-page-bg")
            .or_default()
            .push(finding(
                "design.no.chromatic-page-bg",
                "color.surface.backdrop",
                format!(
                    "the page backdrop carries chroma {chroma:.3}; §11 keeps the room quiet (a tint, not a lamp)"
                ),
            ));
    }

    // -- the type scale is monotonic.
    for problem in type_scale_problems(&appearance.css) {
        findings_by_rule
            .entry("design.type.scale")
            .or_default()
            .push(finding("design.type.scale", "fontSize", problem));
    }

    // -- durations and easings are the declared ones.
    for problem in motion_problems(config, &register) {
        findings_by_rule
            .entry("design.motion.duration")
            .or_default()
            .push(finding("design.motion.duration", "motion", problem));
    }

    // -- a type with zero views (§3.6: viewless types rot).
    for name in config.types.keys() {
        let has_view = config
            .views
            .values()
            .any(|view| view.type_.as_deref() == Some(name.as_str()));
        if !has_view {
            findings_by_rule
                .entry("content.type.views")
                .or_default()
                .push(finding(
                    "content.type.views",
                    "content/views.json#/views".to_string(),
                    format!("type \"{name}\" has no view — a junk drawer in the making"),
                ));
        }
    }

    // -- every field carries a plain-language label (§5 I2): the label is the
    //    only name a student reads, so an unlabelled field is a hole in the UI.
    for (name, def) in &config.types {
        for (index, field) in def.fields.iter().enumerate() {
            let labelled = field
                .label
                .as_deref()
                .is_some_and(|label| !label.trim().is_empty());
            if !labelled {
                findings_by_rule
                    .entry("content.field.no-label")
                    .or_default()
                    .push(finding(
                        "content.field.no-label",
                        format!("content/types.json#/types/{name}/fields/{index}/label"),
                        format!("field \"{}\" on type \"{name}\" has no label", field.key),
                    ));
            }
        }
    }

    // -- §11's taste rules, where they are name-based by design (§5's ceiling:
    //    they detect a streak by its label, not its meaning).
    for (path, text) in labels(config) {
        if let Some(phrase) = streak_phrase(&text) {
            findings_by_rule
                .entry("design.no.streak-guilt")
                .or_default()
                .push(finding(
                    "design.no.streak-guilt",
                    path.clone(),
                    format!(
                        "\"{phrase}\" counts consecutive days, not work done — §11 refuses streak loss"
                    ),
                ));
        }
        if let Some(word) = ring_word(&text) {
            findings_by_rule
                .entry("design.no.donut-ring")
                .or_default()
                .push(finding(
                    "design.no.donut-ring",
                    path,
                    format!(
                        "\"{word}\" draws progress as a whole — §11 refuses the ring for a partial thing"
                    ),
                ));
        }
    }

    for rule in LINT_RULES.iter() {
        let severity = lint
            .map(|lint| lint.severity(rule))
            .unwrap_or_else(|| rule.severity.to_string());
        let waiver = lint.and_then(|lint| lint.waiver(rule.id));
        let mut findings = findings_by_rule.get(rule.id).cloned().unwrap_or_default();
        for found in &mut findings {
            found.severity = severity.clone();
            if let Some(waiver) = waiver {
                found.waived = true;
                found.reason = waiver.reason.clone();
            }
        }
        reports.push(RuleReport {
            id: rule.id.to_string(),
            severity,
            checked: rule.checked,
            note: rule.note.to_string(),
            findings,
        });
    }

    let mut errors = 0;
    let mut advisories = 0;
    let mut waived = 0;
    for report in &reports {
        for found in &report.findings {
            if found.waived {
                waived += 1;
            } else if report.severity == "error" {
                errors += 1;
            } else {
                advisories += 1;
            }
        }
    }
    let unobservable = reports
        .iter()
        .filter(|report| !report.checked)
        .map(|report| report.id.clone())
        .collect();

    Ok(DesignReport {
        appearance,
        rules: reports,
        errors,
        advisories,
        waived,
        unobservable,
    })
}

fn finding(rule: &str, path: impl Into<String>, message: impl Into<String>) -> Finding {
    Finding {
        rule: rule.to_string(),
        severity: String::new(),
        path: path.into(),
        message: message.into(),
        waived: false,
        reason: None,
    }
}

fn short(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

/// The design system's scale, low to high. Fixed vocabulary, like the field
/// types: a rung added to the register without a place here is a stated gap.
const TYPE_RUNGS: [&str; 10] = [
    "--text-2xs",
    "--text-xs",
    "--text-sm",
    "--text-base",
    "--text-md",
    "--text-lg",
    "--text-xl",
    "--text-2xl",
    "--text-3xl",
    "--text-display",
];

fn type_scale_problems(css: &BTreeMap<String, String>) -> Vec<String> {
    let mut problems = Vec::new();
    let mut previous: Option<(String, f64)> = None;
    for rung in TYPE_RUNGS {
        let Some(text) = css.get(rung) else {
            continue;
        };
        let Some(px) = text.strip_suffix("px").and_then(|n| n.parse::<f64>().ok()) else {
            continue;
        };
        if let Some((last_rung, last_px)) = &previous
            && px < *last_px
        {
            problems.push(format!(
                "{rung} ({px}px) is smaller than {last_rung} ({last_px}px) — the scale must be monotonic"
            ));
        }
        previous = Some((rung.to_string(), px));
    }
    problems
}

/// §5: "one curve, declared durations only" — a changed duration or easing is
/// a finding; an unchanged register has none.
fn motion_problems(config: &ResolvedConfig, bundled: &TokenRegister) -> Vec<String> {
    let mut problems = Vec::new();
    for (path, token) in &config.tokens.tokens {
        if !(path.starts_with("motion.duration.") || path.starts_with("motion.easing.")) {
            continue;
        }
        if let Some(original) = bundled.tokens.get(path)
            && original.value != token.value
        {
            problems.push(format!(
                "{path} was changed from {} to {} — motion is declared, not themed",
                scalar_text(&original.value),
                scalar_text(&token.value)
            ));
        }
    }
    problems
}

/// Every label-like string a plan carries, with its JSON path. Name-based
/// linting needs names; nothing here is a guess about behaviour.
fn labels(config: &ResolvedConfig) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(shell) = &config.shell
        && let Some(navigation) = &shell.navigation
    {
        for (index, entry) in navigation.iter().enumerate() {
            out.push((
                format!("content/shell.json#/navigation/{index}/title"),
                entry.title.clone(),
            ));
        }
    }
    for (name, view) in &config.views {
        out.push((format!("content/views.json#/views/{name}"), name.clone()));
        if let Some(blocks) = &view.blocks {
            collect_block_labels(
                blocks,
                &format!("content/views.json#/views/{name}/blocks"),
                &mut out,
            );
        }
    }
    for (name, def) in &config.types {
        for field in &def.fields {
            if let Some(label) = &field.label {
                out.push((
                    format!("content/types.json#/types/{name}/fields/{}", field.key),
                    label.clone(),
                ));
            }
        }
    }
    if let Some(metrics) = &config.rules.metrics {
        for (name, metric) in metrics {
            out.push((
                format!("content/rules.json#/metrics/{name}"),
                metric.label.clone(),
            ));
        }
    }
    out
}

fn collect_block_labels(
    blocks: &[crate::model::BlockDef],
    path: &str,
    out: &mut Vec<(String, String)>,
) {
    for (index, block) in blocks.iter().enumerate() {
        let at = format!("{path}/{index}");
        for (key, value) in [
            ("title", &block.title),
            ("label", &block.label),
            ("text", &block.text),
        ] {
            if let Some(text) = value {
                out.push((format!("{at}/{key}"), text.clone()));
            }
        }
        if let Some(children) = &block.blocks {
            collect_block_labels(children, &format!("{at}/blocks"), out);
        }
        if let Some(children) = &block.else_blocks {
            collect_block_labels(children, &format!("{at}/else"), out);
        }
    }
}

/// §11's refusal, name-based: the phrases a streak counter actually uses.
fn streak_phrase(text: &str) -> Option<&'static str> {
    const PHRASES: [&str; 6] = [
        "streak",
        "don't break the chain",
        "dont break the chain",
        "you're behind",
        "you are behind",
        "consistency score",
    ];
    let lowered = text.to_lowercase();
    PHRASES.into_iter().find(|phrase| lowered.contains(phrase))
}

/// §11's refusal of the ring, name-based: a whole drawn for a partial thing.
fn ring_word(text: &str) -> Option<&'static str> {
    const WORDS: [&str; 2] = ["donut", "ring"];
    let lowered = text.to_lowercase();
    WORDS.into_iter().find(|word| {
        lowered
            .split(|character: char| !character.is_alphanumeric())
            .any(|part| part == *word)
    })
}

/// The chroma of an `oklch(L C H)` value, when it is one.
fn oklch_chroma(text: &str) -> Option<f64> {
    let inner = text.trim().strip_prefix("oklch(")?.strip_suffix(')')?;
    let body = inner.split('/').next()?;
    let parts: Vec<&str> = body.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    parts[1].parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resources() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Sources/SAMCore/Resources")
    }

    fn register() -> TokenRegister {
        TokenRegister::bundled(&resources()).expect("the bundled register loads")
    }

    #[test]
    fn every_token_maps_to_a_custom_property() {
        let register = register();
        let mut unmapped = Vec::new();
        for (path, _) in &register.tokens {
            if path.contains("$dark") {
                continue;
            }
            if css_name_for(path).is_none() {
                unmapped.push(path.clone());
            }
        }
        assert!(
            unmapped.is_empty(),
            "tokens with no CSS custom property: {unmapped:?}"
        );
    }

    #[test]
    fn the_dark_overlay_applies_every_name() {
        let register = register();
        let dark = apply_dark_overlay(&register).expect("every $dark key maps to a token");
        let css = css_values(&dark);
        assert_eq!(
            css.get("--card").map(String::as_str),
            Some("oklch(30.0% 0.010 260)")
        );
        assert_eq!(
            css.get("--ink").map(String::as_str),
            Some("oklch(93.0% 0.006 260)")
        );
        assert_eq!(
            css.get("--w-mint").map(String::as_str),
            Some("oklch(32.0% 0.045 168)")
        );
        assert_eq!(
            css.get("--fg-mint").map(String::as_str),
            Some("oklch(90.0% 0.045 168)")
        );
        assert_eq!(
            css.get("--wash-rose").map(String::as_str),
            Some("oklch(30.0% 0.030 10)")
        );
        assert_eq!(
            css.get("--chip-ok").map(String::as_str),
            Some("oklch(32.0% 0.050 168)")
        );
        assert_eq!(css.get("--weight-body").map(String::as_str), Some("350"));
        // The overlay namespace itself has no custom property.
        assert!(!css.contains_key("--$dark"));
    }

    #[test]
    fn the_light_resolution_keeps_the_registers_values() {
        let register = register();
        let light = resolve(&register, None, "light", 1.0, None).expect("resolves");
        let css = &light.css;
        assert_eq!(
            css.get("--card").map(String::as_str),
            Some("oklch(98.6% 0.002 260)")
        );
        assert_eq!(light.resolved_mode, "light");
        assert!(
            light.failures().count() == 0,
            "the default palette passes its own floor"
        );
    }

    #[test]
    fn auto_follows_the_callers_mode_and_a_dark_theme_does_not() {
        let register = register();
        let auto = ThemeDoc {
            schema_version: Some(1),
            id: "auto".into(),
            name: "Auto".into(),
            mode: Some("auto".into()),
            description: None,
            tokens: None,
        };
        assert_eq!(
            resolve(&register, Some(&auto), "dark", 1.0, None)
                .unwrap()
                .resolved_mode,
            "dark"
        );
        let dark = ThemeDoc {
            mode: Some("dark".into()),
            ..auto.clone()
        };
        assert_eq!(
            resolve(&register, Some(&dark), "light", 1.0, None)
                .unwrap()
                .resolved_mode,
            "dark"
        );
    }

    #[test]
    fn text_scale_multiplies_every_font_size_rung() {
        let register = register();
        let scaled = resolve(&register, None, "light", 1.15, None).expect("resolves");
        assert_eq!(
            scaled.css.get("--text-base").map(String::as_str),
            Some("17.25px")
        );
        assert_eq!(
            scaled.css.get("--text-2xs").map(String::as_str),
            Some("12.65px")
        );
        assert_eq!(scaled.text_scale, 1.15);
        assert_eq!(
            scaled.css.get("--space-2xs").map(String::as_str),
            Some("4px")
        );
    }

    #[test]
    fn contrast_is_recomputed_from_resolved_colours() {
        // The measured metadata says 15.8 for ink on card; the computed value
        // must agree with the design system's own number.
        let register = register();
        let table = contrast_table(&register);
        let pair = table
            .iter()
            .find(|pair| pair.fg == "color.ink-level.ink" && pair.bg == "color.surface.card")
            .expect("the pair is judged");
        assert!(pair.computed);
        assert!(
            (pair.ratio - 15.8).abs() < 0.35,
            "computed {:.2}, metadata says 15.8",
            pair.ratio
        );
        let ink = parse_colour("oklch(24.0% 0.010 260)").unwrap();
        let card = parse_colour("oklch(98.6% 0.002 260)").unwrap();
        assert!((contrast_ratio(ink, card) - pair.ratio).abs() < 0.01);
    }

    #[test]
    fn a_translucent_colour_is_reported_uncomputed_not_passing() {
        let parsed = parse_colour("oklch(100% 0 0 / 0.55)");
        assert!(parsed.is_none());
        let hex = parse_colour("#4a2f2c47");
        assert!(hex.is_none());
        assert!(parse_colour("#f0dcdb").is_some());
    }

    #[test]
    fn every_shipped_theme_resolves_and_passes_the_floor() {
        let register = register();
        let themes = list(&resources()).expect("themes load");
        assert!(themes.len() >= 4, "four themes ship");
        for theme in &themes {
            for mode in ["light", "dark"] {
                let resolved = resolve(&register, Some(theme), mode, 1.0, None)
                    .unwrap_or_else(|error| panic!("{}: {error}", theme.id));
                for pair in resolved.failures() {
                    panic!(
                        "{} in {mode}: {} on {} is {:.2}:1",
                        theme.id, pair.fg, pair.bg, pair.ratio
                    );
                }
            }
        }
    }

    #[test]
    fn type_scale_and_lint_vocabulary_are_pinned() {
        assert_eq!(LINT_RULES.len(), 13);
        assert_eq!(LINT_RULES.iter().filter(|rule| rule.checked).count(), 9);
        let register = register();
        let css = css_values(&register);
        assert!(type_scale_problems(&css).is_empty());
    }
}
