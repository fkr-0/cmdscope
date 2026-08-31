use anyhow::Result;
use cmdscope::live::FileFingerprint;
use cmdscope::menu::Menu;
use cmdscope::{AppConfig, AppModel, HistoryStore, KeyMap, ModalKey, Msg, tui};
use crossterm::{
    event::{
        self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    collections::BTreeMap,
    io,
    path::Path,
    time::{Duration, Instant},
};

type TuiTerminal = Terminal<CrosstermBackend<io::Stderr>>;

struct TerminalSession {
    terminal: TuiTerminal,
    restored: bool,
}

fn modal_key(model: &AppModel, key: KeyEvent) -> Option<ModalKey> {
    if let Some(menu) = model.actions_menu() {
        if menu.cancel.iter().any(|binding| binding.matches_event(key)) {
            return Some(ModalKey::Cancel);
        }
        if menu
            .previous
            .iter()
            .any(|binding| binding.matches_event(key))
        {
            return Some(ModalKey::Previous);
        }
        if menu.next.iter().any(|binding| binding.matches_event(key)) {
            return Some(ModalKey::Next);
        }
        if menu
            .confirm
            .iter()
            .any(|binding| binding.matches_event(key))
        {
            return Some(ModalKey::Confirm);
        }
    }
    if let Some(window) = model.active_window_config() {
        for (name, binding) in &window.keymap {
            if binding
                .parse::<cmdscope::KeyChord>()
                .ok()
                .is_some_and(|chord| chord.matches_event(key))
            {
                return match name.as_str() {
                    "cancel" | "close" => Some(ModalKey::Cancel),
                    "previous" => Some(ModalKey::Previous),
                    "next" => Some(ModalKey::Next),
                    "confirm" => Some(ModalKey::Confirm),
                    _ => None,
                };
            }
        }
    }
    None
}

impl TerminalSession {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let mut stderr = io::stderr();
        if let Err(error) = execute!(stderr, EnterAlternateScreen, EnableBracketedPaste) {
            let _ = execute!(stderr, DisableBracketedPaste, LeaveAlternateScreen);
            let _ = disable_raw_mode();
            return Err(error.into());
        }
        let terminal = match Terminal::new(CrosstermBackend::new(stderr)) {
            Ok(terminal) => terminal,
            Err(error) => {
                let mut stderr = io::stderr();
                let _ = execute!(stderr, DisableBracketedPaste, LeaveAlternateScreen);
                let _ = disable_raw_mode();
                return Err(error.into());
            }
        };
        Ok(Self {
            terminal,
            restored: false,
        })
    }

    fn restore(&mut self) -> Result<()> {
        if self.restored {
            return Ok(());
        }
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            DisableBracketedPaste,
            LeaveAlternateScreen
        )?;
        self.terminal.show_cursor()?;
        self.restored = true;
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if !self.restored {
            let _ = disable_raw_mode();
            let _ = execute!(
                self.terminal.backend_mut(),
                DisableBracketedPaste,
                LeaveAlternateScreen
            );
            let _ = self.terminal.show_cursor();
        }
    }
}

pub fn run_tui(
    store: HistoryStore,
    cwd: Option<String>,
    git_root: Option<String>,
    config: AppConfig,
    keymap: KeyMap,
    menus: BTreeMap<String, Menu>,
) -> Result<Option<String>> {
    let source_path = store.source_path().map(Path::to_path_buf);
    let effective_columns = config.ui.effective_columns();
    let mut session = TerminalSession::enter()?;
    let mut model =
        AppModel::new_with_config(store, cwd, git_root, &effective_columns, config.pwd.mode);
    model.configure_interactions(menus);
    model.configure_windows(config.ui.windows.clone());
    model.configure_wraps(config.ui.wraps.clone());

    let result = run_event_loop(
        &mut session.terminal,
        &mut model,
        &config,
        &keymap,
        source_path.as_deref(),
    );
    let cleanup = session.restore();
    match (result, cleanup) {
        (Ok(selected), Ok(())) => Ok(selected),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error.context("failed to restore terminal")),
    }
}

fn run_event_loop(
    terminal: &mut TuiTerminal,
    model: &mut AppModel,
    config: &AppConfig,
    keymap: &KeyMap,
    source_path: Option<&Path>,
) -> Result<Option<String>> {
    let mut fingerprint = source_path.and_then(|path| FileFingerprint::read(path).ok());
    let mut last_refresh = Instant::now();
    draw(terminal, model, config)?;
    loop {
        if !event::poll(Duration::from_millis(150))? {
            if let Some(path) = source_path
                && last_refresh.elapsed() >= Duration::from_millis(150)
                && let Ok(next) = FileFingerprint::read(path)
                && fingerprint.as_ref() != Some(&next)
                && let Ok(store) = HistoryStore::load_sqlite(path)
            {
                model.replace_history(store);
                fingerprint = Some(next);
                last_refresh = Instant::now();
                draw(terminal, model, config)?;
            }
            continue;
        }
        match event::read()? {
            Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
                if model.modal_open() {
                    match modal_key(model, key) {
                        Some(ModalKey::Cancel) => model.close_modal(),
                        Some(ModalKey::Previous) => model.handle_modal_previous(),
                        Some(ModalKey::Next) => model.handle_modal_next(),
                        Some(ModalKey::Confirm) => model.execute_menu_action(),
                        None => {
                            if let Some(action) = keymap.action_for(key) {
                                model.update(Msg::from(action));
                            }
                        }
                    }
                    if model.should_quit() {
                        return Ok(model.accepted_command().map(ToOwned::to_owned));
                    }
                    draw(terminal, model, config)?;
                    continue;
                }
                if let Some(action) = keymap.action_for(key) {
                    model.update(Msg::from(action));
                } else if let Some(character) = text_input(key) {
                    model.update(Msg::Input(character));
                } else {
                    continue;
                }

                if model.should_quit() {
                    return Ok(model.accepted_command().map(ToOwned::to_owned));
                }
                draw(terminal, model, config)?;
            }
            Event::Resize(_, _) => draw(terminal, model, config)?,
            Event::Paste(input) => {
                model.update(Msg::Paste(input));
                draw(terminal, model, config)?;
            }
            _ => {}
        }
    }
}

fn draw(terminal: &mut TuiTerminal, model: &AppModel, config: &AppConfig) -> Result<()> {
    terminal.draw(|frame| {
        if let Some(position) = tui::render(model, config, frame.area(), frame.buffer_mut()) {
            frame.set_cursor_position(position);
        }
    })?;
    Ok(())
}

fn text_input(key: KeyEvent) -> Option<char> {
    match (key.code, key.modifiers) {
        (KeyCode::Char(character), KeyModifiers::NONE | KeyModifiers::SHIFT) => Some(character),
        _ => None,
    }
}
