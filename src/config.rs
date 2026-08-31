use crate::{action::Action, columns::ColumnId, keymap::KeyMap};
use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer};
use std::{collections::BTreeMap, io::ErrorKind, path::Path};

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

fn validate_action(action: &Action, config: &UiConfig) -> Result<()> {
    match action {
        Action::Menu(name) if !config.menus.contains_key(name) => {
            anyhow::bail!("unknown menu reference {name:?}")
        }
        Action::Window(name)
            if !config.windows.contains_key(name)
                && !matches!(
                    name.as_str(),
                    "inspect" | "location" | "timeline" | "preview"
                ) =>
        {
            anyhow::bail!("unknown window reference {name:?}")
        }
        Action::Wrap(name) if !config.wraps.contains_key(name) => {
            anyhow::bail!("unknown wrap reference {name:?}")
        }
        _ => Ok(()),
    }
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
#[serde(deny_unknown_fields)]
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
    #[serde(default = "default_key_actions", deserialize_with = "one_or_many")]
    pub actions: Vec<String>,
    #[serde(default = "default_key_toggle_date", deserialize_with = "one_or_many")]
    pub toggle_date: Vec<String>,
    #[serde(default = "default_key_toggle_pwd", deserialize_with = "one_or_many")]
    pub toggle_pwd: Vec<String>,
    #[serde(default = "default_key_toggle_exit", deserialize_with = "one_or_many")]
    pub toggle_exit: Vec<String>,
    #[serde(
        default = "default_key_toggle_duration",
        deserialize_with = "one_or_many"
    )]
    pub toggle_duration: Vec<String>,
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
    #[serde(default = "default_key_delete", deserialize_with = "one_or_many")]
    pub delete: Vec<String>,
    #[serde(default = "default_key_delete_word", deserialize_with = "one_or_many")]
    pub delete_word: Vec<String>,
    #[serde(default = "default_key_clear_query", deserialize_with = "one_or_many")]
    pub clear_query: Vec<String>,
    #[serde(default = "default_key_cursor_left", deserialize_with = "one_or_many")]
    pub cursor_left: Vec<String>,
    #[serde(default = "default_key_cursor_right", deserialize_with = "one_or_many")]
    pub cursor_right: Vec<String>,
    #[serde(default = "default_key_cursor_start", deserialize_with = "one_or_many")]
    pub cursor_start: Vec<String>,
    #[serde(default = "default_key_cursor_end", deserialize_with = "one_or_many")]
    pub cursor_end: Vec<String>,
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
            actions: default_key_actions(),
            toggle_date: default_key_toggle_date(),
            toggle_pwd: default_key_toggle_pwd(),
            toggle_exit: default_key_toggle_exit(),
            toggle_duration: default_key_toggle_duration(),
            select_next: default_key_select_next(),
            select_previous: default_key_select_previous(),
            accept: default_key_accept(),
            quit: default_key_quit(),
            backspace: default_key_backspace(),
            delete: default_key_delete(),
            delete_word: default_key_delete_word(),
            clear_query: default_key_clear_query(),
            cursor_left: default_key_cursor_left(),
            cursor_right: default_key_cursor_right(),
            cursor_start: default_key_cursor_start(),
            cursor_end: default_key_cursor_end(),
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
#[serde(deny_unknown_fields)]
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

/// Compact and deterministic representations supported by metadata columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DateFormat {
    #[default]
    Relative,
    RelativeLong,
    Date,
    Datetime,
    DatetimeSeconds,
    Iso8601,
    Epoch,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColumnConfig {
    #[serde(default = "default_column_date", alias = "age")]
    pub date: bool,
    #[serde(default = "default_column_pwd")]
    pub pwd: bool,
    #[serde(default)]
    pub exit: bool,
    #[serde(default)]
    pub duration: bool,
    #[serde(default)]
    pub date_format: DateFormat,
    #[serde(default = "default_column_min_width")]
    pub min_width: usize,
    #[serde(default)]
    pub order: Vec<ColumnId>,
}

impl Default for ColumnConfig {
    fn default() -> Self {
        Self {
            date: true,
            pwd: true,
            exit: false,
            duration: false,
            date_format: DateFormat::Relative,
            min_width: 12,
            order: vec![
                ColumnId::Date,
                ColumnId::Pwd,
                ColumnId::Exit,
                ColumnId::Duration,
            ],
        }
    }
}

