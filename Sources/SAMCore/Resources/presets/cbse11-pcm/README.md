# CBSE Class 11 PCM — Physics, Chemistry, Mathematics

**Promise.** One place to run the CBSE Class 11 board year for Physics (042), Chemistry (043)
and Mathematics (041): every chapter named the way CBSE names it, the flip study loop with a
proof gate, numerical practice with a computed score, and spaced reviews at 1, 7 and 30 days.

**Label: Verified as of 2026-09-26.** The chapter and unit names, the unit marks and the
assessment structure quoted below come from the CBSE syllabus PDFs listed under *Sources*
(fetched and read on 2026-09-26). Everything else in the example records — the term start, the
week dates, the assessment dates, the planned weekly hours — is the plan author's illustrative
schedule for a template, **not** a CBSE calendar. The only CBSE-sourced dates are the tentative
periodic-test windows printed in the syllabus PDFs (PT-I July–August, PT-II November,
PT-III December–January); schools set the exact dates.

## What this preset ships

```
content/types.json       11 types — term, course, unit, topic, week, resource, assessment,
                         session, note, milestone, problemset
content/views.json       23 saved views; Today, Plan, Courses, Practice, Reviews, Progress,
                         Library and Notes are the eight navigation destinations
content/rules.json       the flip and progress pipelines, the fixed 1/7/30 scheduler,
                         three metrics, the plan timezone
content/shell.json       the eight navigation entries and the seed keybindings
content/appearance.json  theme pinned to cadence-light
content/records/*.jsonl  63 example records: one term, 3 courses, 9 chapters (3 per subject),
                         18 topics, 8 weeks, 10 resources, 3 assessments, 3 sessions,
                         2 notes, 3 monthly milestones, 3 practice sets
```

Method, as data:

- `topic` is trackable under the **flip** pipeline — `learned → proved → anchored`, complete
  only at `anchored`. A topic of kind `watch` cannot reach `proved` without at least **two
  problems solved** as evidence (`minProblems: 2`); a prompt alone is not evidence. `anchored`
  needs a logged review or an explicit anchor-skip reason.
- `problemset` is trackable under the **progress** pipeline, with `pct` as a formula field:
  `pct(solved, total)` — computed, never typed, and `null` while `total` is 0. The Practice
  screen shows the table with that column.
- Completed stages schedule reviews 1, 7 and 30 days out (`fixed` scheduler).
- `session` and `note` are marked `"private": true`, so they stay out of a shared profile.

About `course.credits`: this preset uses it as the **student's own planned weekly hours** for
that subject (Physics 6, Chemistry 5, Mathematics 7 — the Courses board groups by it). It is a
pacing weight, not a CBSE figure; no CBSE source assigns credits to Class 11 subjects.

## Start a plan from it

```
SAM plan.new --preset cbse11-pcm --plan <new-plan-directory> --json
SAM view today.screen --plan <new-plan-directory> --json
```

`plan.new` takes exactly one of `--plan <directory>` (a path you choose) or `--name <plan-name>`
(created in the platform data directory). The copy is a complete, writable plan — `README.md`
travels with it — and it is the preset's own content, never a live layer.

## Verified syllabus content used here

### Physics (042) — Class XI, theory 70 marks

| Unit | Chapters | Marks |
| --- | --- | --- |
| Unit I: Physical World and Measurement | Ch 1 Units and Measurements | 23 (Units I–III together) |
| Unit II: Kinematics | Ch 2 Motion in a Straight Line · Ch 3 Motion in a Plane | |
| Unit III: Laws of Motion | Ch 4 Laws of Motion | |
| Units IV–V | Ch 5 Work, Energy and Power · Ch 6 System of Particles and Rotational Motion | 17 |
| Unit VI: Gravitation | Ch 7 Gravitation | |
| Unit VII: Properties of Bulk Matter | Ch 8 Mechanical Properties of Solids · Ch 9 Mechanical Properties of Fluids · Ch 10 Thermal Properties of Matter | 20 |
| Unit VIII: Thermodynamics | Ch 11 Thermodynamics | |
| Unit IX: Behaviour of Perfect Gases and Kinetic Theory of Gases | Ch 12 Kinetic Theory | |
| Unit X: Oscillations and Waves | Ch 13 Oscillations · Ch 14 Waves | 10 |

