# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html/).

## [Unreleased]

## [0.2.0] - 2026-07-26

### Added

- Added TOML configuration for every non-text-input action, including multiple
  aliases per action and startup validation for invalid or conflicting chords.
- Added global, current-directory, subtree, and Git-root history scopes with
  configurable directory matching semantics.
- Added chronological context review around a selected command, including
  configurable expansion and shrink controls.
- Added toggleable date and working-directory metadata columns.
- Added deterministic search-work instrumentation and a reproducible
  100,000-row release benchmark.

### Changed

- Replaced full-history rescanning and full-result sorting on each keypress with
  incremental candidate narrowing, cached query-prefix layers, and bounded
  top-K ranking while retaining Skim's fzf-style score.
- Split the monolithic core into history, scope, search, application state,
  configuration, keymap, rendering, and terminal-runtime modules.
- Changed the default metadata shortcut from `ctrl-m` to `alt-m`, because many
  terminals encode Ctrl-M identically to Enter. Existing TOML files may still
  select `ctrl-m` where the terminal can distinguish it.
- Stopped redrawing the terminal while idle and removed command/PWD string
  cloning from the render path.

### Fixed

- Restored raw mode, alternate-screen state, and cursor visibility on normal
  exits and recoverable terminal errors.
- Rejected duplicate key assignments instead of resolving them ambiguously.
- Made the documented `page-up` and `page-down` key aliases parse correctly.
- Prevented application context state from representing contradictory
  anchor/radius/mode combinations.

### Performance

- A reference release benchmark over a ten-query sequence and 100,000 history
  rows improved from 547 ms to 341 ms (1.61×). After candidate narrowing,
  subsequent characters scanned 10,000 rows instead of 100,000.

## [0.1.0] - 2026-05-08

### Added

- Added the initial Rust/Ratatui picker for an Atuin-compatible SQLite history
  database.
- Added fuzzy command search, same-directory filtering, command selection, and
  shell-friendly stdout output.
- Added CI, tagged GitHub releases, and Linux, macOS, and Windows archives.

### Compatibility

- The repository's first published tag was named `v0.0.1`, while the crate
  metadata at that revision already reported version `0.1.0`.
