# cmdscope

`cmdscope` is a Rust terminal UI for interactively searching an Atuin-style shell history SQLite database.
It is designed for shell `Ctrl-R` usage: fuzzy-filter commands, optionally restrict results to the current directory, inspect time-neighbor context around a result, then print the selected command for shell insertion.

## Features

- Layered architecture with independent history, scope, search, model, keymap, and rendering modules.
- Fzf-compatible scoring through `skim`, combined with incremental candidate narrowing and prefix caching.
- Bounded top-K ranking instead of sorting every fuzzy match after each keypress.
- Allocation-free key dispatch from a startup-validated TOML keymap.
- Adaptive `ratatui` + `crossterm` TUI with Atuin-inspired search chrome,
  relative timing, an inspection tab, responsive information density, and a
  real cursor-aware query editor.
- Atuin `history` table support, including `cwd` and `deleted_at` filtering.
- Same-directory mode for context-sensitive command recall.
- Context mode that ignores the active filter string and shows commands around the selected result in time order.
- Visible selected-row marker plus toggleable history metadata columns such as date and pwd.

## Database schema

The app expects an Atuin-like table:

    create table history (
      id text primary key,
      timestamp integer not null,
      duration integer not null,
      exit integer not null,
      command text not null,
      cwd text not null,
      session text not null,
      hostname text not null,
      deleted_at integer,
      author text,
      intent text
    );

The example project data lives at `./history.db`, but local `.db` files are ignored by git.

## Quick start

Build the binary:

    cargo build

Run against a local history database:

    cargo run -- --db ./history.db

Smoke-test DB loading without entering the TUI:

    cargo run -- --db ./history.db --print-first

Print the installed version:

    cmdscope --version

Use an Atuin database directly:

    CMDSCOPE_DB="$HOME/.local/share/atuin/history.db" cmdscope

Use a custom config file:

    cmdscope --config ./examples.config.toml --db ./history.db

## TUI layout

The full layout follows the compact information hierarchy of Atuin's shell
search without depending on Atuin's application code:

| Region | Contents |
| --- | --- |
| Header | `cmdscope` version, responsive key hints, shown/total history count |
| Tabs | `Search` and `Inspect`; context review activates `Inspect` |
| History | selectable rows with accessible status, execution duration, relative age, command, and width-aware metadata |
| Query | fixed-width scope badge (`GLOBAL`, `PWD:*`, `GIT-ROOT`, or `INSPECT ±N`) and a horizontally scrolling editor |
| Preview | safe multiline selected-command preview with exit, duration, and pwd context |

History rows render as:

    > ✓   123ms    19h ago cargo test  2023-11-14  /home/me/project

Successful rows use `✓`; failed rows use `×`, so command status is not encoded
by color alone. Duration remains green for exit status `0` and red for a
non-zero status. The selected row has a `>` marker plus reverse-video styling
so it remains visible across terminal themes.

Rows reserve space for the command first. Optional date and pwd metadata is
elided as the terminal narrows, and long paths are left-truncated so the most
specific directory components remain visible. The header follows the same
priority order: key hints disappear before identity and result counts.

The renderer automatically removes the preview and borders on shorter terminal
windows. In ultra-compact windows it keeps only the history list and scope/query
line, preserving useful interaction rather than overflowing fixed panels. The
query line avoids an extra border, reducing visual box noise and returning more
rows to the history list.

The query editor supports mid-string Unicode editing, an actual terminal
cursor, horizontal scrolling with ellipsis markers, bracketed paste, word
deletion, and clear-to-empty. Pasted newlines and tabs become spaces so the
single-line search field remains predictable.

Metadata is visible by default and can be toggled at runtime with `toggle_metadata` (`alt-m` by default). The configured metadata column order is preserved.

Commands and working directories containing tabs, newlines, escape bytes, or
other control characters are rendered with safe visible symbols so one history
entry cannot corrupt adjacent TUI rows. Bidi overrides, directional isolates,
zero-width format characters, and the typed query receive the same treatment.

## Key bindings