Practical: 30 marks — two experiments (7+7), record 5, one activity 3, investigatory project 3,
viva 5.

### Chemistry (043) — Class XI, theory 70 marks

| Unit | Marks |
| --- | --- |
| Unit 1: Some Basic Concepts of Chemistry | 7 |
| Unit 2: Structure of Atom | 9 |
| Unit 3: Classification of Elements and Periodicity in Properties | 6 |
| Unit 4: Chemical Bonding and Molecular Structure | 7 |
| Unit 5: Chemical Thermodynamics | 9 |
| Unit 6: Equilibrium | 7 |
| Unit 7: Redox Reactions | 4 |
| Unit 8: Organic Chemistry: Some Basic Principles and Techniques | 11 |
| Unit 9: Hydrocarbons | 10 |

Practical: 30 marks — volumetric analysis 8, salt analysis 8, content-based experiment 6, project
work 4, class record and viva 4. Two further topics are in the syllabus but **assessed only
formatively**: s- and p-block elements, and the gaseous state.

### Mathematics (041) — Class XI, theory 80 marks

| Unit | Chapters | Marks |
| --- | --- | --- |
| Unit I: Sets and Functions | Sets · Relations & Functions · Trigonometric Functions | 23 |
| Unit II: Algebra | Complex Numbers and Quadratic Equations · Linear Inequalities · Permutations and Combinations · Binomial Theorem · Sequence and Series | 25 |
| Unit III: Coordinate Geometry | Straight Lines · Conic Sections · Introduction to Three-dimensional Geometry | 12 |
| Unit IV: Calculus | Limits and Derivatives | 08 |
| Unit V: Statistics and Probability | Statistics · Probability | 12 |

Internal assessment: 20 marks — periodic tests (best two of three) 10, mathematics activities 10.
CBSE notes there is no chapter-wise weightage and that all chapters are to be covered.

### The nine chapters this preset schedules

Physics: Units and Measurements (Ch 1) · Laws of Motion (Ch 4) · Work, Energy and Power (Ch 5).
Chemistry: Some Basic Concepts of Chemistry · Chemical Bonding and Molecular Structure ·
Equilibrium. Mathematics: Sets · Relations and Functions · Sequences and Series.

These are the first eight weeks of the plan, three chapters per subject; the full lists above are
the source of record. The `index` field carries the NCERT textbook chapter number (CBSE numbers
Mathematics chapters within each unit instead: its "Sequence and Series" is item 5 of Unit II,
and its "Relations & Functions" is item 2 of Unit I). The plan studies chapters in its own
order, which is why chapter numbers are not consecutive.

## Sources

Fetched and read on **2026-09-26**:

1. CBSE, *Physics (Subject Code 042), Classes XI–XII (2026-27)*, course structure and chapter
   list for Class XI — <https://cbseacademic.nic.in/web_material/CurriculumMain27/SecPart2/Physics_SecP2_2026-27.pdf>
2. CBSE, *Chemistry (Subject Code 043), Classes XI–XII (2026-27)*, course structure, unit marks
   and practical scheme — <https://cbseacademic.nic.in/web_material/CurriculumMain27/SecPart2/Chemistry_SecP2_2026-27.pdf>
3. CBSE, *Mathematics (Subject Code 041), Classes XI–XII (2026-27)*, course structure, unit
   marks, internal assessment and periodic-test windows —
   <https://cbseacademic.nic.in/web_material/CurriculumMain27/SecPart2/Maths_SecP2_2026-27.pdf>
4. NCERT, *Textbooks PDF (I–XII)*, the download entry point for the prescribed books
   (Physics Part I/II, Chemistry Part I/II, Mathematics) —
   <https://ncert.nic.in/textbook.php>

The NCERT links cited in the CBSE Chemistry syllabus under a `ncert.nic.in/exemplar/…` path
returned 404 on 2026-09-26 and are therefore **not** shipped in this preset's Library.
