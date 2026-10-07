//! The port's oracle (§6.1, D22).
//!
//! `Sources/SAMCore/Resources/fixtures/**` crosses into the Rust workspace
//! **byte-for-byte** and stays the specification of behaviour the Swift engine
//! already defined. A corpus with no failing test is not a corpus, so every
//! fixture is listed here with its size: deleting one, adding one, or editing
//! one fails this test rather than quietly narrowing what "passes" means.
//!
//! Phase 1 loads every fixture through the **real** loader — `ConfigStore::load`
//! → `ResolvedConfig` — not the Phase 0A fixture resolver, which that phase
//! deleted. The `problemset` model test §10 asks for asserts the same facts it
//! did then, against the same bytes.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sam_core::config_store::{ConfigStoreError, load_with, raw_revision};
use sam_core::evaluator::{L2Value, formula_scalar};
use sam_core::model::Record;
use sam_core::resolved_config::ResolvedConfig;
use serde_json::json;

/// Every fixture file and its byte length. The list is the corpus: the plan
/// counts 45 files (of which 27 are files and the rest directories); what is
/// pinned here is what actually exists, and that is a deviation recorded in
/// `BUILDLOG.md` rather than silently reconciled.
///
/// `screen/**` is the **Phase 5** addition: a plan whose views compose blocks,
/// including a kind this build does not know (forward compatibility), so the
/// placeholder path has bytes behind it rather than only a unit test.
///
/// `phase6/**` is the **Phase 6** addition: pipelines, scheduler state and
/// derived metrics as bytes — a completed topic with a due fixed-ladder review,
/// a proved-but-unanchored topic, a `progress`-carrying pipeline, and two views
/// (a panel destination and a translatable filter for the index parity gate).
const CORPUS: [(&str, u64); 38] = [
    ("crlf-ok/content/records/topic.jsonl", 197),
    ("crlf-ok/content/rules.json", 66),
    ("crlf-ok/content/types.json", 397),
    ("invalid/dangling-relation/content/records/topic.jsonl", 116),
    ("invalid/dangling-relation/content/rules.json", 91),
    ("invalid/dangling-relation/content/types.json", 574),
    ("invalid/duplicate-ids/content/records/topic.jsonl", 185),
    ("invalid/duplicate-ids/content/rules.json", 91),
    ("invalid/duplicate-ids/content/types.json", 574),
    ("invalid/duplicate-keys/content/types.json", 150),
    ("invalid/empty-input/content/types.json", 0),
    ("invalid/formula-cycle/content/types.json", 285),
    ("invalid/invalid-utf8/content/types.json", 75),
    ("invalid/invalid-value/content/records/course.jsonl", 103),
    ("invalid/invalid-value/content/records/topic.jsonl", 113),
    ("invalid/invalid-value/content/rules.json", 91),
    ("invalid/invalid-value/content/types.json", 574),
    ("invalid/malformed-jsonl/content/records/topic.jsonl", 118),
    ("invalid/malformed-jsonl/content/rules.json", 66),
    ("invalid/malformed-jsonl/content/types.json", 397),
    ("invalid/type-parent-cycle/content/types.json", 248),
    ("invalid/unknown-schema-version/content/types.json", 85),
    ("phase6/content/records/course.jsonl", 99),
    ("phase6/content/records/problemset.jsonl", 271),
    ("phase6/content/records/topic.jsonl", 427),
    ("phase6/content/rules.json", 1226),
    ("phase6/content/types.json", 1290),
    ("phase6/content/views.json", 881),
    ("phase6/state/state.json", 1037),
    ("problemset/content/records/course.jsonl", 254),
    ("problemset/content/records/problemset.jsonl", 699),
    ("problemset/content/rules.json", 174),
    ("problemset/content/types.json", 1226),
    ("problemset/content/views.json", 324),
    ("screen/content/records/topic.jsonl", 374),
    ("screen/content/rules.json", 119),
    ("screen/content/types.json", 331),
    ("screen/content/views.json", 774),
];

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The token register is a bundled input (§1.4.A); in the workspace that is the
/// design system's machine-readable twin, which Phase 0B copies into the bundle.
fn resources() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Sources/SAMCore/Resources")
}

