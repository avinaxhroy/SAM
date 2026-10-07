# `language` — a spaced-repetition language plan

**Promise.** Learn one language from its writing system to a working vocabulary by
keeping every word, phrase and dialogue in a review queue that a real scheduler
orders for you.

**Label: Structural template — no date-specific content.** This preset is a shape,
not a syllabus: it asserts no exam dates, no chapter lists and no calendar. Sessions
and self-quizzes are ordered by *week number* (`session.week`, `assessment.week`),
so the plan carries no dates to go stale. The Japanese examples are ordinary
beginner items, included so every table shows something real.

Tested against this build: `--configcheck` exits 0 with **11 types · 53 records ·
22 views · 3 pipelines** and no advisories; `--rendercheck` succeeds; all eight
navigation views resolve.

## What a preset is

A complete, writable plan tree. `SAM plan.new --preset language --name my-japanese`
copies it into your data directory as *your* plan — content, schema and state are
yours from then on, and nothing resurrects a deleted record later.

```text
content/types.json      11 kind of thing: language · track · unit · word · sentence ·
                        dialogue · resource · assessment · session · note · milestone
content/views.json      22 saved queries; every type has at least one
content/rules.json      the srs / check / pages machines, the fsrs scheduler, metrics
content/shell.json      navigation: Today · Vocabulary · Kana · Plan · Reviews ·
                        Progress · Media · Notes
content/appearance.json pins the cadence-light theme
content/records/*.jsonl 53 example records, one canonical record per line
```

## The study method is data, not code

Switching how this plan teaches you is a JSON write. Nothing else changes — not the
views, not the records, not the app.

| Where | Value | What it does |
| --- | --- | --- |
| `content/types.json#/types/word/pipeline` | `srs` | a word is trackable, and `srs` is the machine it follows |
| `content/rules.json#/pipelines/srs` | `stages: [new, learning, review, mature]`, `scheduler: fsrs` | the stage names and which scheduler drives them |
| `content/rules.json#/scheduler/name` | `fsrs` | the plan's default scheduler |
| `content/rules.json#/scheduler/fsrs/desiredRetention` | `0.9` | the retention the intervals are computed for |

Three edits that each change behaviour with zero code changes — all three run against
a copy of this preset and leave `--configcheck` green:

```sh
# 1 · the plan's default scheduler
SAM settings.set --key scheduler --value fixed --plan <plan> --json

# 2 · a pipeline that names its own scheduler wins for its own records, so the word
#     machine is re-pointed where it is declared: set
#     content/rules.json#/pipelines/srs/scheduler to "fixed" (replace "fsrs") and the
#     next logReview answers with the ladder, not the model:
#     → scheduler fixed · due 2026-10-03 · intervalDays 7 · algorithm null

# 3 · plain checkboxes: a word is done when you say so
SAM type.setPipeline --type word --pipeline check --fresh --plan <plan> --json
```

`word` follows `srs`, `sentence` follows `check` (one `done` stage) and `dialogue`
follows `pages` (`reading → read`). One plan, three methods, all declared in
`content/rules.json` — this is §1.2 #4 (the study method is data) in a file you can
read.

### Prove the scheduler with the preset's own records

```sh
# log a review; the FSRS model computes the next civil due date
SAM record.logReview --id w.ja.mizu --rating good --at 2026-09-26T09:00:00Z --plan <plan> --json
# → scheduler fsrs · FSRS-6 · default parameters · due 2026-09-28 · intervalDays 2

# the queue as data: due now, coming up, and waiting on a first pass
SAM reviews.due --plan <plan> --json
```

The reviews panel (`shell.json` → Reviews) renders that same three-way queue — due
now · coming up · waiting on a first pass — and dispatches `record.logReview` back;
`rules.json#/metrics` folds the same data for the Progress panel.

## What the example records show

* **22 words** with a `kind` (`kana` · `kanji` · `word` · `phrase`), a reading and a
  meaning — e.g. `水` / `みず` / water; `w.ja.mizu` links to its unit and its track.
* **4 sentences** and **2 dialogues** — one `check` machine and one `pages` machine.
* **2 self-quizzes** scored out of a total, with a derived column:
  `result` is `pct(score, total)` — a Layer-2 formula, never persisted, computed at
  render (`90` and `83.33`).
* **3 sessions** (25 · 30 · 20 minutes) and **3 notes**, both kinds `"private": true`,
  so their records stay out of a shared profile unless the export asks for them.
* **3 milestones** and **4 resources**; resource URLs point at `example.org` because
  this template pins no shipping source.

Records are canonical JSONL: sorted keys, one record per line, a final newline. A
word is a line you can diff:

```json
{"fields":{"kind":"kanji","meaning":"water","reading":"みず","source":"Everyday nouns","term":"水"},"id":"w.ja.mizu","links":{"track":["t.vocab.01"],"unit":["u.vocab.01"]},"schemaVersion":1,"type":"word"}
```

Non-relation values live in `fields`; `links` holds ids, always as arrays, and every
id must exist.

## Verify it yourself

```sh
SAM_RESOURCES=$PWD/Sources/SAMCore/Resources ./target/release/sam --configcheck --plan Sources/SAMCore/Resources/presets/language --json
SAM_RESOURCES=$PWD/Sources/SAMCore/Resources ./target/release/sam --rendercheck --plan Sources/SAMCore/Resources/presets/language
SAM_RESOURCES=$PWD/Sources/SAMCore/Resources ./target/release/sam view today.screen --plan Sources/SAMCore/Resources/presets/language --json
```

## Sources

* **JLPT levels N5–N1** — the vocabulary of `language.level`
  (`https://www.jlpt.jp/e/about/levelsummary.html`, retrieved 2026-09-26): the test has
  five levels, N5 easiest and N1 hardest.
* Everything else here is structure or example data: the Japanese terms and readings
  are ordinary beginner items, and every target (`92` kana cards, `300` words) is a
  **planning target of this template**, not a claim about a syllabus, an exam or a
  course. No dates are asserted anywhere in the preset.