fn default_column_date() -> bool {
    true
}
fn default_column_pwd() -> bool {
    true
}
fn default_column_min_width() -> usize {
    12
}

impl ColumnConfig {
    pub fn validate(&self) -> Result<()> {
        if self.min_width < 4 {
            anyhow::bail!("column min_width must be at least 4");
        }
        Ok(())
    }

    pub fn visible(&self) -> Vec<ColumnId> {
        let enabled = [
            self.date.then_some(ColumnId::Date),
            self.pwd.then_some(ColumnId::Pwd),
            self.exit.then_some(ColumnId::Exit),
            self.duration.then_some(ColumnId::Duration),
        ]
        .into_iter()
        .flatten()
        .collect::<std::collections::HashSet<_>>();
        let mut order = if self.order.is_empty() {
            vec![
                ColumnId::Date,
                ColumnId::Pwd,
                ColumnId::Exit,
                ColumnId::Duration,
            ]
        } else {
            self.order.clone()
        };
        for column in [
            ColumnId::Date,
            ColumnId::Pwd,
            ColumnId::Exit,
            ColumnId::Duration,
        ] {
            if !order.contains(&column) {
                order.push(column);
            }
        }
        order
            .into_iter()
            .filter(|column| enabled.contains(column))
            .collect()
    }
}