fn load_fixture(name: &str) -> Result<ResolvedConfig, ConfigStoreError> {
    load_with(&corpus_root().join(name), "explicit", &resources())
}

/// Walk the corpus directory and return every file, relative and `/`-joined.
fn walk(dir: &Path, root: &Path, out: &mut BTreeSet<String>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", dir.display()));
    for entry in entries {
        let path = entry.expect("a readable directory entry").path();
        if path.is_dir() {
            walk(&path, root, out);
        } else if path.file_name().is_none_or(|name| name != ".DS_Store") {
            out.insert(
                path.strip_prefix(root)
                    .expect("every path is under the corpus root")
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
}

#[test]
fn fixture_corpus_is_present_counted_and_named() {
    let root = corpus_root();
    assert!(
        root.is_dir(),
        "the port's oracle is missing at {} — the corpus is carried across byte-for-byte (§6.1)",
        root.display()
    );

    let expected: BTreeSet<String> = CORPUS.iter().map(|(path, _)| path.to_string()).collect();
    let mut found = BTreeSet::new();
    walk(&root, &root, &mut found);

    let missing: Vec<&String> = expected.difference(&found).collect();
    let unexpected: Vec<&String> = found.difference(&expected).collect();
    assert!(
        missing.is_empty(),
        "fixtures missing from the corpus: {missing:#?}"
    );
    assert!(
        unexpected.is_empty(),
        "unlisted files in the corpus: {unexpected:#?} — add them to CORPUS deliberately"
    );
    assert_eq!(found.len(), CORPUS.len());

    for (path, size) in CORPUS {
        let actual = std::fs::metadata(root.join(path))
            .unwrap_or_else(|error| panic!("cannot stat {path}: {error}"))
            .len();
        assert_eq!(
            actual, size,
            "{path} changed size ({actual} bytes, expected {size}) — the corpus is not ours to edit"
        );
    }
}

#[test]
fn crlf_and_a_missing_final_newline_load() {
    // §6.1 rule 1: "the Swift engine accepted CRLF, a missing final newline,
    // human-readable ids and byte-identical no-op upserts, and each has a
    // fixture. A port that quietly requires LF … has stopped passing."
    let config = load_fixture("crlf-ok").expect("the CRLF fixture loads without normalising it");
    assert_eq!(config.record_count(), 2, "both records load");
    assert_eq!(config.types.len(), 2, "course and topic");

    // The first line ends CRLF and the last has no final newline; both parse.
    let ids: Vec<&str> = config.records_iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, vec!["t.1", "t.2"]);
    assert_eq!(
        config.record("t.2").unwrap().fields["title"],
        json!("no final newline")
    );
}

#[test]
fn malformed_input_produces_a_path_precise_diagnostic() {
    let diagnostics = expect_invalid("invalid/malformed-jsonl");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "json.invalid"
                && diagnostic.path == "content/records/topic.jsonl"
                && diagnostic.line == Some(2)),
        "the second line is the broken one: {diagnostics:?}"
    );

    let diagnostics = expect_invalid("invalid/empty-input");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.path == "content/types.json"),
        "empty input is not an empty plan: {diagnostics:?}"
    );

    let diagnostics = expect_invalid("invalid/invalid-utf8");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "encoding.invalid"),
        "invalid UTF-8 is rejected: {diagnostics:?}"
    );
}

