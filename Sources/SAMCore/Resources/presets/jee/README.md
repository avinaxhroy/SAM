# JEE - a two-year JEE (Main + Advanced) preparation plan

**One sentence:** a complete two-year JEE (Main + Advanced) plan for Physics, Chemistry and
Mathematics, where every chapter of the official syllabus is a unit, every sitting is a
logged session, and progress is counted in questions solved rather than hours watched.

**Label: Structural template - no date-specific content.**

No examination date, cut-off, rank band, seat number or chapter weightage is asserted
anywhere in this preset, and no calendar date is shipped in any example record:

- `week.start` / `week.end`, `assessment.date` and `problemset.lastAttempt` are declared
  and deliberately empty - fill them with your own dates.
- The plan is anchored on its own `week.index` (1 ... 104) and `week.phase`
  (Foundation -> Build -> Practice -> Revision), so every screen works before a single
  calendar date exists.
- The three `milestone` targets (250 marks, 95 percentile, rank 5000) are examples of a
  student's own goal, not published cut-offs, qualifying bands or ranks. The schema labels
  the field "Target (your own number - the shipped value is an example)".
- `resource.url` and `problemset.source` carry no value except the one link below, whose
  address was read from the official site.

## Verified as of 2026-09-26

The three subjects and the chapter-level `unit` records use the unit names of the official
JEE (Main) Paper 1 (B.E./B.Tech.) syllabus: Mathematics units 1-14, Physics units 1-20 and
Chemistry units 1-20 (physical, inorganic and organic). Each unit's `index` is its number in
that official syllabus. Each unit's `stage` ("Class 11" / "Class 12") follows the NCERT
textbook that teaches that unit.

## What the preset is

- **Types (10):** `course` (the subject), `unit` (a chapter), `topic` (trackable, `flip`),
  `week`, `resource`, `assessment` (weekly test / part test / full mock), `session`, `note`,
  `milestone`, and `problemset` (trackable, `progress`; its `pct` is a formula and is never
  stored). `session` and `note` are private, so a shared profile leaves them behind.
- **Navigation:** Today, Plan (units as a timeline by subject), Subjects (chapters as a
  board by subject), Practice (question banks as a table), Mocks (assessments as a
  calendar), Reviews and Progress (panels), Library, Notes.
- **Method:** `flip` is learned -> proved -> anchored, where `proved` needs two solved
  problems for a `watch` topic and anchored work returns on day 1, 7 and 30
  (`scheduler.fixed`). `problemset` follows `progress`, whose fraction is its computed
  `pct`, so the table can never drift from `solved` / `total`.
- **Metrics:** questions solved, minutes logged, topics open.

## Sources

1. *Syllabus for JEE (Main) - 2026, Paper 1 (B.E./B.Tech.): Mathematics, Physics and
   Chemistry* - National Testing Agency. Retrieved 2026-09-26.
   <https://cdnbbsr.s3waas.gov.in/s3f8e59f4b2fe7c5705bf878bbd494ccdf/uploads/2025/10/202510311323551056.pdf>
   (linked from the official syllabus page <https://jeemain.nta.nic.in/document/syllabus-2026/>)
2. *Information Bulletin - 2026, Joint Entrance Examination (Main)* - National Testing
   Agency. Retrieved 2026-09-26.
   <https://cdnbbsr.s3waas.gov.in/s3f8e59f4b2fe7c5705bf878bbd494ccdf/uploads/2025/10/202510311145384616.pdf>
   (JEE (Main) is conducted in two sessions and is the eligibility test for JEE (Advanced).)
3. *Textbooks, Classes XI and XII* - National Council of Educational Research and Training.
   Retrieved 2026-09-26. <https://ncert.nic.in/textbook.php>
   (the Class 11 / Class 12 split recorded in each unit's `stage`)

Nothing else here is a factual claim about the examination. The study structure, the week
phases and the example records are this template's own design.
