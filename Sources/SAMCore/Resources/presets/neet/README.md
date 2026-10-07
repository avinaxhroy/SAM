# NEET — Biology, Physics, Chemistry

A six-week, chapter-by-chapter NEET (UG) preparation plan: three subjects, one
chapter spine each, a flip-loop topic queue, problem sets with a derived
percentage, chapter tests and full mocks, and derived progress metrics — so a
student can start from a working plan instead of an empty schema.

**Label: Verified as of 2026-09-26** for the syllabus spine (§ *Syllabus
sources* below). The **timeline is structural**: `week.start`/`week.end`,
`unit.start`/`unit.end` and the assessment dates are an illustrative
preparation window (Mon 2026-10-05 → Sun 2026-11-15), not an announced exam
date. No exam date, cut-off, seat number or weightage appears anywhere in this
preset, because none was verified.

## What ships

```
content/types.json          10 types
content/views.json          27 views — 9 of them the shell's navigation screens
content/rules.json          2 pipelines, the fixed scheduler, 5 metrics
content/shell.json          navigation + the standard keybindings
content/appearance.json     theme cadence-light
content/records/*.jsonl     61 records, one canonical record per line
README.md                   this file
```

### Types

| Type | What it is | Notes |
| --- | --- | --- |
| `course` | Biology, Physics, Chemistry | `subject` is the select that groups the Subjects board |
| `unit` | one chapter / syllabus unit | `parent: course`; `syllabusUnit` names the official unit |
| `topic` | one sitting of work | trackable, pipeline `flip` — learned → proved → anchored |
| `problemset` | an MCQ bank or a numericals set | trackable, pipeline `progress`, `pct` is a formula |
| `week` | one week of the window | |
| `resource` | a textbook, a PDF, the official portal | |
| `assessment` | chapter test or full mock | `course` is optional, so a full mock has none |
| `session` | time actually spent | `private: true` |
| `note` | a recall sheet | `private: true` |
| `milestone` | a month-level target | |

`session` and `note` are marked `private: true`: they are personal, and a
shared profile excludes them.

### The flip loop, as data

`topic` follows `flip`: `learned → proved → anchored`, with
`gates.proved.require = learned` and `gates.anchored.require = proved`. The proof
gate applies to `watch` topics and asks for `minProblems: 2` — a prompt is not
evidence. `anchorSkip` is allowed but needs a written reason. `completeWhen` is
`anchored`, so an open topic is anything that has not been anchored, and that is
exactly what the Today and Reviews screens filter on (`!complete()`).

`problemset` follows `progress`: its fraction comes from the `pct` formula field
(`pct(solved, total)`), never from a typed-in number. `pct` is derived and is
never persisted in `fields`.

### Views

| Screen | What it shows |
| --- | --- |
| Today | open topics, grouped by kind, oldest week first; planned minutes |
| Plan | the chapter timeline grouped by subject, the mock calendar, weeks, milestones, every topic |
| Subjects | courses as a board grouped by `subject`, and topics by subject |
| Practice | the problem-set table (chapter, difficulty, total, attempted, solved, pct, source, last attempt), a "not finished" view, and a board by difficulty |
| Mocks | the assessment calendar and the assessment table |
| Reviews | the flip queue with the `reviews` panel |
| Progress | minutes per day for the last 14 days, questions solved, sets not finished, the sessions table, with the `progress` panel |
| Library | resources by kind and as a table |
| Notes | every note |

Every type has at least one view whose `type` names it, and every view listed in
`shell.json` resolves.

### Metrics

`questions.practised` (sum of `solved` over `practice.sets`), `sets.pending`
(count over `practice.pending`), `topics.open`,
`minutes.logged`, `tests.written`. The first one is the "questions practised"
metric the Progress panel shows.

### Records

3 courses, 9 units (3 per subject), 15 topics (5 per subject), 8 problem sets,
4 assessments (2 chapter tests + 2 full mocks), 6 weeks, 5 resources, 5
sessions, 3 notes, 3 milestones.

