# terminal-history

`terminal-history` is a Rust terminal UI for interactively searching an Atuin-style shell history SQLite database.
It is designed for shell `Ctrl-R` usage: fuzzy-filter commands, optionally restrict results to the current directory, inspect time-neighbor context around a result, then print the selected command for shell insertion.

## Features

- Elm-style architecture split into pure model/update logic and terminal rendering.
- Fuzzy matching through `skim`'s matcher instead of a custom fuzzy implementation.
- `ratatui` + `crossterm` TUI with keyboard-driven interaction.
- Atuin `history` table support, including `cwd` and `deleted_at` filtering.
- Same-directory mode for context-sensitive command recall.
- Context mode that ignores the active filter string and shows commands around the selected result in time order.

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

## Build and run

    cargo build
    cargo run -- --db ./history.db

Smoke-test DB loading without entering the TUI:

    cargo run -- --db ./history.db --print-first

## Key bindings

| Key | Action |
| --- | --- |
| text input | update fuzzy filter |
| Backspace | remove one filter character |
| Up/Down | move selection |
| Tab | toggle all-history vs same-pwd filter |
| Ctrl-X | toggle time-neighbor context for the selected command |
| Enter | accept selected command and print it to stdout |
| Esc / Ctrl-C | quit without selecting |

## Shell integration

Example Bash/Zsh binding shape:

    thist-widget() {
      local selected
      selected="$(TERMINAL_HISTORY_DB="$HOME/.local/share/atuin/history.db" thist)" || return
      [[ -n "$selected" ]] || return
      BUFFER="$selected"
      CURSOR=${#BUFFER}
      zle redisplay
    }
    zle -N thist-widget
    bindkey '^R' thist-widget

For Bash/readline, wire `thist` as a command substitution in a custom `bind -x` function.

## Development

    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo test --all-targets

The tests use TDD-friendly pure model/store behavior so the interactive terminal loop stays thin.

## Config

`thist` reads `--config`, `TERMINAL_HISTORY_CONFIG`, or `XDG_CONFIG_HOME/terminal-history/config.toml`.
All shortcuts are configurable through TOML:

    [keys]
    global = "ctrl-g"
    pwd = "ctrl-p"
    git_root = "ctrl-r"
    context = "ctrl-o"
    context_expand = "alt-]"
    context_shrink = "alt-["
    toggle_pwd_mode = "ctrl-s"

    [pwd]
    # exact: only the current directory
    # subdirs: current directory plus all subdirectories
    mode = "subdirs"

Search modes:

| Mode | Meaning |
| --- | --- |
| global | all history rows |
| pwd:exact | only commands whose `cwd` equals the current directory |
| pwd:subdirs | commands in the current directory and children |
| git-root | commands under `git rev-parse --show-toplevel` |

Context review:

- Enter context with the configured `context` key.
- The active filter string is ignored while reviewing context.
- Expand or shrink time-neighbor radius with `context_expand` / `context_shrink`.
- Typing, switching scope, or toggling pwd mode leaves context mode and resumes fuzzy filtering.

## Release builds

GitHub Actions runs CI on pushes and pull requests. Pushing a tag matching `v*` creates a GitHub Release and uploads packaged binaries:

    git tag v0.1.0
    git push origin v0.1.0

Release assets currently include:

- `thist-x86_64-unknown-linux-gnu.tar.gz`
- `thist-macos.tar.gz`
- `thist-x86_64-pc-windows-msvc.zip`

Each archive contains the `thist` executable, `README.md`, and `examples.config.toml`.
