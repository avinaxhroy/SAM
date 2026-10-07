# The seed — SAM's schema-only starter

**Label: structural template — no date-specific content, no pre-authored term.**

This is the preset every plan is created from when no other is named (`SAM
plan.new --name <plan>`, or the app's first-launch flow). It ships the §3.2
starter schema — the twelve kinds, their views, the flip-proof study method, the
fixed review ladder and the shell's destinations — plus a handful of example
records that exist to show what each kind is for. It is not a term: the student's
own term is authored inside the app, or by an agent through `SAM apply` (§4.7).

## What it demonstrates

- **The study method is data.** `topic.pipeline = "flip"` — switch it to `check`
  with `SAM type.setPipeline` (or the Reviews panel) and the behaviour changes
  with no code.
- **Derived, never persisted.** Formula and progress values come from the
  pipeline and the evaluator; records hold only fields and links.
- **Private kinds.** `note` and `session` declare `"private": true`, so a shared
  `*.samprofile` leaves their records out by default.
- **Views are first-class.** Every kind has at least one view; the shell
  navigation points at screens, and a type with no view is a lint finding (§3.6).

## Sources

No factual or date-specific claim is made: the structures are specified by
`SAM_PLAN.md` §3.1–§3.6. The palette it renders with is `design/tokens.json`
(§1.4.A), read as data and never re-typed.
