use anyhow::Result;
use cmdscope::live::FileFingerprint;
use cmdscope::menu::Menu;
use cmdscope::{AppConfig, AppModel, HistoryStore, KeyMap, ModalKey, Msg, UiState, tui};
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
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

type TuiTerminal = Terminal<CrosstermBackend<io::Stderr>>;
const LIVE_REFRESH_INTERVAL: Duration = Duration::from_millis(150);

enum RuntimeJob {
    ReloadHistory {
        path: PathBuf,
        fingerprint: FileFingerprint,
    },
    SaveUiState {
        path: PathBuf,
        state: UiState,
    },
}

enum RuntimeResult {
    HistoryLoaded {
        fingerprint: FileFingerprint,
        result: std::result::Result<HistoryStore, String>,
    },
    UiStateSaved(std::result::Result<(), String>),
}

struct AsyncDispatcher {
    jobs: Option<Sender<RuntimeJob>>,
    results: Receiver<RuntimeResult>,
    worker: Option<JoinHandle<()>>,
}

impl AsyncDispatcher {
    fn new() -> Self {
        let (job_tx, job_rx) = mpsc::channel::<RuntimeJob>();
        let (result_tx, result_rx) = mpsc::channel::<RuntimeResult>();
        let worker = thread::spawn(move || {
            while let Ok(job) = job_rx.recv() {
                let result = match job {
                    RuntimeJob::ReloadHistory { path, fingerprint } => {
                        RuntimeResult::HistoryLoaded {
                            fingerprint,
                            result: HistoryStore::load_sqlite(path)
                                .map_err(|error| format!("{error:#}")),
                        }
                    }
                    RuntimeJob::SaveUiState { path, state } => RuntimeResult::UiStateSaved(
                        state
                            .save_atomic(path)
                            .map_err(|error| format!("{error:#}")),
                    ),
                };
                if result_tx.send(result).is_err() {
                    break;
                }
            }
        });
        Self {
            jobs: Some(job_tx),
            results: result_rx,
            worker: Some(worker),
        }
    }

    fn dispatch(&self, job: RuntimeJob) -> Result<()> {
        self.jobs
            .as_ref()
            .expect("dispatcher sender exists while running")
            .send(job)
            .map_err(|_| anyhow::anyhow!("async dispatcher stopped unexpectedly"))
    }
}

