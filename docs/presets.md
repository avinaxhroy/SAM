# Presets and curriculum templates

SAM includes bundled curriculum presets stored under `Sources/SAMCore/Resources/presets/`. Each preset represents a complete, self-contained configuration defining entity kinds, views, review rules, navigation, and initial records.

---

## Preset structure

Every preset directory contains these core files:

```
presets/<preset-name>/
├── README.md               # Summary of the preset and its intended use case
├── content/
│   ├── types.json          # Entity schemas (kinds, fields, relations)
│   ├── views.json          # Screen layouts, block arrangements, and columns
│   ├── rules.json          # Schedulers, progression pipelines, and metrics
│   ├── shell.json          # Navigation rail destinations
│   ├── appearance.json     # Theme configuration and token overrides
│   └── records/            # Pre-populated JSONL record files (if any)
└── state/
    └── state.json          # Initial progress tracking state
```

When creating a new plan via `sam plan.new --preset <name>`, the engine copies these documents into the destination directory. A plan is an independent copy on disk; modifying a plan never alters the bundled template.

---

## Shipped presets

### 1. `blank`

The blank preset contains the full core schema but zero record files.

- **Primary use**: First-time application onboarding and fresh start workflows.
- **Characteristics**: Includes all 12 core types (`program`, `term`, `course`, `unit`, `topic`, `week`, `resource`, `problemset`, `session`, `review`, `assessment`, `note`) with empty `content/records/`. Today's view presents the empty state and invites adding your first course.

### 2. `seed`

The structural reference template used by SAM's internal test suites.

- **Primary use**: Learning SAM's data model and validating engine compliance.
- **Characteristics**: Ships a balanced set of sample records illustrating how topics link to units, courses, and resources. Demonstrates the `flip` pipeline, fixed review ladders, and private record configurations.

### 3. `university-term`

Modeled after a traditional higher-education academic semester.

- **Primary use**: University students balancing multiple courses with concurrent lectures, assignments, and exams.
- **Hierarchy**: `program` -> `term` -> `course` -> `unit` -> `topic`.
- **Key fields**: Course credit counts, instructor contacts, office hour schedules, lecture delivery modes, and exam milestone dates.

### 4. `jee`

Designed for intensive, two-year preparation for the Joint Entrance Examination (Engineering).

- **Primary use**: High-volume problem practice across Physics, Chemistry, and Mathematics.
- **Characteristics**: Emphasizes the `problemset` kind, problem-solving speed tracking, accuracy ratios, and unit-by-unit mastery thresholds.

### 5. `neet`

Configured for pre-medical entrance examination preparation.

- **Primary use**: High-retention study across Physics, Chemistry, Botany, and Zoology.
- **Characteristics**: Pairs with the FSRS spaced repetition pipeline for high-volume conceptual recall, NCERT chapter mappings, and revision cycles.

### 6. `cbse11-pcm`

Aligned with the standard Class 11 national board curriculum for secondary education.

- **Primary use**: High school students tracking standard textbook chapters and laboratory exercises.
- **Characteristics**: Pre-structured chapters for CBSE Physics, Chemistry, and Mathematics with syllabus milestones.

### 7. `self-study`

Structured for self-taught programmers, researchers, and independent learners.

- **Primary use**: Following online lecture series (such as MIT OpenCourseWare), reading technical books, and building projects.
- **Characteristics**: Focuses on milestones, estimated vs. actual study minutes, and practical project deliverables rather than institutional term dates.

### 8. `language`

Tailored for foreign language acquisition and immersion.

- **Primary use**: Balancing vocabulary drills, grammar study, reading immersion, and listening practice.
- **Characteristics**: Uses customized entity kinds for grammar rules, vocabulary units, listening logs, and conversation practice sessions, backed by spaced repetition intervals.

---

## Authoring custom presets

You can turn any customized study plan into a reusable preset or share it with others.

### Step 1: Clean personal logs

If you want to share a curriculum without including your private notes or review history:
1. Ensure personal kinds declare `"private": true` in `content/types.json` (such as `note` or `session`).
2. Run `profile.export`:
   ```bash
   sam profile.export --out my-template.samprofile --plan ./my-plan
   ```
   By default, `profile.export` strips private entity kinds, leaving clean structural courses, units, and topics.

### Step 2: Distribute or load

Other students can import your exported profile to create their own plan:
```bash
sam profile.import --file my-template.samprofile --name "Discrete Math Plan"
```

To install a custom preset permanently into the application bundle, place the directory under `Sources/SAMCore/Resources/presets/<custom-name>/` and rebuild the engine.