| Key | Action |
| --- | --- |
| text input | update fuzzy filter |
| Backspace | remove the character before the query cursor |
| Delete / Ctrl-D | remove the character under the query cursor |
| Left / Ctrl-B | move the query cursor left |
| Right / Ctrl-F | move the query cursor right |
| Home / Ctrl-A | move to the beginning of the query |
| End / Ctrl-E | move to the end of the query |
| Ctrl-W | remove the previous query word |
| Ctrl-U | clear the query |
| Up / Ctrl-K | select previous result |
| Down / Ctrl-N | select next result |
| Tab | toggle global vs same-pwd filter |
| Ctrl-G | show global history |
| Ctrl-P | show pwd history |
| Ctrl-R | show Git-root history |
| Ctrl-S | toggle exact vs subtree pwd matching |
| Alt-M | toggle history metadata columns |
| Ctrl-O | toggle time-neighbor context for the selected command |
| Alt-] | expand context radius |
| Alt-[ | shrink context radius |
| Enter | accept selected command and print it to stdout |
| Esc / Ctrl-C | quit without selecting |

Every non-text-input action is configurable in TOML. A binding accepts either one token or an array of aliases. Invalid tokens and bindings assigned to multiple actions fail during startup instead of producing ambiguous runtime behavior.

Modified Unicode characters are supported, for example `ctrl-ä` and
`shift-ö`. Super, Hyper, and protocol-level Meta event modifiers are not
configurable and are rejected rather than being mistaken for plain keys.

## Shell integration

`cmdscope` writes the full-screen UI to stderr and reserves stdout exclusively
for the accepted command. The command is emitted as history text, not shell-
escaped or evaluated, so quotes, variables, pipes, semicolons, glob characters,
and internal newlines reach the shell editing buffer unchanged. Bash/Zsh
command substitution (and Fish `string collect` by default) strips trailing
newlines, including cmdscope's record separator; a history entry whose command
itself ends with newline bytes cannot preserve those trailing bytes through
these bindings.

Zsh/ZLE:

    cmdscope-widget() {
      local selected
      selected="$(CMDSCOPE_DB="$HOME/.local/share/atuin/history.db" cmdscope)" || return
      [[ -n "$selected" ]] || return
      BUFFER="$selected"
      CURSOR=${#BUFFER}
      zle redisplay
    }
    zle -N cmdscope-widget
    bindkey '^R' cmdscope-widget

Bash/readline:

    cmdscope-widget() {
      local selected
      selected="$(CMDSCOPE_DB="$HOME/.local/share/atuin/history.db" cmdscope)" || return
      [[ -n "$selected" ]] || return
      READLINE_LINE="$selected"
      READLINE_POINT=${#READLINE_LINE}
    }
    bind -x '"\C-r":cmdscope-widget'

Fish:

    function cmdscope-widget
        set -l selected (env CMDSCOPE_DB="$HOME/.local/share/atuin/history.db" cmdscope | string collect)
        or return
        test -n "$selected"; or return
        commandline --replace "$selected"
        commandline -f repaint
    end
    bind \cr cmdscope-widget

Fish's `string collect` is important for multiline history entries because an
ordinary unquoted Fish command substitution splits output at newlines.

## Development

Recommended checks before committing:

    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo test --all-targets
    cargo doc --no-deps

The tests use TDD-friendly pure model/store behavior so the interactive terminal loop stays thin.

## Config

`cmdscope` resolves configuration in this order:

1. `--config <path>`
2. `CMDSCOPE_CONFIG=<path>`
3. `$XDG_CONFIG_HOME/cmdscope/config.toml`
4. built-in defaults if no config file exists

Paths supplied explicitly through `--config` or `CMDSCOPE_CONFIG` are required;
a missing or unreadable explicit file is an error. Unknown TOML fields are also
rejected so misspelled action names cannot silently fall back to defaults.

All non-text shortcuts are configurable through TOML:

    [keys]
    global = "ctrl-g"
    pwd = "ctrl-p"
    git_root = "ctrl-r"
    toggle_scope = "tab"
    context = "ctrl-o"
    context_expand = "alt-]"
    context_shrink = "alt-["
    toggle_pwd_mode = "ctrl-s"
    toggle_metadata = "alt-m"
    select_next = ["down", "ctrl-n"]
    select_previous = ["up", "ctrl-k"]
    accept = "enter"
    quit = ["esc", "ctrl-c"]
    backspace = "backspace"
    delete = ["delete", "ctrl-d"]
    delete_word = "ctrl-w"
    clear_query = "ctrl-u"
    cursor_left = ["left", "ctrl-b"]
    cursor_right = ["right", "ctrl-f"]
    cursor_start = ["home", "ctrl-a"]
    cursor_end = ["end", "ctrl-e"]

    [pwd]
    # exact: only the current directory
    # subdirs: current directory plus all subdirectories
    mode = "subdirs"

    [ui]
    # Columns shown when metadata is visible. Supported: "date", "pwd".
    # Use an empty list for command-only rows by default.
    history_columns = ["date", "pwd"]

### Key token format

Shortcut values use lowercase key tokens:

| Token shape | Example |
| --- | --- |
| control character | `ctrl-g` |
| alt character | `alt-]` |
| plain character | `g` |
| named key | `enter`, `esc`, `up`, `down`, `left`, `right`, `home`, `end`, `pageup`, `pagedown`, `backspace`, `delete`, `insert`, `tab` |

`ctrl-m` is deliberately not the default metadata binding: many terminal protocols encode it identically to Enter. It remains configurable on terminals that can distinguish it.

## Database compatibility

`cmdscope` opens the database read-only and accepts both current Atuin history
tables and older compatible tables without `deleted_at`. The required columns
are:

    id, timestamp, duration, exit, command, cwd, session, hostname

When `deleted_at` exists, soft-deleted rows are excluded. Missing tables,
missing required columns, and incompatible row values produce path-qualified
diagnostics. A short SQLite busy timeout tolerates brief concurrent writer
transactions without changing the database.

Rows are ordered by timestamp and then ID, giving deterministic results and
context windows when multiple commands share a timestamp.

## Search performance

The interactive search engine retains `skim`'s fzf-style dynamic-programming score, but avoids the surrounding work that previously dominated each keypress:

1. Scope changes build a reusable vector of history indices.
   The active fuzzy query is retained and applied to the new scope.
2. Extending a query scans only matches from the previous query layer.
3. Backspace restores an already-ranked prefix layer without rescanning.
4. Prefix caching is bounded to 32 layers so long queries cannot grow memory without limit.
5. A bounded heap keeps only the best 200 results instead of fully sorting every match.
6. The model and renderer borrow immutable history rows by index instead of cloning commands.
7. The event loop blocks while idle and redraws only after input or resize events.

Run the reproducible comparison benchmark with:

    cargo run --release --example filter_bench -- 100000

The benchmark runs both the former full-rescan/full-sort shape and the incremental engine over the same generated history and query sequence. Timing varies by machine; the emitted `scanned` counts are deterministic evidence of the work reduction.

Reference run on July 26, 2026 with 100,000 rows:

| Strategy | Ten-query sequence | Relative |
| --- | ---: | ---: |
| Full rescan + full sort | 563 ms | 1.00× |
| Incremental + bounded top-K | 355 ms | 1.59× faster |

Once the query had narrowed the candidate set, subsequent characters scanned 10,000 rows rather than 100,000. The first broad characters still necessarily inspect the full active scope, so the improvement grows with query selectivity and history size.

## Architecture

    SQLite
      │
      ▼
    history.rs ── immutable entries + cwd/id indexes
      │
      ├── scope.rs ── global/pwd/git-root semantics
      │
      ▼
    search.rs ── incremental fuzzy session + top-K ranking
      │
      ▼
    app.rs ── pure messages and selection/context state
      │
      ├── config.rs + keymap.rs ── TOML validation and event dispatch
      │
      ▼
    tui.rs ── render-only Ratatui projection
      │
      ▼
    terminal.rs ── terminal lifetime, input dispatch, blocking event loop

`main.rs` now owns only CLI parsing and environment discovery. This keeps
storage, filtering, cursor-aware query state, rendering, and terminal runtime in
separate reviewable units. The renderer returns an optional terminal cursor
position; the terminal layer applies it without moving input behavior into the
view.

### Metadata columns

| Column | Meaning |
| --- | --- |
| `date` | calendar date derived from the history timestamp |
| `pwd` | working directory recorded for the history entry |

The date renderer accepts common Atuin timestamp magnitudes and normalizes seconds, milliseconds, microseconds, or nanoseconds before rendering `YYYY-MM-DD`.

Search modes:

| Mode | Meaning |
| --- | --- |
| global | all history rows |
| pwd:exact | only commands whose `cwd` equals the current directory |
| pwd:subdirs | commands in the current directory and children |
| git-root | commands under `git rev-parse --show-toplevel` |

Unavailable pwd or Git-root context produces an empty result set rather than
silently falling back to global history. Unix root scopes include all absolute
descendants; Windows drive and UNC paths are matched case-insensitively across
slash styles, including non-ASCII case pairs. Directory matching is deliberately
lexical after platform-aware separator/case normalization: mount points behave
like ordinary component-bounded subtrees, while a symlink alias and its physical
target remain distinct unless the history database recorded the same path form.
This avoids filesystem lookups and silently retargeting old history when a
symlink later changes.

Context review:

- Enter context with the configured `context` key.
- The active filter string is ignored while reviewing context.
- Expand or shrink time-neighbor radius with `context_expand` / `context_shrink`.
- Typing, switching scope, or toggling pwd mode leaves context mode and resumes fuzzy filtering.
- Interactive context is anchored by immutable entry index. Public ID-based
  context lookup returns no result when duplicate IDs make the request
  ambiguous.

## Rust API

The crate exposes the pure core used by the binary: `HistoryEntry`, `HistoryStore`, `SearchEngine`, `SearchStats`, `SearchMode`, `SearchScope`, `AppConfig`, `KeyMap`, `AppModel`, `Msg`, and `tui::render`.

Generate API docs locally:

    cargo doc --no-deps --open

The public API is intentionally small so tests can exercise behavior without starting a terminal.

## Release builds

GitHub Actions runs CI on pushes and pull requests. Pushing a tag matching `v*` creates a GitHub Release and uploads packaged binaries:

    git tag v0.2.2
    git push origin v0.2.2

Release assets currently include:

- `cmdscope-x86_64-unknown-linux-gnu.tar.gz`
- `cmdscope-macos.tar.gz`
- `cmdscope-x86_64-pc-windows-msvc.zip`

Each archive contains the `cmdscope` executable, `README.md`, `CHANGELOG.md`,
and `examples.config.toml`.