/// Every invalid fixture fails with the code its Phase 1 gate names, against
/// the file it must be reported for.
#[test]
fn every_invalid_fixture_fails_with_its_gate_code() {
    let expected: [(&str, &str, &str); 10] = [
        (
            "invalid-value",
            "record.invalid-value",
            "content/records/topic.jsonl",
        ),
        (
            "unknown-schema-version",
            "schema-version.unsupported",
            "content/types.json",
        ),
        (
            "type-parent-cycle",
            "types.parent-cycle",
            "content/types.json",
        ),
        ("formula-cycle", "formula.cycle", "content/types.json"),
        (
            "dangling-relation",
            "record.dangling-link",
            "content/records/topic.jsonl",
        ),
        (
            "duplicate-ids",
            "record.duplicate-id",
            "content/records/topic.jsonl",
        ),
        ("empty-input", "json.eof", "content/types.json"),
        ("invalid-utf8", "encoding.invalid", "content/types.json"),
        (
            "malformed-jsonl",
            "json.invalid",
            "content/records/topic.jsonl",
        ),
        ("duplicate-keys", "json.duplicate-key", "content/types.json"),
    ];

    for (name, code, file) in expected {
        let diagnostics = expect_invalid(&format!("invalid/{name}"));
        let wants_line = file.ends_with(".jsonl");
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.code == code
                && diagnostic.path.starts_with(file)
                && (!wants_line || diagnostic.line.is_some())),
            "{name}: expected {code} at {file}, got {:?}",
            diagnostics
                .iter()
                .map(|diagnostic| (&diagnostic.code, &diagnostic.path))
                .collect::<Vec<_>>()
        );
    }
}

fn expect_invalid(name: &str) -> Vec<sam_core::diagnostic::Diagnostic> {
    match load_fixture(name) {
        Ok(_) => panic!("{name} loaded, but it is an invalid fixture"),
        Err(ConfigStoreError::Validation(diagnostics)) => diagnostics,
        Err(error) => panic!("{name} failed as I/O, not validation: {error}"),
    }
}

