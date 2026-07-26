use anyhow::Result;
use cmdscope::{AppConfig, AppModel, HistoryStore, KeyMap, Msg, tui};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

type TuiTerminal = Terminal<CrosstermBackend<io::Stdout>>;

struct TerminalSession {
    terminal: TuiTerminal,
    restored: bool,
}

impl TerminalSession {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error.into());
        }
        let terminal = match Terminal::new(CrosstermBackend::new(stdout)) {
            Ok(terminal) => terminal,
            Err(error) => {
                let mut stdout = io::stdout();
                let _ = execute!(stdout, LeaveAlternateScreen);
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
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        self.restored = true;
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if !self.restored {
            let _ = disable_raw_mode();
            let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
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
) -> Result<Option<String>> {
    let mut session = TerminalSession::enter()?;
    let mut model = AppModel::new_with_environment(store, cwd, git_root, config.pwd.mode);

    let result = run_event_loop(&mut session.terminal, &mut model, &config, &keymap);
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
) -> Result<Option<String>> {
    draw(terminal, model, config)?;
    loop {
        match event::read()? {
            Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
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
            _ => {}
        }
    }
}

fn draw(terminal: &mut TuiTerminal, model: &AppModel, config: &AppConfig) -> Result<()> {
    terminal.draw(|frame| tui::render(model, config, frame.area(), frame.buffer_mut()))?;
    Ok(())
}

fn text_input(key: KeyEvent) -> Option<char> {
    match (key.code, key.modifiers) {
        (KeyCode::Char(character), KeyModifiers::NONE | KeyModifiers::SHIFT) => Some(character),
        _ => None,
    }
}
