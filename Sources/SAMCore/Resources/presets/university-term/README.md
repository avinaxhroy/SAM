# University Term — preset

**Promise.** A generic, ready-to-edit one-term study plan for a Computer Science
degree: four courses, their modules and topics, a learn → prove → anchor queue,
assessments, projects and milestones — switchable to a real degree without
touching code.

**Label: Structural template — no date-specific content.**

## What this preset is

This is a **structural template**, not a verified academic calendar. It ships a
complete, writable plan tree — types, views, shell, rules, appearance and example
records — for a term that is deliberately generic.

- **It is date-free.** The schema declares the date fields a real term needs
  (`program.start`, `term.start`, `assessment.date`, `project.deadline`), but no
  example record carries a date value. The plan is laid out by **week number**
  (weeks 1–8, modules `Weeks 1–3 · …`), so it works for any term you start.
  Nothing here claims that any real course runs on any real day.
- **It is institute-free.** No institution, faculty, board or exam body is
  named. "Example University" is a placeholder in the one `program` record; the
  course codes and instructor names are illustrative labels, not real people or
  departments.
- **The topic content is generic curriculum vocabulary** — "Asymptotic
  notation", "Processes and scheduling", "Estimation and inference" — the
  recurring chapter headings shared across CS degrees, not one institution's
  syllabus.

Fill in the dates, the institution and any course-specific detail inside the app
or through `SAM apply`; the structure is already in place.

## What is inside

| Kind | What it models | Records |
| --- | --- | --- |
| `program` | institute, degree, start, semesters | 1 |
| `term` | label, start, weeks, credits (child of `program`) | 1 |
| `course` | code, name, short, credits, instructor, note | 4 |
| `unit` | a module, numbered by week range (child of `course`) | 10 |
| `topic` | trackable, pipeline `flip` (learned → proved → anchored) | 16 |
| `week` | week index and label (child of `term`) | 8 |
| `resource` | video/pdf/article/book/tool with a URL | 6 |
| `assessment` | quiz, assignment, midterm, final | 4 |
| `session` | private study log: week, course, minutes, note | 4 |
| `note` | private written notes tied to a course | 3 |
| `project` | course project with an optional deadline | 2 |
| `milestone` | a dated-by-week checkpoint | 3 |
| `problemset` | **extension type** (SAM_PLAN §3.3): chapter, difficulty, attempted/solved, `pct` formula, pipeline `progress` | 4 |

**Views.** Today, Plan (timeline by module), Courses (board by credits),
Assessments (calendar), Projects (table), Reviews (panel), Progress (panel),
Library, Notes — every type has at least one view, and the shell navigation
points only at declared views.

**Rules.** The `flip` pipeline with its proof gate (two problems solved before
`proved`), a `progress` pipeline for problem sets, the `fixed` scheduler
(1 / 7 / 30 days), and the metrics *planned minutes*, *minutes logged* and *open
topics*.

## Sources

No factual or date-specific claim is made by this preset, so there is nothing to
cite against an official source. The structures it uses are specified by the
project itself:

- `SAM_PLAN.md` §3.3 — the `problemset` extension type and its `pct(solved, total)` formula.
- `SAM_PLAN.md` §3.4 — the closed field-type vocabulary.
- `SAM_PLAN.md` §3.5 — the `flip` / `progress` pipelines and the `fixed` scheduler.
- `SAM_PLAN.md` §3.6 — layouts, filters and screen blocks.

Course and module names are generic computer-science curriculum labels, chosen
for illustration only and not attributed to any institution or published
syllabus.