/// §10: "prove the model with one representative extension" — §3.3's
/// `problemset`, asserting fields, relation cardinality and a formula.
#[test]
fn problemset_fixture_resolves_fields_cardinality_and_formula() {
    let config: ResolvedConfig = load_fixture("problemset").expect("the problemset fixture loads");

    // Fields, including the closed-vocabulary kinds the fixture uses.
    let problemset = config.types.get("problemset").expect("the type exists");
    assert_eq!(problemset.fields.len(), 8);
    assert_eq!(problemset.icon.as_deref(), Some("checklist"));
    assert_eq!(problemset.pipeline.as_deref(), Some("progress"));
    assert_eq!(problemset.trackable, Some(true));
    assert_eq!(
        problemset
            .field("difficulty")
            .and_then(|f| f.options.clone()),
        Some(vec!["easy".to_string(), "medium".into(), "hard".into()])
    );

    // Relation cardinality: `course` is not a declared field on `problemset`
    // — it is the parent edge, and §3.1 makes it single-valued.
    assert!(
        !problemset.fields.iter().any(|f| f.key == "course"),
        "the relation comes from `parent`, not from a field"
    );
    assert_eq!(problemset.parent.as_deref(), Some("course"));
    assert!(problemset.allowed_link_keys().contains("course"));

    // The formula field, and its evaluation over the fixture's records.
    let pct = problemset.field("pct").expect("the derived column exists");
    assert!(pct.is_formula());
    assert_eq!(pct.expr.as_deref(), Some("pct(solved, total)"));
    let expr = pct.expr.clone().unwrap();

    let records: Vec<&Record> = config
        .records_iter()
        .filter(|record| record.type_ == "problemset")
        .collect();
    assert_eq!(records.len(), 3);
    assert_eq!(
        config.record_count(),
        5,
        "three problem sets and two courses"
    );

    let by_id = |id: &str| {
        records
            .iter()
            .find(|record| record.id == id)
            .unwrap_or_else(|| panic!("record {id} is in the fixture"))
    };

    // 12/20 → 60. Formula values are derived, never persisted in `fields`.
    let first = by_id("p.dp.01");
    assert!(!first.fields.contains_key("pct"));
    assert_eq!(
        formula_scalar(&config, first, &expr),
        Some(L2Value::Int(60))
    );
    assert_eq!(first.links.get("course"), Some(&vec!["math".to_string()]));

    // §4.4: pct is null when the denominator is ≤ 0 — not zero, not an error
    // it hides by inventing a value.
    let empty = by_id("p.dp.02");
    assert_eq!(empty.fields["total"], serde_json::json!(0));
    assert_eq!(formula_scalar(&config, empty, &expr), Some(L2Value::Null));

    let full = by_id("p.dp.03");
    assert_eq!(
        formula_scalar(&config, full, &expr),
        Some(L2Value::Int(100))
    );

    // Views are saved queries over the same type (§3.6).
    let view = config.views.get("math.problems").expect("the view exists");
    assert_eq!(view.type_.as_deref(), Some("problemset"));
    assert_eq!(view.layout.as_deref(), Some("table"));
    assert_eq!(view.filter.as_deref(), Some("course.id == 'math'"));
    assert_eq!(view.columns.as_ref().map(Vec::len), Some(6));

    // Pipelines are data (§3.5) — `progress` derives its fraction from `pct`.
    let progress = config
        .rules
        .pipelines
        .as_ref()
        .and_then(|pipelines| pipelines.get("progress"))
        .expect("the pipeline exists");
    assert_eq!(progress.stages, vec!["started", "midway", "done"]);
    assert_eq!(progress.progress.as_deref(), Some("pct"));

    // Canonical serialization round-trips byte-identically (§4.1), so a
    // repeated write is a byte-level no-op.
    assert!(config.records_iter().all(ResolvedConfig::round_trips));

    // And the whole thing dumps as a content tree, which is what
    // `--rendercheck` prints.
    let tree = config.content_tree().join("\n");
    assert!(
        tree.contains("problemset checklist parent=course trackable pipeline=progress"),
        "{tree}"
    );
    assert!(tree.contains("links: course→math"));
    assert!(tree.contains("p.dp.01"));
    assert!(tree.contains("formula: pct=60"));
    assert!(
        tree.contains("formula: pct=null"),
        "the zero-denominator record"
    );
}

