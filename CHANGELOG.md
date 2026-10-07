# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0-beta] - 2026-10-07

### Added
- **Today Dashboard**: Objective focus card, titlebar session timer, and dynamic queue grouping (`late`, `due`, `next`, `stale`).
- **Curriculum Planning**: Course units, weekly syllabus breakdown, and topic tracking with estimates and prerequisite links.
- **Practice & Proof Gates**: Multi-stage learning progression (`unseen` -> `learning` -> `learned` -> `proved`) with required practice problem thresholds.
- **Spaced Repetition**: Review queue with keyboard-driven recall grading (`Space` to reveal, `1` Again, `2` Good) supporting Fixed ladders, SM-2, and FSRS-6 algorithms.
- **Progress Tracking**: Denominator-backed metrics replacing arbitrary streaks, tracking revised topics, solved problems, and study minutes.
- **Headless CLI (`sam`)**: Full terminal automation surface with JSONL batch mutations (`sam apply`), dry-run validation, and schema introspection.
- **Multi-Platform App**: Tauri v2 desktop application pairing Rust core engine with Svelte 5 frontend across macOS, Linux, and Windows.
- **Curriculum Presets**: 8 built-in templates (`blank`, `university-term`, `self-study`, `jee`, `neet`, `cbse11-pcm`, `language`, `seed`).