fn default_history_columns() -> Vec<HistoryColumn> {
    vec![HistoryColumn::Date, HistoryColumn::Pwd]
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiConfig {
    #[serde(default = "default_history_columns")]
    pub history_columns: Vec<HistoryColumn>,
    #[serde(default)]
    pub columns: ColumnConfig,
    #[serde(default = "default_preview")]
    pub preview: bool,
    #[serde(default)]
    pub column: BTreeMap<ColumnId, ColumnPresentation>,
    #[serde(default)]
    pub menus: BTreeMap<String, MenuConfig>,
    #[serde(default)]
    pub windows: BTreeMap<String, WindowConfig>,
    #[serde(default)]
    pub wraps: BTreeMap<String, WrapConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColumnPresentation {
    #[serde(default)]
    pub width: Option<usize>,
    #[serde(default)]
    pub min_width: Option<usize>,
    #[serde(default)]
    pub max_width: Option<usize>,
    #[serde(default)]
    pub align: Alignment,
    #[serde(default)]
    pub truncation: Truncation,
    #[serde(default = "default_column_priority")]
    pub priority: u8,
}

impl Default for ColumnPresentation {
    fn default() -> Self {
        Self {
            width: None,
            min_width: None,
            max_width: None,
            align: Alignment::Left,
            truncation: Truncation::End,
            priority: 100,
        }
    }
}
fn default_column_priority() -> u8 {
    100
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Truncation {
    #[default]
    End,
    Start,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WrapConfig {
    pub template: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MenuItemConfig {
    pub label: String,
    #[serde(default)]
    pub key: Option<String>,
    pub action: Action,
    #[serde(default)]
    pub on_select: Vec<Action>,
    #[serde(default)]
    pub on_leave: Vec<Action>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MenuConfig {
    #[serde(default)]
    pub name: String,
    pub items: Vec<MenuItemConfig>,
    #[serde(default = "default_menu_confirm", deserialize_with = "one_or_many")]
    pub confirm: Vec<String>,
    #[serde(default = "default_menu_next", deserialize_with = "one_or_many")]
    pub next: Vec<String>,
    #[serde(default = "default_menu_previous", deserialize_with = "one_or_many")]
    pub previous: Vec<String>,
    #[serde(default = "default_menu_cancel", deserialize_with = "one_or_many")]
    pub cancel: Vec<String>,
    #[serde(default)]
    pub on_open: Vec<Action>,
    #[serde(default)]
    pub on_leave: Vec<Action>,
}
fn default_menu_confirm() -> Vec<String> {
    vec!["enter".into()]
}
fn default_menu_next() -> Vec<String> {
    vec!["down".into(), "j".into()]
}
fn default_menu_previous() -> Vec<String> {
    vec!["up".into(), "k".into()]
}
fn default_menu_cancel() -> Vec<String> {
    vec!["esc".into()]
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowConfig {
    pub kind: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub keymap: BTreeMap<String, String>,
    #[serde(default)]
    pub width: Option<u16>,
    #[serde(default)]
    pub height: Option<u16>,
    #[serde(default)]
    pub on_open: Vec<Action>,
    #[serde(default)]
    pub on_close: Vec<Action>,
}

impl UiConfig {
    pub fn effective_columns(&self) -> ColumnConfig {
        let mut columns = if self.columns == ColumnConfig::default()
            && self.history_columns != default_history_columns()
        {
            ColumnConfig {
                date: self.history_columns.contains(&HistoryColumn::Date),
                pwd: self.history_columns.contains(&HistoryColumn::Pwd),
                ..self.columns.clone()
            }
        } else {
            self.columns.clone()
        };
        if columns.order.is_empty() {
            columns.order = vec![
                ColumnId::Date,
                ColumnId::Pwd,
                ColumnId::Exit,
                ColumnId::Duration,
            ];
        }
        columns
    }

    pub fn presentation(&self, column: ColumnId) -> ColumnPresentation {
        self.column.get(&column).cloned().unwrap_or_default()
    }

    pub fn compile_menus(&self) -> Result<BTreeMap<String, crate::menu::Menu>> {
        let mut menus = BTreeMap::new();
        for (name, config) in &self.menus {
            let mut config = config.clone();
            if config.name.is_empty() {
                config.name = name.clone();
            }
            menus.insert(name.clone(), crate::menu::Menu::from_config(&config)?);
        }
        Ok(menus)
    }

    pub fn validate_references(&self) -> Result<()> {
        self.columns.validate()?;
        for (name, menu) in &self.menus {
            for item in &menu.items {
                validate_action(&item.action, self).with_context(|| format!("menu {name:?}"))?;
                for action in item.on_select.iter().chain(item.on_leave.iter()) {
                    validate_action(action, self).with_context(|| format!("menu {name:?}"))?;
                }
            }
            for action in menu.on_open.iter().chain(menu.on_leave.iter()) {
                validate_action(action, self).with_context(|| format!("menu {name:?}"))?;
            }
            for binding in menu
                .confirm
                .iter()
                .chain(menu.next.iter())
                .chain(menu.previous.iter())
                .chain(menu.cancel.iter())
            {
                binding
                    .parse::<crate::KeyChord>()
                    .with_context(|| format!("menu {name:?} binding {binding:?}"))?;
            }
        }
        for (name, window) in &self.windows {
            if !matches!(window.kind.as_str(), "location" | "timeline" | "preview") {
                anyhow::bail!("window {name:?} has unsupported kind {:?}", window.kind);
            }
            for binding in window.keymap.values() {
                binding
                    .parse::<crate::KeyChord>()
                    .with_context(|| format!("window {name:?} binding {binding:?}"))?;
            }
            for action in window.on_open.iter().chain(window.on_close.iter()) {
                validate_action(action, self).with_context(|| format!("window {name:?}"))?;
            }
        }
        Ok(())
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            history_columns: default_history_columns(),
            columns: ColumnConfig::default(),
            preview: true,
            column: BTreeMap::new(),
            menus: BTreeMap::new(),
            windows: BTreeMap::new(),
            wraps: BTreeMap::new(),
        }
    }
}

fn default_preview() -> bool {
    true
}

/// Top-level TOML configuration.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(deny_unknown_fields)]
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
        match std::fs::symlink_metadata(path) {
            Ok(_) => Self::load_required(path),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => {
                Err(error).with_context(|| format!("failed to inspect config {}", path.display()))
            }
        }
    }

    pub fn load_required(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
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
fn default_key_actions() -> Vec<String> {
    binding("ctrl-space")
}
fn default_key_toggle_date() -> Vec<String> {
    binding("alt-1")
}
fn default_key_toggle_pwd() -> Vec<String> {
    binding("alt-2")
}
fn default_key_toggle_exit() -> Vec<String> {
    binding("alt-3")
}
fn default_key_toggle_duration() -> Vec<String> {
    binding("alt-4")
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
fn default_key_delete() -> Vec<String> {
    bindings(&["delete", "ctrl-d"])
}
fn default_key_delete_word() -> Vec<String> {
    binding("ctrl-w")
}
fn default_key_clear_query() -> Vec<String> {
    binding("ctrl-u")
}
fn default_key_cursor_left() -> Vec<String> {
    bindings(&["left", "ctrl-b"])
}
fn default_key_cursor_right() -> Vec<String> {
    bindings(&["right", "ctrl-f"])
}
fn default_key_cursor_start() -> Vec<String> {
    bindings(&["home", "ctrl-a"])
}
fn default_key_cursor_end() -> Vec<String> {
    bindings(&["end", "ctrl-e"])
}