/// Phase 5: a screen's blocks cross the real loader. The fixture carries a kind
/// this build does not know, which must **load** (forward compatibility, §0 rule
/// 3) with a warning and resolve to a stated placeholder rather than to nothing.
#[test]
fn the_screen_fixture_loads_and_its_unknown_block_is_a_placeholder() {
    let config: ResolvedConfig = load_fixture("screen").expect("the screen fixture loads");
    let view = config.views.get("screen.demo").expect("the screen exists");
    let blocks = view.blocks.as_deref().expect("it declares blocks");
    assert_eq!(blocks.len(), 5);
    assert_eq!(blocks[3].kind, "table");
    assert!(blocks[3].view.is_none(), "and it draws its own view");
    assert!(blocks[2].is_container());
    assert_eq!(
        config
            .load_warnings
            .iter()
            .filter(|warning| warning.code == "view.unknown-block")
            .count(),
        1,
        "the unknown kind is reported, once: {:?}",
        config.load_warnings
    );

    let outcome = sam_core::views::resolve(
        &config,
        "screen.demo",
        &sam_core::evaluator::EvalContext::frozen(&config, None, 1_768_924_800),
    )
    .expect("the screen resolves");
    let kinds: Vec<&str> = outcome
        .blocks
        .iter()
        .map(|block| block["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, vec!["stat", "callout", "columns", "table", "graph"]);
    assert_eq!(
        outcome.blocks[0]["value"],
        json!(100),
        "30 + 45 + 25 minutes"
    );
    let nested = outcome.blocks[2]["blocks"].as_array().unwrap();
    assert_eq!(nested[0]["kind"], json!("placeholder"));
    assert_eq!(nested[0]["unknown"], json!("sparkline"));
    assert_eq!(nested[1]["value"], json!(3), "the count is over the view");
}

/// `paths` must answer on an invalid plan (§4.7): the revision is over bytes,
/// and it is the same value a successful load publishes.
#[test]
fn a_raw_revision_exists_for_a_plan_that_does_not_validate() {
    let invalid = corpus_root().join("invalid/formula-cycle");
    assert!(load_fixture("invalid/formula-cycle").is_err());
    let revision = raw_revision(&invalid);
    assert_eq!(revision.len(), 16, "FNV-1a 64, hex");
    assert_ne!(revision, raw_revision(&corpus_root().join("problemset")));
}

/// The `phase6` fixture, asserted through the **real** loader and the engine's
/// own reads: a completed topic with a due review, a proved-but-unanchored
/// topic, a `progress`-carrying pipeline, and metrics that are the evaluator's
/// own folds (§6 Phase 6).
#[test]
fn the_phase6_fixture_carries_pipelines_state_and_metrics() {
    let config: ResolvedConfig = load_fixture("phase6").expect("the phase6 fixture loads");
    assert_eq!(config.views.len(), 5);
    assert_eq!(
        config.views["reviews.screen"].panel.as_deref(),
        Some("reviews"),
        "a destination chooses its panel"
    );
    assert_eq!(
        config.views["progress.screen"].panel.as_deref(),
        Some("progress")
    );
    assert_eq!(
        config
            .rules
            .pipelines
            .as_ref()
            .map(|pipelines| pipelines.len()),
        Some(3)
    );

    assert_eq!(config.rules.timezone.as_deref(), Some("UTC"));
    assert_eq!(sam_core::evaluator::plan_timezone(&config), 0);

    let plan = corpus_root().join("phase6");
    let state = sam_core::views::PlanState::load(&plan);
    let entry = state
        .entry("t.bayes")
        .expect("the completed topic has state");
    assert!(entry.recorded("anchored"), "stages are read from the bytes");
    let review = entry
        .review
        .as_ref()
        .and_then(sam_core::scheduler::Review::from_json)
        .expect("and so is its schedule");
    assert_eq!(review.due.as_deref(), Some("2026-01-11"));
    assert_eq!(
        review.step,
        Some(0),
        "the anchoring review consumed no offset"
    );

    let context = sam_core::evaluator::EvalContext::frozen(&config, Some(&state), 1_768_924_800);
    let queue = sam_core::scheduler::due_json(&config, &context);
    let due: Vec<&str> = queue["due"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|row| row["id"].as_str())
        .collect();
    assert_eq!(due, vec!["t.bayes"], "2026-01-11 is due on 2026-01-20");
    assert_eq!(queue["due"][0]["overdueDays"], json!(9));

    let metrics = sam_core::rules::metrics_json(&config, &context);
    let value = |id: &str| {
        metrics["metrics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == json!(id))
            .unwrap()["value"]
            .clone()
    };
    assert_eq!(value("problems.solved"), json!(15), "12 + 3 solved");
    assert_eq!(value("topics.open"), json!(2), "t.naive and t.cond");

    // The `progress` field is derived from the pipeline (§3.5): pct 12/20 = 60.
    let outcome = sam_core::views::resolve(&config, "sets.all", &context).expect("resolves");
    let derived = sam_core::views::record_json(&config, &outcome.records[0]);
    assert_eq!(derived["derived"]["pct"], json!(60));
    let progress = sam_core::evaluator::Evaluation::new(context, 10_000)
        .evaluate(
            &sam_core::expression::parse("done", "test").unwrap(),
            Some(&outcome.records[0]),
        )
        .expect("progress evaluates");
    assert_eq!(
        progress,
        sam_core::evaluator::L2Value::Double(0.6),
        "the pipeline's progress names `pct`, so the fraction is 60/100"
    );
}
