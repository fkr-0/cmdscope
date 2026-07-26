use anyhow::{Context, Result};
use clap::Parser;
use cmdscope::{AppConfig, HistoryStore};
use std::{env, path::PathBuf, process::Command};

mod terminal;

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
    let keymap = config.compile_keymap()?;
    let store = HistoryStore::load_sqlite(&args.db)?;
    if args.print_first {
        if let Some(entry) = store.search("", cmdscope::SearchMode::All, None, 1).first() {
            println!("{}", entry.command);
        }
        return Ok(());
    }

    let cwd = env::current_dir()
        .ok()
        .map(|path| path.display().to_string());
    let git_root = detect_git_root();
    let selected =
        terminal::run_tui(store, cwd, git_root, config, keymap).context("terminal UI failed")?;
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
    (!root.is_empty()).then(|| root.to_string())
}
