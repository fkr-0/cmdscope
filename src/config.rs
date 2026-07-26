use crate::keymap::KeyMap;
use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer};
use std::path::Path;

fn one_or_many<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Value {
        One(String),
        Many(Vec<String>),
    }

    Ok(match Value::deserialize(deserializer)? {
        Value::One(value) => vec![value],
        Value::Many(values) => values,
    })
}

fn binding(value: &str) -> Vec<String> {
    vec![value.to_string()]
}

fn bindings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

/// TOML-configurable bindings for every non-text-input action.
///
/// Each field accepts either one string or a list of strings.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct KeyConfig {
    #[serde(default = "default_key_global", deserialize_with = "one_or_many")]
    pub global: Vec<String>,
    #[serde(default = "default_key_pwd", deserialize_with = "one_or_many")]
    pub pwd: Vec<String>,
    #[serde(default = "default_key_git_root", deserialize_with = "one_or_many")]
    pub git_root: Vec<String>,
    #[serde(default = "default_key_toggle_scope", deserialize_with = "one_or_many")]
    pub toggle_scope: Vec<String>,
    #[serde(
        default = "default_key_toggle_pwd_mode",
        deserialize_with = "one_or_many"
    )]
    pub toggle_pwd_mode: Vec<String>,
    #[serde(default = "default_key_context", deserialize_with = "one_or_many")]
    pub context: Vec<String>,
    #[serde(
        default = "default_key_context_expand",
        deserialize_with = "one_or_many"
    )]
    pub context_expand: Vec<String>,
    #[serde(
        default = "default_key_context_shrink",
        deserialize_with = "one_or_many"
    )]
    pub context_shrink: Vec<String>,
    #[serde(
        default = "default_key_toggle_metadata",
        deserialize_with = "one_or_many"
    )]
    pub toggle_metadata: Vec<String>,
    #[serde(default = "default_key_select_next", deserialize_with = "one_or_many")]
    pub select_next: Vec<String>,
    #[serde(
        default = "default_key_select_previous",
        deserialize_with = "one_or_many"
    )]
    pub select_previous: Vec<String>,
    #[serde(default = "default_key_accept", deserialize_with = "one_or_many")]
    pub accept: Vec<String>,
    #[serde(default = "default_key_quit", deserialize_with = "one_or_many")]
    pub quit: Vec<String>,
    #[serde(default = "default_key_backspace", deserialize_with = "one_or_many")]
    pub backspace: Vec<String>,
}

impl Default for KeyConfig {
    fn default() -> Self {
        Self {
            global: default_key_global(),
            pwd: default_key_pwd(),
            git_root: default_key_git_root(),
            toggle_scope: default_key_toggle_scope(),
            toggle_pwd_mode: default_key_toggle_pwd_mode(),
            context: default_key_context(),
            context_expand: default_key_context_expand(),
            context_shrink: default_key_context_shrink(),
            toggle_metadata: default_key_toggle_metadata(),
            select_next: default_key_select_next(),
            select_previous: default_key_select_previous(),
            accept: default_key_accept(),
            quit: default_key_quit(),
            backspace: default_key_backspace(),
        }
    }
}

impl KeyConfig {
    /// Compact human-readable rendering used in the shortcuts footer.
    pub fn display(bindings: &[String]) -> String {
        bindings.join("/")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct PwdConfig {
    #[serde(default)]
    pub mode: crate::PwdMatchMode,
}

/// Metadata columns that can be rendered next to each command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryColumn {
    Date,
    Pwd,
}

fn default_history_columns() -> Vec<HistoryColumn> {
    vec![HistoryColumn::Date, HistoryColumn::Pwd]
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_history_columns")]
    pub history_columns: Vec<HistoryColumn>,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            history_columns: default_history_columns(),
        }
    }
}

/// Top-level TOML configuration.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub keys: KeyConfig,
    #[serde(default)]
    pub pwd: PwdConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

impl AppConfig {
    pub fn from_toml(input: &str) -> Result<Self> {
        toml::from_str(input).context("failed to parse cmdscope config")
    }

    pub fn load_optional(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let input = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config {}", path.display()))?;
        Self::from_toml(&input)
    }

    /// Parse and validate all configured key chords once at startup.
    pub fn compile_keymap(&self) -> Result<KeyMap> {
        KeyMap::from_config(&self.keys)
    }
}

fn default_key_global() -> Vec<String> {
    binding("ctrl-g")
}
fn default_key_pwd() -> Vec<String> {
    binding("ctrl-p")
}
fn default_key_git_root() -> Vec<String> {
    binding("ctrl-r")
}
fn default_key_toggle_scope() -> Vec<String> {
    binding("tab")
}
fn default_key_toggle_pwd_mode() -> Vec<String> {
    binding("ctrl-s")
}
fn default_key_context() -> Vec<String> {
    binding("ctrl-o")
}
fn default_key_context_expand() -> Vec<String> {
    binding("alt-]")
}
fn default_key_context_shrink() -> Vec<String> {
    binding("alt-[")
}
fn default_key_toggle_metadata() -> Vec<String> {
    binding("alt-m")
}
fn default_key_select_next() -> Vec<String> {
    bindings(&["down", "ctrl-n"])
}
fn default_key_select_previous() -> Vec<String> {
    bindings(&["up", "ctrl-k"])
}
fn default_key_accept() -> Vec<String> {
    binding("enter")
}
fn default_key_quit() -> Vec<String> {
    bindings(&["esc", "ctrl-c"])
}
fn default_key_backspace() -> Vec<String> {
    binding("backspace")
}