`unit.index` is this plan's own ordering (1, 2, 3 within each subject), **not**
the unit number in the official syllabus. The official number is carried
verbatim in `unit.syllabusUnit`, e.g. `Physics Unit 12` for Current Electricity.

## Syllabus sources

All chapter and unit names in `content/records/unit.jsonl` and the unit lists
below are taken verbatim from the syllabus the National Medical Commission's
Under Graduate Medical Education Board finalised for NEET (UG)-2026 and the
National Testing Agency published on the official NEET portal. **Retrieved
2026-09-26.**

1. NMC UGMEB, *Syllabus for NEET (UG)-2026* (public notice dated 22 December
   2025), published on the official NEET (UG) portal under Public Notices:
   <https://cdnbbsr.s3waas.gov.in/s37bc1ec1d9c3426357e69acd5bf320061/uploads/2026/01/202601081066816297.pdf>
2. The same notice on the NTA site:
   <https://www.nta.ac.in/Download/Notice/Notice_20260108180635.pdf>
3. National Eligibility cum Entrance Test (UG), official portal (subject-wise
   syllabus notice, and the entry point for the current year's information
   bulletin): <https://neet.nta.nic.in/>
4. NCERT, *Textbooks PDF (I–XII)* — the Class 11 and Class 12 textbooks the
   syllabus is drawn from: <https://ncert.nic.in/textbook.php>

### The official unit spine (for extending this plan)

The preset ships three representative units per subject. The full spine, as
published in source 1, is:

**Physics** — 1 Physics and Measurement · 2 Kinematics · 3 Laws of Motion ·
4 Work, Energy, and Power · 5 Rotational Motion · 6 Gravitation · 7 Properties
of Solids and Liquids · 8 Thermodynamics · 9 Kinetic Theory of Gases ·
10 Oscillations and Waves · 11 Electrostatics · 12 Current Electricity ·
13 Magnetic Effects of Current and Magnetism · 14 Electromagnetic Induction and
Alternating Currents · 15 Electromagnetic Waves · 16 Optics · 17 Dual Nature of
Matter and Radiation · 18 Atoms and Nuclei · 19 Electronic Devices ·
20 Experimental Skills.

**Chemistry** — Physical: 1 Some Basic Concepts in Chemistry · 2 Atomic
Structure · 3 Chemical Bonding and Molecular Structure · 4 Chemical
Thermodynamics · 5 Solutions · 6 Equilibrium · 7 Redox Reactions and
Electrochemistry · 8 Chemical Kinetics. Inorganic: 9 Classification of Elements
and Periodicity in Properties · 10 p-Block Elements · 11 d- and f-Block
Elements · 12 Co-ordination Compounds. Organic: 13 Purification and
Characterisation of Organic Compounds · 14 Some Basic Principles of Organic
Chemistry · 15 Hydrocarbons · 16 Organic Compounds Containing Halogens ·
17 Organic Compounds Containing Oxygen · 18 Organic Compounds Containing
Nitrogen · 19 Biomolecules · 20 Principles Related to Practical Chemistry.

**Biology** — 1 Diversity in Living World · 2 Structural Organisation in
Animals and Plants · 3 Cell Structure and Function · 4 Plant Physiology ·
5 Human Physiology · 6 Reproduction · 7 Genetics and Evolution · 8 Biology and
Human Welfare · 9 Biotechnology and Its Applications · 10 Ecology and
Environment.

Add a unit by appending one line to `content/records/unit.jsonl` with the next
`unit.index` for its subject and the official unit in `syllabusUnit`.

## What this preset does not claim

No exam date, cut-off, seat number, question count, marking scheme or
subject-wise weightage. At the retrieval date above, the portal carried no
announced date for the next NEET (UG) cycle, so the six-week window is a
planning device and nothing more. Verify the current cycle against source 3
before relying on any date.
