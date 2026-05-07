use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;
use skim::{fuzzy_matcher::FuzzyMatcher, prelude::*};
use std::path::Path;

pub mod tui;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: i64,
    pub duration: i64,
    pub exit: i64,
    pub command: String,
    pub cwd: String,
    pub session: String,
    pub hostname: String,
}

impl HistoryEntry {
    pub fn new(
        id: impl Into<String>,
        timestamp: i64,
        exit: i64,
        command: impl Into<String>,
        cwd: impl Into<String>,
        session: impl Into<String>,
        hostname: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            timestamp,
            duration: 0,
            exit,
            command: command.into(),
            cwd: cwd.into(),
            session: session.into(),
            hostname: hostname.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PwdMatchMode {
    #[default]
    Exact,
    #[serde(alias = "subdirs")]
    IncludeSubdirs,
}

impl PwdMatchMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::IncludeSubdirs => "subdirs",
        }
    }

    fn toggle(self) -> Self {
        match self {
            Self::Exact => Self::IncludeSubdirs,
            Self::IncludeSubdirs => Self::Exact,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    All,
    SamePwd,
    GitRoot,
}

impl SearchMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "global",
            Self::SamePwd => "pwd",
            Self::GitRoot => "git root",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchScope {
    Global,
    Pwd {
        pwd: Option<String>,
        mode: PwdMatchMode,
    },
    GitRoot {
        root: Option<String>,
    },
}

impl SearchScope {
    pub fn global() -> Self {
        Self::Global
    }

    pub fn pwd(pwd: Option<&str>, mode: PwdMatchMode) -> Self {
        Self::Pwd {
            pwd: pwd.map(ToOwned::to_owned),
            mode,
        }
    }

    pub fn git_root(root: Option<&str>) -> Self {
        Self::GitRoot {
            root: root.map(ToOwned::to_owned),
        }
    }

    fn matches(&self, entry: &HistoryEntry) -> bool {
        match self {
            Self::Global => true,
            Self::Pwd {
                pwd: Some(pwd),
                mode,
            } => path_matches(pwd, &entry.cwd, *mode),
            Self::Pwd { pwd: None, .. } => true,
            Self::GitRoot { root: Some(root) } => {
                path_matches(root, &entry.cwd, PwdMatchMode::IncludeSubdirs)
            }
            Self::GitRoot { root: None } => true,
        }
    }
}

fn path_matches(root: &str, candidate: &str, mode: PwdMatchMode) -> bool {
    let root = trim_trailing_slashes(root);
    let candidate = trim_trailing_slashes(candidate);
    match mode {
        PwdMatchMode::Exact => candidate == root,
        PwdMatchMode::IncludeSubdirs => {
            candidate == root
                || candidate
                    .strip_prefix(root)
                    .is_some_and(|rest| rest.starts_with('/'))
        }
    }
}

fn trim_trailing_slashes(path: &str) -> &str {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() { "/" } else { trimmed }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct KeyConfig {
    #[serde(default = "default_key_global")]
    pub global: String,
    #[serde(default = "default_key_pwd")]
    pub pwd: String,
    #[serde(default = "default_key_git_root")]
    pub git_root: String,
    #[serde(default = "default_key_context")]
    pub context: String,
    #[serde(default = "default_key_context_expand")]
    pub context_expand: String,
    #[serde(default = "default_key_context_shrink")]
    pub context_shrink: String,
    #[serde(default = "default_key_toggle_pwd_mode")]
    pub toggle_pwd_mode: String,
}

impl Default for KeyConfig {
    fn default() -> Self {
        Self {
            global: default_key_global(),
            pwd: default_key_pwd(),
            git_root: default_key_git_root(),
            context: default_key_context(),
            context_expand: default_key_context_expand(),
            context_shrink: default_key_context_shrink(),
            toggle_pwd_mode: default_key_toggle_pwd_mode(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct PwdConfig {
    #[serde(default)]
    pub mode: PwdMatchMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub keys: KeyConfig,
    #[serde(default)]
    pub pwd: PwdConfig,
}

impl AppConfig {
    pub fn from_toml(input: &str) -> Result<Self> {
        toml::from_str(input).context("failed to parse terminal-history config")
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
}

fn default_key_global() -> String {
    "ctrl-g".to_string()
}
fn default_key_pwd() -> String {
    "ctrl-p".to_string()
}
fn default_key_git_root() -> String {
    "ctrl-r".to_string()
}
fn default_key_context() -> String {
    "ctrl-o".to_string()
}
fn default_key_context_expand() -> String {
    "alt-]".to_string()
}
fn default_key_context_shrink() -> String {
    "alt-[".to_string()
}
fn default_key_toggle_pwd_mode() -> String {
    "ctrl-s".to_string()
}

#[derive(Debug, Clone)]
pub struct HistoryStore {
    entries: Vec<HistoryEntry>,
}

impl HistoryStore {
    pub fn from_entries(mut entries: Vec<HistoryEntry>) -> Self {
        entries.sort_by_key(|entry| entry.timestamp);
        Self { entries }
    }

    pub fn load_sqlite(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_context(|| format!("failed to open history database {}", path.display()))?;

        let mut statement = connection.prepare(
            "select id, timestamp, duration, exit, command, cwd, session, hostname \
             from history where deleted_at is null order by timestamp asc",
        )?;
        let entries = statement
            .query_map([], |row| {
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    duration: row.get(2)?,
                    exit: row.get(3)?,
                    command: row.get(4)?,
                    cwd: row.get(5)?,
                    session: row.get(6)?,
                    hostname: row.get(7)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(Self::from_entries(entries))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    pub fn search(
        &self,
        query: &str,
        mode: SearchMode,
        current_pwd: Option<&str>,
        limit: usize,
    ) -> Vec<HistoryEntry> {
        let scope = match mode {
            SearchMode::All => SearchScope::global(),
            SearchMode::SamePwd => SearchScope::pwd(current_pwd, PwdMatchMode::Exact),
            SearchMode::GitRoot => SearchScope::git_root(current_pwd),
        };
        self.search_with_scope(query, &scope, limit)
    }

    pub fn search_with_scope(
        &self,
        query: &str,
        scope: &SearchScope,
        limit: usize,
    ) -> Vec<HistoryEntry> {
        let candidates: Vec<&HistoryEntry> = self
            .entries
            .iter()
            .filter(|entry| scope.matches(entry))
            .collect();
        let mut results: Vec<&HistoryEntry> = if query.is_empty() {
            candidates
        } else {
            let matcher = SkimMatcherV2::default().ignore_case();
            let mut scored = candidates
                .into_iter()
                .filter_map(|entry| {
                    matcher
                        .fuzzy_match(&entry.command, query)
                        .map(|score| (score, entry.timestamp, entry))
                })
                .collect::<Vec<_>>();
            scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));
            scored.into_iter().map(|(_, _, entry)| entry).collect()
        };
        if query.is_empty() {
            results.sort_by_key(|entry| std::cmp::Reverse(entry.timestamp));
        }
        results.into_iter().take(limit).cloned().collect()
    }

    pub fn context_around_with_scope(
        &self,
        selected_id: &str,
        radius: usize,
        scope: &SearchScope,
    ) -> Option<Vec<HistoryEntry>> {
        let scoped = self
            .entries
            .iter()
            .filter(|entry| scope.matches(entry))
            .collect::<Vec<_>>();
        let selected_index = scoped.iter().position(|entry| entry.id == selected_id)?;
        let start = selected_index.saturating_sub(radius);
        let end = (selected_index + radius + 1).min(scoped.len());
        Some(
            scoped[start..end]
                .iter()
                .map(|entry| (*entry).clone())
                .collect(),
        )
    }

    pub fn context_around(
        &self,
        selected_id: &str,
        radius: usize,
        mode: SearchMode,
        current_pwd: Option<&str>,
    ) -> Option<Vec<HistoryEntry>> {
        let scope = match mode {
            SearchMode::All => SearchScope::global(),
            SearchMode::SamePwd => SearchScope::pwd(current_pwd, PwdMatchMode::Exact),
            SearchMode::GitRoot => SearchScope::git_root(current_pwd),
        };
        self.context_around_with_scope(selected_id, radius, &scope)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    Input(char),
    Backspace,
    SelectNext,
    SelectPrevious,
    TogglePwdFilter,
    TogglePwdMatchMode,
    ShowGlobal,
    ShowPwd,
    ShowGitRoot,
    ToggleContext,
    ContextExpand,
    ContextShrink,
    Accept,
    Quit,
}

#[derive(Debug, Clone)]
pub struct AppModel {
    store: HistoryStore,
    current_pwd: Option<String>,
    git_root: Option<String>,
    query: String,
    search_mode: SearchMode,
    pwd_match_mode: PwdMatchMode,
    selected_index: usize,
    visible: Vec<HistoryEntry>,
    context_mode: bool,
    context_anchor_id: Option<String>,
    context_radius: usize,
    should_quit: bool,
    accepted_command: Option<String>,
}

impl AppModel {
    pub fn new(store: HistoryStore, current_pwd: Option<String>) -> Self {
        Self::new_with_environment(store, current_pwd, None, PwdMatchMode::Exact)
    }

    pub fn new_with_environment(
        store: HistoryStore,
        current_pwd: Option<String>,
        git_root: Option<String>,
        pwd_match_mode: PwdMatchMode,
    ) -> Self {
        let mut model = Self {
            store,
            current_pwd,
            git_root,
            query: String::new(),
            search_mode: SearchMode::All,
            pwd_match_mode,
            selected_index: 0,
            visible: Vec::new(),
            context_mode: false,
            context_anchor_id: None,
            context_radius: 1,
            should_quit: false,
            accepted_command: None,
        };
        model.refresh_results();
        model
    }

    pub fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Input(char) => {
                self.leave_context();
                self.query.push(char);
                self.selected_index = 0;
                self.refresh_results();
            }
            Msg::Backspace => {
                self.leave_context();
                self.query.pop();
                self.selected_index = 0;
                self.refresh_results();
            }
            Msg::SelectNext => {
                if !self.visible.is_empty() {
                    self.selected_index = (self.selected_index + 1).min(self.visible.len() - 1);
                }
            }
            Msg::SelectPrevious => self.selected_index = self.selected_index.saturating_sub(1),
            Msg::TogglePwdFilter => {
                self.search_mode = match self.search_mode {
                    SearchMode::All => SearchMode::SamePwd,
                    SearchMode::SamePwd | SearchMode::GitRoot => SearchMode::All,
                };
                self.leave_context();
                self.selected_index = 0;
                self.refresh_results();
            }
            Msg::TogglePwdMatchMode => {
                self.pwd_match_mode = self.pwd_match_mode.toggle();
                self.leave_context();
                self.selected_index = 0;
                self.refresh_results();
            }
            Msg::ShowGlobal => self.switch_mode(SearchMode::All),
            Msg::ShowPwd => self.switch_mode(SearchMode::SamePwd),
            Msg::ShowGitRoot => self.switch_mode(SearchMode::GitRoot),
            Msg::ToggleContext => {
                if self.context_mode {
                    self.leave_context();
                    self.refresh_results();
                } else if let Some(selected) = self.selected().cloned() {
                    self.context_anchor_id = Some(selected.id);
                    self.context_radius = 1;
                    self.refresh_context();
                }
            }
            Msg::ContextExpand => {
                if self.context_mode {
                    self.context_radius += 1;
                    self.refresh_context();
                }
            }
            Msg::ContextShrink => {
                if self.context_mode {
                    self.context_radius = self.context_radius.saturating_sub(1).max(1);
                    self.refresh_context();
                }
            }
            Msg::Accept => {
                self.accepted_command = self.selected().map(|entry| entry.command.clone());
                self.should_quit = true;
            }
            Msg::Quit => self.should_quit = true,
        }
    }

    fn switch_mode(&mut self, mode: SearchMode) {
        self.search_mode = mode;
        self.leave_context();
        self.selected_index = 0;
        self.refresh_results();
    }

    fn leave_context(&mut self) {
        self.context_mode = false;
        self.context_anchor_id = None;
    }

    fn scope(&self) -> SearchScope {
        match self.search_mode {
            SearchMode::All => SearchScope::global(),
            SearchMode::SamePwd => {
                SearchScope::pwd(self.current_pwd.as_deref(), self.pwd_match_mode)
            }
            SearchMode::GitRoot => {
                SearchScope::git_root(self.git_root.as_deref().or(self.current_pwd.as_deref()))
            }
        }
    }

    fn refresh_results(&mut self) {
        self.visible = self
            .store
            .search_with_scope(&self.query, &self.scope(), 200);
        if self.selected_index >= self.visible.len() {
            self.selected_index = self.visible.len().saturating_sub(1);
        }
    }

    fn refresh_context(&mut self) {
        let Some(anchor_id) = self.context_anchor_id.clone() else {
            return;
        };
        if let Some(context) =
            self.store
                .context_around_with_scope(&anchor_id, self.context_radius, &self.scope())
        {
            self.visible = context;
            self.selected_index = self
                .visible
                .iter()
                .position(|entry| entry.id == anchor_id)
                .unwrap_or(0);
            self.context_mode = true;
        }
    }

    pub fn query(&self) -> &str {
        &self.query
    }
    pub fn search_mode(&self) -> SearchMode {
        self.search_mode
    }
    pub fn pwd_match_mode(&self) -> PwdMatchMode {
        self.pwd_match_mode
    }
    pub fn context_radius(&self) -> usize {
        self.context_radius
    }
    pub fn in_context_mode(&self) -> bool {
        self.context_mode
    }
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }
    pub fn accepted_command(&self) -> Option<&str> {
        self.accepted_command.as_deref()
    }
    pub fn visible(&self) -> &[HistoryEntry] {
        &self.visible
    }
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }
    pub fn selected(&self) -> Option<&HistoryEntry> {
        self.visible.get(self.selected_index)
    }
    pub fn visible_commands(&self) -> Vec<&str> {
        self.visible
            .iter()
            .map(|entry| entry.command.as_str())
            .collect()
    }
}