impl Drop for AsyncDispatcher {
    fn drop(&mut self) {
        self.jobs.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

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
        for (index, item) in menu.items.iter().enumerate() {
            if item
                .key
                .as_deref()
                .and_then(|binding| binding.parse::<cmdscope::KeyChord>().ok())
                .is_some_and(|binding| binding.matches_event(key))
            {
                return Some(ModalKey::Activate(index));
            }
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
    ui_state_path: Option<PathBuf>,
) -> Result<Option<String>> {
    let source_path = store.source_path().map(Path::to_path_buf);
    // Main startup has already resolved persisted state, config, env, and CLI layers.
    // Use that resolved value directly so a runtime override that happens to equal
    // built-in defaults cannot be replaced by the legacy history_columns fallback.
    let effective_columns = config.ui.columns.clone();
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
        ui_state_path.as_deref(),
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
    ui_state_path: Option<&Path>,
) -> Result<Option<String>> {
    let mut fingerprint = source_path.and_then(|path| FileFingerprint::read(path).ok());
    let mut last_refresh = Instant::now();
    let mut reload_in_flight = false;
    let mut scheduled_ui_revision = model.ui_state_revision();
    let dispatcher = AsyncDispatcher::new();
    draw(terminal, model, config)?;
    loop {
        let redraw =
            drain_runtime_results(&dispatcher, model, &mut fingerprint, &mut reload_in_flight)?;
        let now = Instant::now();
        dispatch_history_refresh_if_due(
            &dispatcher,
            source_path,
            &fingerprint,
            &mut last_refresh,
            &mut reload_in_flight,
            now,
        )?;
        if model.ui_state_revision() != scheduled_ui_revision {
            if let Some(path) = ui_state_path {
                dispatcher.dispatch(RuntimeJob::SaveUiState {
                    path: path.to_path_buf(),
                    state: model.ui_state(),
                })?;
            }
            scheduled_ui_revision = model.ui_state_revision();
        }
        if redraw {
            draw(terminal, model, config)?;
        }
        let timeout = refresh_poll_timeout(source_path.is_some(), last_refresh, Instant::now());
        if !event::poll(timeout)? {
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
                        Some(ModalKey::Activate(index)) => model.execute_menu_item(index),
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

fn drain_runtime_results(
    dispatcher: &AsyncDispatcher,
    model: &mut AppModel,
    fingerprint: &mut Option<FileFingerprint>,
    reload_in_flight: &mut bool,
) -> Result<bool> {
    let mut redraw = false;
    loop {
        match dispatcher.results.try_recv() {
            Ok(RuntimeResult::HistoryLoaded {
                fingerprint: next,
                result,
            }) => {
                *reload_in_flight = false;
                if let Ok(store) = result {
                    model.replace_history(store);
                    *fingerprint = Some(next);
                    redraw = true;
                }
            }
            Ok(RuntimeResult::UiStateSaved(Ok(()))) => {}
            Ok(RuntimeResult::UiStateSaved(Err(error))) => {
                anyhow::bail!("failed to persist UI state asynchronously: {error}");
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                anyhow::bail!("async dispatcher stopped unexpectedly");
            }
        }
    }
    Ok(redraw)
}

fn dispatch_history_refresh_if_due(
    dispatcher: &AsyncDispatcher,
    source_path: Option<&Path>,
    fingerprint: &Option<FileFingerprint>,
    last_refresh: &mut Instant,
    reload_in_flight: &mut bool,
    now: Instant,
) -> Result<()> {
    let Some(path) = source_path else {
        return Ok(());
    };
    if now.duration_since(*last_refresh) < LIVE_REFRESH_INTERVAL {
        return Ok(());
    }
    *last_refresh = now;
    if *reload_in_flight {
        return Ok(());
    }
    let Ok(next) = FileFingerprint::read(path) else {
        return Ok(());
    };
    if fingerprint.as_ref() == Some(&next) {
        return Ok(());
    }
    dispatcher.dispatch(RuntimeJob::ReloadHistory {
        path: path.to_path_buf(),
        fingerprint: next,
    })?;
    *reload_in_flight = true;
    Ok(())
}

fn refresh_poll_timeout(has_live_source: bool, last_refresh: Instant, now: Instant) -> Duration {
    if !has_live_source {
        return LIVE_REFRESH_INTERVAL;
    }
    LIVE_REFRESH_INTERVAL.saturating_sub(now.duration_since(last_refresh))
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

#[cfg(test)]
mod tests {
    use super::*;
    use cmdscope::HistoryEntry;

    fn model() -> AppModel {
        AppModel::new(
            HistoryStore::from_entries(vec![HistoryEntry::new(
                "1", 100, 0, "ls", "/repo", "s", "h",
            )]),
            Some("/repo".into()),
        )
    }

    #[test]
    fn item_activation_key_dispatches_exact_menu_item() {
        let config = AppConfig::from_toml(
            r#"
            [ui.menus.actions]
            [[ui.menus.actions.items]]
            label = "append"
            action = "append"
            [[ui.menus.actions.items]]
            label = "insert"
            key = "i"
            action = "insert+exit"
            "#,
        )
        .unwrap();
        config.ui.validate_references().unwrap();
        let mut model = model();
        model.configure_interactions(config.ui.compile_menus().unwrap());
        model.update(Msg::OpenActions);

        let key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
        assert_eq!(modal_key(&model, key), Some(ModalKey::Activate(1)));
    }

    #[test]
    fn live_refresh_deadline_is_not_postponed_by_ready_input() {
        let start = Instant::now();
        assert_eq!(
            refresh_poll_timeout(true, start, start + Duration::from_millis(40)),
            Duration::from_millis(110)
        );
        assert_eq!(
            refresh_poll_timeout(true, start, start + LIVE_REFRESH_INTERVAL),
            Duration::ZERO
        );
        assert_eq!(
            refresh_poll_timeout(false, start, start + Duration::from_secs(1)),
            LIVE_REFRESH_INTERVAL
        );
    }

    #[test]
    fn async_dispatcher_persists_ui_state() {
        let directory = std::env::temp_dir().join(format!(
            "cmdscope-async-state-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = directory.join("ui-state.toml");
        let dispatcher = AsyncDispatcher::new();
        let state = UiState::default();

        dispatcher
            .dispatch(RuntimeJob::SaveUiState {
                path: path.clone(),
                state: state.clone(),
            })
            .unwrap();
        match dispatcher
            .results
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
        {
            RuntimeResult::UiStateSaved(result) => result.unwrap(),
            RuntimeResult::HistoryLoaded { .. } => panic!("unexpected history result"),
        }
        assert_eq!(UiState::load_optional(&path).unwrap(), Some(state));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
