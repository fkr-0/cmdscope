use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{env, io, path::PathBuf, process::Command, time::Duration};
use cmdscope::{AppConfig, AppModel, HistoryStore, Msg, tui};

#[derive(Debug, Parser)]
#[command(name = "cmdscope", about = "Interactive Atuin history picker")]
struct Args {
    #[arg(long, env = "CMDSCOPE_DB", default_value = "history.db")]
    db: PathBuf,

    #[arg(long, env = "CMDSCOPE_CONFIG")]
    config: Option<PathBuf>,

    #[arg(long)]
    print_first: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let config_path = args.config.unwrap_or_else(default_config_path);
    let config = AppConfig::load_optional(config_path)?;
    let store = HistoryStore::load_sqlite(&args.db)?;
    if args.print_first {
        if let Some(entry) = store
            .search("", cmdscope::SearchMode::All, None, 1)
            .first()
        {
            println!("{}", entry.command);
        }
        return Ok(());
    }

    let cwd = env::current_dir()
        .ok()
        .map(|path| path.display().to_string());
    let git_root = detect_git_root();
    let selected = run_tui(store, cwd, git_root, config).context("terminal UI failed")?;
    if let Some(command) = selected {
        println!("{}", command);
    }
    Ok(())
}

fn default_config_path() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cmdscope/config.toml")
}

fn detect_git_root() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let root = String::from_utf8(output.stdout).ok()?;
    let root = root.trim();
    if root.is_empty() {
        None
    } else {
        Some(root.to_string())
    }
}

fn run_tui(
    store: HistoryStore,
    cwd: Option<String>,
    git_root: Option<String>,
    config: AppConfig,
) -> Result<Option<String>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut model = AppModel::new_with_environment(store, cwd, git_root, config.pwd.mode);

    let result = loop {
        terminal.draw(|frame| tui::render(&model, &config, frame.area(), frame.buffer_mut()))?;
        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
        {
            match translate_key(key, &config) {
                Some(Msg::Quit) => break Ok(None),
                Some(msg) => model.update(msg),
                None => {}
            }
        }
        if model.should_quit() {
            break Ok(model.accepted_command().map(ToOwned::to_owned));
        }
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn translate_key(key: KeyEvent, config: &AppConfig) -> Option<Msg> {
    let token = key_token(key)?;
    if token == config.keys.global {
        return Some(Msg::ShowGlobal);
    }
    if token == config.keys.pwd {
        return Some(Msg::ShowPwd);
    }
    if token == config.keys.git_root {
        return Some(Msg::ShowGitRoot);
    }
    if token == config.keys.context {
        return Some(Msg::ToggleContext);
    }
    if token == config.keys.context_expand {
        return Some(Msg::ContextExpand);
    }
    if token == config.keys.context_shrink {
        return Some(Msg::ContextShrink);
    }
    if token == config.keys.toggle_pwd_mode {
        return Some(Msg::TogglePwdMatchMode);
    }

    match (key.code, key.modifiers) {
        (KeyCode::Enter, _) => Some(Msg::Accept),
        (KeyCode::Esc, _) => Some(Msg::Quit),
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Msg::Quit),
        (KeyCode::Tab, _) => Some(Msg::TogglePwdFilter),
        (KeyCode::Down, _) => Some(Msg::SelectNext),
        (KeyCode::Up, _) => Some(Msg::SelectPrevious),
        (KeyCode::Backspace, _) => Some(Msg::Backspace),
        (KeyCode::Char(char), KeyModifiers::NONE | KeyModifiers::SHIFT) => Some(Msg::Input(char)),
        _ => None,
    }
}

fn key_token(key: KeyEvent) -> Option<String> {
    match (key.code, key.modifiers) {
        (KeyCode::Char(char), KeyModifiers::CONTROL) => Some(format!("ctrl-{char}")),
        (KeyCode::Char(char), KeyModifiers::ALT) => Some(format!("alt-{char}")),
        (KeyCode::Char(char), KeyModifiers::NONE | KeyModifiers::SHIFT) => Some(char.to_string()),
        (KeyCode::Tab, _) => Some("tab".to_string()),
        (KeyCode::Enter, _) => Some("enter".to_string()),
        (KeyCode::Esc, _) => Some("esc".to_string()),
        (KeyCode::Up, _) => Some("up".to_string()),
        (KeyCode::Down, _) => Some("down".to_string()),
        (KeyCode::Left, _) => Some("left".to_string()),
        (KeyCode::Right, _) => Some("right".to_string()),
        (KeyCode::Backspace, _) => Some("backspace".to_string()),
        _ => None,
    }
}
