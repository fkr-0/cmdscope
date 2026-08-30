use anyhow::{Context, Result};
use clap::Parser;
use cmdscope::{AppConfig, HistoryStore};
use std::{
    env,
    io::{self, Write},
    path::PathBuf,
    process::Command,
};

mod terminal;

#[derive(Debug, Parser)]
#[command(name = "cmdscope", version, about = "Interactive Atuin history picker")]
struct Args {
    #[arg(long, env = "CMDSCOPE_DB")]
    db: Option<PathBuf>,

    #[arg(long, env = "CMDSCOPE_CONFIG")]
    config: Option<PathBuf>,

    #[arg(long)]
    print_first: bool,

    /// Terminate selected-command stdout with NUL instead of newline.
    #[arg(long = "null")]
    nul: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let config = match args.config {
        Some(path) => AppConfig::load_required(path)?,
        None => AppConfig::load_optional(default_config_path())?,
    };
    let keymap = config.compile_keymap()?;
    let db = args.db.unwrap_or_else(default_db_path);
    let store = HistoryStore::load_sqlite(&db)?;
    if args.print_first {
        if let Some(entry) = store.search("", cmdscope::SearchMode::All, None, 1).first() {
            write_command(&entry.command, args.nul)?;
        }
        return Ok(());
    }

    let cwd = runtime_cwd();
    let git_root = detect_git_root();
    let selected =
        terminal::run_tui(store, cwd, git_root, config, keymap).context("terminal UI failed")?;
    if let Some(command) = selected {
        write_command(&command, args.nul)?;
    }
    Ok(())
}

fn write_command(command: &str, nul_terminated: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    stdout
        .write_all(command.as_bytes())
        .context("failed to write selected command")?;
    stdout
        .write_all(if nul_terminated { b"\0" } else { b"\n" })
        .context("failed to write selected command terminator")?;
    stdout
        .flush()
        .context("failed to flush selected command output")
}

fn runtime_cwd() -> Option<String> {
    runtime_cwd_from(env::var_os("PWD").map(PathBuf::from), || {
        env::current_dir().ok()
    })
}

fn runtime_cwd_from(
    shell_pwd: Option<PathBuf>,
    process_cwd: impl FnOnce() -> Option<PathBuf>,
) -> Option<String> {
    shell_pwd
        .or_else(process_cwd)
        .map(|path| path.display().to_string())
}

fn default_db_path() -> PathBuf {
    let local = PathBuf::from("history.db");
    if local.exists() {
        return local;
    }

    atuin_default_db_path(
        env::var_os("XDG_DATA_HOME").map(PathBuf::from),
        env::var_os("HOME").map(PathBuf::from),
    )
    .unwrap_or(local)
}

fn atuin_default_db_path(xdg_data_home: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    xdg_data_home
        .or_else(|| home.map(|home| home.join(".local/share")))
        .map(|data_home| data_home.join("atuin/history.db"))
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

#[cfg(test)]
mod tests {
    use super::{atuin_default_db_path, runtime_cwd_from};
    use std::path::PathBuf;

    #[test]
    fn runtime_cwd_prefers_shell_pwd_without_resolving_it() {
        let cwd = runtime_cwd_from(Some(PathBuf::from("/logical/project-link")), || {
            panic!("process cwd fallback must not run when PWD is present")
        });

        assert_eq!(cwd.as_deref(), Some("/logical/project-link"));
    }

    #[test]
    fn runtime_cwd_falls_back_when_shell_pwd_is_unset() {
        let cwd = runtime_cwd_from(None, || Some(PathBuf::from("/physical/project")));

        assert_eq!(cwd.as_deref(), Some("/physical/project"));
    }

    #[test]
    fn atuin_default_db_prefers_xdg_data_home() {
        let path = atuin_default_db_path(
            Some(PathBuf::from("/xdg/data")),
            Some(PathBuf::from("/home/me")),
        );

        assert_eq!(path, Some(PathBuf::from("/xdg/data/atuin/history.db")));
    }

    #[test]
    fn atuin_default_db_falls_back_to_home_local_share() {
        let path = atuin_default_db_path(None, Some(PathBuf::from("/home/me")));

        assert_eq!(
            path,
            Some(PathBuf::from("/home/me/.local/share/atuin/history.db"))
        );
    }
}
