# Changelog

All notable changes to this project are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html/).

## [Unreleased]

### Added

- Added typed `date`/`pwd`/`exit`/`duration` presentation columns with compact relative-age defaults and runtime column visibility shortcuts.
- Added the built-in keyboard-first Actions menu with text-only append/compose actions and innermost Escape cancellation.
- Added explicit fuzzy, quoted-literal, and `/regex/` query stages with `#` candidate-stream subfilters and malformed-query diagnostics.
- Added bounded SQLite metadata observation for live history refresh while preserving the active query and selected history identity when possible.
- Added an adaptive Atuin-inspired TUI with `Search`/`Inspect` tabs, a
  version/help/history-count header, aligned execution-duration and relative-age
  columns, scope-aware query badges, and a selected-command preview.
- Added cursor-aware mid-query editing with Unicode-safe left/right, home/end,
  forward delete, previous-word delete, clear-query, horizontal scrolling, and
  a real terminal cursor.
- Added bracketed-paste handling that normalizes multiline clipboard text into
  the single-line search field.
- Added opt-in NUL-terminated selected-command output with shell bindings that
  preserve trailing newline bytes in Bash, Zsh, and Fish editing buffers.
- Added XDG-aware discovery of Atuin's standard history database when neither an
  explicit database nor an existing compatibility `./history.db` is selected.
- Added robustness coverage for long and Unicode commands, shell metacharacter
  queries, high-cardinality fuzzy matches, directory boundaries, chronological
  context across sessions, and shell-safe selected-command output.

### Changed

- Replaced the fixed three-panel layout and four-line shortcut footer with
  full, compact, and ultra-compact projections that preserve usability in
  small terminal windows.
- Prioritized command text over optional row metadata, left-truncated long
  paths, made header help width-aware, reduced query-panel box noise, and added
  actionable empty states.
- Added non-color `✓`/`×` status indicators and a safe multiline preview with
  selected-command exit, duration, and working-directory context.
- Updated Bash, Zsh, and Fish Ctrl-R integrations to use NUL-delimited command
  transfer without shell command-substitution newline trimming, and clarified
  lexical symlink/mount-point scope semantics.
- Kept mid-string query insertions incremental by narrowing from the previous
  fuzzy-match candidate set instead of rescanning the whole active scope.
- Kept legacy `history_columns` configuration and shell-facing output semantics
  while introducing the typed presentation layer.
- Removed redundant SQLite-side history ordering so large database loads perform
  the deterministic timestamp/ID sort only once in memory.

### Fixed

- Matched Atuin's runtime CWD identity by preferring the shell-provided `PWD`
  over the physical process current directory, fixing same-directory filtering
  when history was recorded through a logical symlink path while retaining a
  fallback when `PWD` is unset.

## [0.2.2] - 2026-07-26

### Added

- Added compatibility loading for older Atuin-style databases that predate the
  optional `deleted_at` soft-delete column.
- Added explicit schema diagnostics for missing tables, missing required
  columns, and incompatible SQLite row values.
- Added boundary coverage for duplicate IDs, equal timestamps, multiline CLI
  output, tiny terminal dimensions, Unicode paths and shortcuts, and bidi
  display controls.

### Changed

- Preserved active fuzzy queries when callers switch a `SearchEngine` scope.
- Anchored interactive context review by immutable history index rather than
  externally supplied history ID.
- Ordered equal-timestamp history rows deterministically by ID.
- Added a one-second SQLite busy timeout to tolerate short concurrent writer
  transactions.
- Made selected-command stdout writes fallible instead of using panic-prone
  print macros.

### Fixed

- Prevented duplicate history IDs from moving context review to a different
  command; ID-based public context lookup now fails closed when an ID is
  ambiguous.
- Prevented unsupported Super, Hyper, and Meta event modifiers from being
  silently stripped and triggering unrelated plain-key bindings.
- Fixed modified non-ASCII shortcuts such as `ctrl-ä` and `shift-ö`.
- Fixed case-insensitive matching for non-ASCII Windows path components.
- Sanitized bidi and zero-width format controls in commands, paths, and the
  typed query before terminal rendering.
- Preserved real scan statistics after scope changes instead of overwriting
  them with a false cache-hit report.

### Performance

- The final 100,000-row benchmark completed the ten-query sequence in 355 ms
  with incremental filtering versus 563 ms with full rescans (1.59× faster).

## [0.2.1] - 2026-07-26

### Added

- Added a standard `--version` CLI flag.
- Added differential search-correctness tests, command-substitution PTY
  coverage, and boundary tests for empty stores, unlimited result counts, and
  maximum context radii.

### Changed

- Rendered the interactive terminal UI on stderr so stdout contains only the
  selected command and remains safe for shell command substitution.
- Bounded cached query-prefix layers to prevent memory growth during unusually
  long search queries.
- Split the shortcut footer into complete action groups so configurable scope,
  editing, context, navigation, and completion bindings remain visible.
- Made explicitly requested config files required while keeping the default
  config path optional.
- Added repository and readme metadata to the Cargo package manifest.

### Fixed

- Fixed subtree matching at the Unix filesystem root.
- Fixed Windows path matching across drive-letter case and slash styles while
  preserving literal backslashes and case sensitivity in Unix paths.
- Prevented unavailable pwd and Git-root scopes from silently exposing broader
  history results.
- Restored the original selected command when leaving chronological context
  review.
- Sanitized command and working-directory control characters before rendering
  them into single-line terminal rows.
- Rejected unknown TOML fields and unreachable raw control-character bindings.
- Treated `BackTab` consistently as `shift-tab`, even when the terminal omits
  the explicit Shift modifier.
- Fixed timestamp magnitude detection for valid future Unix-second values.
- Prevented integer overflow in maximum-radius context windows and maximum
  search-result limits.
- Avoided an invalid list selection when a query has no matches.

### Performance

- The 100,000-row release benchmark retained the incremental filtering gain:
  670 ms for full rescans versus 413 ms for the bounded incremental engine
  (1.62× faster for the ten-query sequence).

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
