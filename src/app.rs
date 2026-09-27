use crate::{
    Action, ColumnId, HistoryEntry, HistoryStore, KeyAction, PwdMatchMode, SearchEngine,
    SearchMode, SearchScope, SearchStats,
    config::{ColumnConfig, WindowConfig, WrapConfig},
    menu::{Menu, default_actions_menu},
    ui_state::{SortDirection, SortField, UiState},
};
use std::collections::BTreeMap;

/// Pure update messages accepted by [`AppModel::update`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    Input(char),
    Paste(String),
    Backspace,
    Delete,
    DeleteWord,
    ClearQuery,
    CursorLeft,
    CursorRight,
    CursorStart,
    CursorEnd,
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
    ToggleMetadata,
    OpenActions,
    ToggleDate,
    TogglePwd,
    ToggleExit,
    ToggleDuration,
    ColumnNext,
    ColumnMoveLeft,
    ColumnMoveRight,
    ColumnToggle,
    SortByColumn,
    SortDirection,
    OpenLocation,
    OpenTimeline,
    Accept,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalKey {
    Confirm,
    Activate(usize),
    Next,
    Previous,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ViewMode {
    Search,
    Context { anchor_index: usize, radius: usize },
}

impl From<KeyAction> for Msg {
    fn from(action: KeyAction) -> Self {
        match action {
            KeyAction::ShowGlobal => Self::ShowGlobal,
            KeyAction::ShowPwd => Self::ShowPwd,
            KeyAction::ShowGitRoot => Self::ShowGitRoot,
            KeyAction::ToggleScope => Self::TogglePwdFilter,
            KeyAction::TogglePwdMode => Self::TogglePwdMatchMode,
            KeyAction::ToggleContext => Self::ToggleContext,
            KeyAction::ContextExpand => Self::ContextExpand,
            KeyAction::ContextShrink => Self::ContextShrink,
            KeyAction::ToggleMetadata => Self::ToggleMetadata,
            KeyAction::OpenActions => Self::OpenActions,
            KeyAction::ToggleDate => Self::ToggleDate,
            KeyAction::TogglePwd => Self::TogglePwd,
            KeyAction::ToggleExit => Self::ToggleExit,
            KeyAction::ToggleDuration => Self::ToggleDuration,
            KeyAction::ColumnNext => Self::ColumnNext,
            KeyAction::ColumnMoveLeft => Self::ColumnMoveLeft,
            KeyAction::ColumnMoveRight => Self::ColumnMoveRight,
            KeyAction::ColumnToggle => Self::ColumnToggle,
            KeyAction::SortByColumn => Self::SortByColumn,
            KeyAction::SortDirection => Self::SortDirection,
            KeyAction::SelectNext => Self::SelectNext,
            KeyAction::SelectPrevious => Self::SelectPrevious,
            KeyAction::Accept => Self::Accept,
            KeyAction::Quit => Self::Quit,
            KeyAction::Backspace => Self::Backspace,
            KeyAction::Delete => Self::Delete,
            KeyAction::DeleteWord => Self::DeleteWord,
            KeyAction::ClearQuery => Self::ClearQuery,
            KeyAction::CursorLeft => Self::CursorLeft,
            KeyAction::CursorRight => Self::CursorRight,
            KeyAction::CursorStart => Self::CursorStart,
            KeyAction::CursorEnd => Self::CursorEnd,
        }
    }
}

/// Pure application state for the interactive picker.
pub struct AppModel {
    search: SearchEngine,
    history_count: usize,
    current_pwd: Option<String>,
    git_root: Option<String>,
    query: String,
    query_cursor: usize,
    search_mode: SearchMode,
    pwd_match_mode: PwdMatchMode,
    selected_index: usize,
    visible: Vec<usize>,
    view_mode: ViewMode,
    metadata_visible: bool,
    columns: ColumnConfig,
    focused_column: ColumnId,
    ui_state_revision: u64,
    actions_menu: Option<Menu>,
    should_quit: bool,
    query_error: Option<String>,
    accepted_command: Option<String>,
    menus: BTreeMap<String, Menu>,
    menu_stack: Vec<Menu>,
    active_window: Option<String>,
    windows: BTreeMap<String, WindowConfig>,
    wraps: BTreeMap<String, WrapConfig>,
}

impl AppModel {
    pub fn new(store: HistoryStore, current_pwd: Option<String>) -> Self {
        Self::new_with_environment(store, current_pwd, None, PwdMatchMode::Exact)
    }

    pub fn new_with_config(
        store: HistoryStore,
        current_pwd: Option<String>,
        git_root: Option<String>,
        config: &ColumnConfig,
        pwd_match_mode: PwdMatchMode,
    ) -> Self {
        let mut model = Self::new_with_environment(store, current_pwd, git_root, pwd_match_mode);
        model.columns = config.clone();
        model.focused_column = config
            .normalized_order()
            .first()
            .copied()
            .unwrap_or(ColumnId::Date);
        model.sort_visible();
        model
    }

    pub fn configure_interactions(&mut self, menus: BTreeMap<String, Menu>) {
        self.menus = menus;
    }

    pub fn configure_windows(&mut self, windows: BTreeMap<String, WindowConfig>) {
        self.windows = windows;
    }

    pub fn configure_wraps(&mut self, wraps: BTreeMap<String, WrapConfig>) {
        self.wraps = wraps;
    }

    pub fn new_with_environment(
        store: HistoryStore,
        current_pwd: Option<String>,
        git_root: Option<String>,
        pwd_match_mode: PwdMatchMode,
    ) -> Self {
        let history_count = store.len();
        let search = SearchEngine::new(store, 200);
        let visible = search.results().to_vec();
        Self {
            search,
            history_count,
            current_pwd,
            git_root,
            query: String::new(),
            query_cursor: 0,
            search_mode: SearchMode::All,
            pwd_match_mode,
            selected_index: 0,
            visible,
            view_mode: ViewMode::Search,
            metadata_visible: true,
            columns: ColumnConfig::default(),
            focused_column: ColumnId::Date,
            ui_state_revision: 0,
            actions_menu: None,
            should_quit: false,
            query_error: None,
            accepted_command: None,
            menus: BTreeMap::new(),
            menu_stack: Vec::new(),
            active_window: None,
            windows: BTreeMap::new(),
            wraps: BTreeMap::new(),
        }
    }

    fn insert_character(&mut self, character: char) {
        self.query.insert(self.query_cursor, character);
        self.query_cursor += character.len_utf8();
        self.reset_search_results();
    }

    fn previous_query_boundary(&self) -> Option<usize> {
        self.query[..self.query_cursor]
            .char_indices()
            .next_back()
            .map(|(index, _)| index)
    }

    fn next_query_boundary(&self) -> Option<usize> {
        self.query[self.query_cursor..]
            .chars()
            .next()
            .map(|character| self.query_cursor + character.len_utf8())
    }

    fn restore_search_view(&mut self) {
        let ViewMode::Context { anchor_index, .. } = self.view_mode else {
            return;
        };
        self.leave_context();
        self.refresh_results();
        self.selected_index = self
            .visible
            .iter()
            .position(|&index| index == anchor_index)
            .unwrap_or(0);
    }

    pub fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Input(character) => {
                self.insert_character(character);
            }
            Msg::Paste(input) => {
                let mut changed = false;
                for character in input.chars() {
                    let character = match character {
                        '\r' | '\n' | '\t' => ' ',
                        character if character.is_control() => continue,
                        character => character,
                    };
                    self.query.insert(self.query_cursor, character);
                    self.query_cursor += character.len_utf8();
                    changed = true;
                }
                if changed {
                    self.reset_search_results();
                }
            }
            Msg::Backspace => {
                if let Some(previous) = self.previous_query_boundary() {
                    self.query.drain(previous..self.query_cursor);
                    self.query_cursor = previous;
                    self.reset_search_results();
                }
            }
            Msg::Delete => {
                if let Some(next) = self.next_query_boundary() {
                    self.query.drain(self.query_cursor..next);
                    self.reset_search_results();
                }
            }
            Msg::DeleteWord => {
                let original = self.query_cursor;
                while self
                    .query_before_cursor()
                    .chars()
                    .next_back()
                    .is_some_and(char::is_whitespace)
                {
                    self.query_cursor = self.previous_query_boundary().unwrap_or(0);
                }
                while self
                    .query_before_cursor()
                    .chars()
                    .next_back()
                    .is_some_and(|character| !character.is_whitespace())
                {
                    self.query_cursor = self.previous_query_boundary().unwrap_or(0);
                }
                if self.query_cursor != original {
                    self.query.drain(self.query_cursor..original);
                    self.reset_search_results();
                }
            }
            Msg::ClearQuery => {
                if !self.query.is_empty() {
                    self.query.clear();
                    self.query_cursor = 0;
                    self.reset_search_results();
                }
            }
            Msg::CursorLeft => {
                self.restore_search_view();
                if let Some(previous) = self.previous_query_boundary() {
                    self.query_cursor = previous;
                }
            }
            Msg::CursorRight => {
                self.restore_search_view();
                if let Some(next) = self.next_query_boundary() {
                    self.query_cursor = next;
                }
            }
            Msg::CursorStart => {
                self.restore_search_view();
                self.query_cursor = 0;
            }
            Msg::CursorEnd => {
                self.restore_search_view();
                self.query_cursor = self.query.len();
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
                self.reset_search_results();
            }
            Msg::TogglePwdMatchMode => {
                self.pwd_match_mode = self.pwd_match_mode.toggle();
                self.reset_search_results();
            }
            Msg::ShowGlobal => self.switch_mode(SearchMode::All),
            Msg::ShowPwd => self.switch_mode(SearchMode::SamePwd),
            Msg::ShowGitRoot => self.switch_mode(SearchMode::GitRoot),
            Msg::ToggleContext => {
                if let ViewMode::Context { anchor_index, .. } = self.view_mode {
                    self.leave_context();
                    self.refresh_results();
                    self.selected_index = self
                        .visible
                        .iter()
                        .position(|&index| index == anchor_index)
                        .unwrap_or(0);
                } else if let Some(&anchor_index) = self.visible.get(self.selected_index) {
                    self.view_mode = ViewMode::Context {
                        anchor_index,
                        radius: 1,
                    };
                    self.refresh_context();
                }
            }
            Msg::ContextExpand => {
                if let ViewMode::Context { radius, .. } = &mut self.view_mode {
                    *radius = radius.saturating_add(1);
                    self.refresh_context();
                }
            }
            Msg::ContextShrink => {
                if let ViewMode::Context { radius, .. } = &mut self.view_mode {
                    *radius = radius.saturating_sub(1).max(1);
                    self.refresh_context();
                }
            }
            Msg::ToggleMetadata => self.metadata_visible = !self.metadata_visible,
            Msg::OpenActions => {
                if self.menus.contains_key("actions") {
                    self.open_named_menu("actions");
                } else {
                    self.actions_menu = Some(default_actions_menu());
                }
            }
            Msg::ToggleDate => self.toggle_column(ColumnId::Date),
            Msg::TogglePwd => self.toggle_column(ColumnId::Pwd),
            Msg::ToggleExit => self.toggle_column(ColumnId::Exit),
            Msg::ToggleDuration => self.toggle_column(ColumnId::Duration),
            Msg::ColumnNext => self.focus_next_column(),
            Msg::ColumnMoveLeft => self.move_focused_column(false),
            Msg::ColumnMoveRight => self.move_focused_column(true),
            Msg::ColumnToggle => self.toggle_column(self.focused_column),
            Msg::SortByColumn => {
                self.columns.sort_by = SortField::from_column(self.focused_column);
                self.bump_ui_state_revision();
                self.resort_visible_preserving_selection();
            }
            Msg::SortDirection => {
                self.columns.sort_direction = self.columns.sort_direction.toggle();
                self.bump_ui_state_revision();
                self.resort_visible_preserving_selection();
            }
            Msg::OpenLocation => self.open_window("location"),
            Msg::OpenTimeline => self.open_window("timeline"),
            Msg::Accept => {
                self.accepted_command = self.selected().map(|entry| entry.command.clone());
                self.should_quit = true;
            }
            Msg::Quit => self.should_quit = true,
        }
    }

    fn switch_mode(&mut self, mode: SearchMode) {
        self.search_mode = mode;
        self.reset_search_results();
    }

    fn reset_search_results(&mut self) {
        self.leave_context();
        self.selected_index = 0;
        self.refresh_results();
    }

    fn leave_context(&mut self) {
        self.view_mode = ViewMode::Search;
    }

    fn scope(&self) -> SearchScope {
        match self.search_mode {
            SearchMode::All => SearchScope::global(),
            SearchMode::SamePwd => {
                SearchScope::pwd(self.current_pwd.as_deref(), self.pwd_match_mode)
            }
            SearchMode::GitRoot => SearchScope::git_root(self.git_root.as_deref()),
        }
    }

    fn refresh_results(&mut self) {
        self.search.set_scope(self.scope());
        if self.search.query() != self.query {
            match self.search.try_set_query(&self.query) {
                Ok(()) => self.query_error = None,
                Err(error) => self.query_error = Some(error.to_string()),
            }
        }
        self.visible.clear();
        self.visible.extend_from_slice(self.search.results());
        self.sort_visible();
        if self.selected_index >= self.visible.len() {
            self.selected_index = self.visible.len().saturating_sub(1);
        }
    }

    fn sort_visible(&mut self) {
        if matches!(self.view_mode, ViewMode::Context { .. }) {
            return;
        }
        let direction = self.columns.sort_direction;
        if self.columns.sort_by == SortField::Relevance {
            if direction == SortDirection::Ascending {
                self.visible.reverse();
            }
            return;
        }
        let field = self.columns.sort_by;
        let search = &self.search;
        self.visible.sort_by(|left, right| {
            let left_entry = search.entry(*left);
            let right_entry = search.entry(*right);
            let order = match field {
                SortField::Relevance => std::cmp::Ordering::Equal,
                SortField::Date => left_entry.timestamp.cmp(&right_entry.timestamp),
                SortField::Pwd => left_entry.cwd.cmp(&right_entry.cwd),
                SortField::Exit => left_entry.exit.cmp(&right_entry.exit),
                SortField::Duration => left_entry.duration.cmp(&right_entry.duration),
                SortField::Command => left_entry.command.cmp(&right_entry.command),
            }
            .then_with(|| left_entry.timestamp.cmp(&right_entry.timestamp))
            .then_with(|| left_entry.id.cmp(&right_entry.id));
            if direction == SortDirection::Ascending {
                order
            } else {
                order.reverse()
            }
        });
    }

    fn resort_visible_preserving_selection(&mut self) {
        let selected_id = self.selected_id().map(str::to_owned);
        self.sort_visible();
        if let Some(id) = selected_id
            && let Some(index) = self
                .visible
                .iter()
                .position(|&index| self.search.entry(index).id == id)
        {
            self.selected_index = index;
        }
    }

    fn bump_ui_state_revision(&mut self) {
        self.ui_state_revision = self.ui_state_revision.wrapping_add(1);
    }

    fn focus_next_column(&mut self) {
        let order = self.columns.normalized_order();
        let current = order
            .iter()
            .position(|&column| column == self.focused_column)
            .unwrap_or(0);
        self.focused_column = order[(current + 1) % order.len()];
    }

    fn move_focused_column(&mut self, right: bool) {
        self.columns.order = self.columns.normalized_order();
        let Some(index) = self
            .columns
            .order
            .iter()
            .position(|&column| column == self.focused_column)
        else {
            return;
        };
        let target = if right {
            (index + 1).min(self.columns.order.len().saturating_sub(1))
        } else {
            index.saturating_sub(1)
        };
        if target != index {
            self.columns.order.swap(index, target);
            self.bump_ui_state_revision();
        }
    }

    fn toggle_column(&mut self, column: ColumnId) {
        let value = match column {
            ColumnId::Date => &mut self.columns.date,
            ColumnId::Pwd => &mut self.columns.pwd,
            ColumnId::Exit => &mut self.columns.exit,
            ColumnId::Duration => &mut self.columns.duration,
        };
        *value = !*value;
        self.bump_ui_state_revision();
    }

    fn refresh_context(&mut self) {
        let (anchor_index, radius) = match self.view_mode {
            ViewMode::Search => return,
            ViewMode::Context {
                anchor_index,
                radius,
            } => (anchor_index, radius),
        };
        if let Some(context) = self.search.context_around_index(anchor_index, radius) {
            self.visible = context;
            self.selected_index = self
                .visible
                .iter()
                .position(|&index| index == anchor_index)
                .unwrap_or(0);
        } else {
            self.leave_context();
            self.refresh_results();
        }
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn query_cursor(&self) -> usize {
        self.query_cursor
    }

    pub fn query_before_cursor(&self) -> &str {
        &self.query[..self.query_cursor]
    }

    pub fn history_count(&self) -> usize {
        self.history_count
    }

    pub fn search_mode(&self) -> SearchMode {
        self.search_mode
    }

    pub fn pwd_match_mode(&self) -> PwdMatchMode {
        self.pwd_match_mode
    }

    pub fn context_radius(&self) -> usize {
        match &self.view_mode {
            ViewMode::Search => 1,
            ViewMode::Context { radius, .. } => *radius,
        }
    }

    pub fn in_context_mode(&self) -> bool {
        matches!(self.view_mode, ViewMode::Context { .. })
    }

    pub fn metadata_visible(&self) -> bool {
        self.metadata_visible
    }

    pub fn column_visible(&self, column: ColumnId) -> bool {
        if !self.metadata_visible {
            return false;
        }
        match column {
            ColumnId::Date => self.columns.date,
            ColumnId::Pwd => self.columns.pwd,
            ColumnId::Exit => self.columns.exit,
            ColumnId::Duration => self.columns.duration,
        }
    }

    pub fn columns(&self) -> &ColumnConfig {
        &self.columns
    }

    pub fn focused_column(&self) -> ColumnId {
        self.focused_column
    }

    pub fn sort_field(&self) -> SortField {
        self.columns.sort_by
    }

    pub fn sort_direction(&self) -> SortDirection {
        self.columns.sort_direction
    }

    pub fn ui_state_revision(&self) -> u64 {
        self.ui_state_revision
    }

    pub fn ui_state(&self) -> UiState {
        UiState::from_columns(&self.columns)
    }

    pub fn selected_id(&self) -> Option<&str> {
        self.selected().map(|entry| entry.id.as_str())
    }

    /// Replace the immutable history snapshot while preserving the active query
    /// and selected identity whenever that identity still exists.
    pub fn replace_history(&mut self, store: HistoryStore) {
        let selected_id = self.selected_id().map(str::to_owned);
        let old_index = self.selected_index;
        self.search = SearchEngine::new(store, 200);
        self.history_count = self.search.history_count();
        self.refresh_results();
        if let Some(id) = selected_id
            && let Some(index) = self
                .visible
                .iter()
                .position(|&index| self.search.entry(index).id == id)
        {
            self.selected_index = index;
            return;
        }
        if !self.visible.is_empty() {
            self.selected_index = old_index.min(self.visible.len() - 1);
        }
    }

    pub fn actions_menu(&self) -> Option<&Menu> {
        self.menu_stack.last().or(self.actions_menu.as_ref())
    }

    pub fn actions_menu_mut(&mut self) -> Option<&mut Menu> {
        self.menu_stack.last_mut().or(self.actions_menu.as_mut())
    }

    pub fn close_actions_menu(&mut self) {
        self.menu_stack.clear();
        self.actions_menu = None;
        self.active_window = None;
    }

    pub fn modal_open(&self) -> bool {
        self.actions_menu.is_some() || !self.menu_stack.is_empty() || self.active_window.is_some()
    }

    pub fn open_named_menu(&mut self, name: &str) {
        if let Some(menu) = self.menus.get(name).cloned() {
            let menu_name = menu.name.clone();
            let hooks = menu.on_open.clone();
            self.menu_stack.push(menu);
            self.actions_menu = None;
            self.run_lifecycle_actions(&hooks);
            let item_hooks = self
                .menu_stack
                .last()
                .filter(|menu| menu.name == menu_name)
                .and_then(Menu::selected_item)
                .map(|item| item.on_select.clone())
                .unwrap_or_default();
            self.run_lifecycle_actions(&item_hooks);
        }
    }

    pub fn open_window(&mut self, name: &str) {
        if let Some(window) = self.windows.get(name) {
            let hooks = window.on_open.clone();
            self.active_window = Some(name.to_owned());
            self.menu_stack.clear();
            self.actions_menu = None;
            self.run_lifecycle_actions(&hooks);
        } else if matches!(name, "location" | "timeline" | "preview" | "inspect") {
            self.active_window = Some(name.to_owned());
            self.menu_stack.clear();
            self.actions_menu = None;
        }
    }

    pub fn active_window(&self) -> Option<&str> {
        self.active_window.as_deref()
    }

    pub fn active_window_config(&self) -> Option<&WindowConfig> {
        self.active_window
            .as_ref()
            .and_then(|name| self.windows.get(name))
    }

    pub fn close_modal(&mut self) {
        if let Some(menu) = self.menu_stack.pop() {
            let item_hooks = menu
                .selected_item()
                .map(|item| item.on_leave.clone())
                .unwrap_or_default();
            let hooks = menu.on_leave.clone();
            self.run_lifecycle_actions(&item_hooks);
            self.run_lifecycle_actions(&hooks);
            return;
        }
        if let Some(name) = self.active_window.take() {
            if let Some(window) = self.windows.get(&name) {
                let hooks = window.on_close.clone();
                self.run_lifecycle_actions(&hooks);
            }
            return;
        }
        if let Some(menu) = self.actions_menu.take() {
            let item_hooks = menu
                .selected_item()
                .map(|item| item.on_leave.clone())
                .unwrap_or_default();
            self.run_lifecycle_actions(&item_hooks);
        }
    }

    fn run_lifecycle_actions(&mut self, actions: &[Action]) {
        for action in actions {
            match action {
                Action::Menu(name) => self.open_named_menu(name),
                Action::Window(name) => self.open_window(name),
                Action::Wrap(_)
                | Action::InsertExit
                | Action::Append
                | Action::AppendExit
                | Action::AndAppend
                | Action::OrAppend => {}
            }
        }
    }

    pub fn execute_menu_action(&mut self) {
        let Some(menu) = self.menu_stack.last().or(self.actions_menu.as_ref()) else {
            return;
        };
        let Some(item) = menu.selected_item() else {
            return;
        };
        let on_leave = item.on_leave.clone();
        let action = item.action.clone();
        self.run_lifecycle_actions(&on_leave);
        self.execute_action(action);
    }

    pub fn execute_menu_item(&mut self, index: usize) {
        let Some(menu) = self.actions_menu_mut() else {
            return;
        };
        if index >= menu.items.len() {
            return;
        }
        let selection_changed = menu.selected != index;
        let previous_leave = if selection_changed {
            menu.selected_item()
                .map(|item| item.on_leave.clone())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        menu.selected = index;
        let item = &menu.items[index];
        let on_select = if selection_changed {
            item.on_select.clone()
        } else {
            Vec::new()
        };
        let on_leave = item.on_leave.clone();
        let action = item.action.clone();
        self.run_lifecycle_actions(&previous_leave);
        self.run_lifecycle_actions(&on_select);
        self.run_lifecycle_actions(&on_leave);
        self.execute_action(action);
    }

    fn execute_action(&mut self, action: Action) {
        match action {
            Action::InsertExit => {
                self.accepted_command = self.selected().map(|entry| entry.command.clone());
                self.actions_menu = None;
                self.should_quit = true;
            }
            Action::Append => {
                let command = self.selected().map(|entry| entry.command.clone());
                if let Some(command) = command {
                    if !self.query.is_empty() {
                        self.query.push(' ');
                    }
                    self.query.push_str(&command);
                    self.query_cursor = self.query.len();
                    self.actions_menu = None;
                    self.reset_search_results();
                }
            }
            Action::AppendExit | Action::AndAppend | Action::OrAppend => {
                if let Some(entry) = self.selected() {
                    let composed = action.compose(&self.query, entry);
                    self.accepted_command = Some(composed);
                    self.actions_menu = None;
                    self.should_quit = true;
                }
            }
            Action::Menu(name) => self.open_named_menu(&name),
            Action::Window(name) => self.open_window(&name),
            Action::Wrap(name) => {
                if let Some(entry) = self.selected()
                    && let Some(template) = self.wraps.get(&name)
                {
                    let composed = template
                        .template
                        .replace("{command}", &entry.command)
                        .replace("{query}", &self.query);
                    self.accepted_command = Some(composed);
                    self.should_quit = true;
                }
            }
        }
    }

    pub fn inspect_entries(&self, kind: &str) -> Vec<&HistoryEntry> {
        let Some(&selected_index) = self.visible.get(self.selected_index) else {
            return Vec::new();
        };
        let scope = if kind == "location" {
            SearchScope::pwd(
                self.selected().map(|entry| entry.cwd.as_str()),
                self.pwd_match_mode,
            )
        } else {
            SearchScope::global()
        };
        self.search
            .context_around_index_with_scope(selected_index, 3, &scope)
            .unwrap_or_default()
            .into_iter()
            .map(|index| self.search.entry(index))
            .collect()
    }

    pub fn menu_stack_len(&self) -> usize {
        self.menu_stack.len()
    }

    pub fn query_error(&self) -> Option<&str> {
        self.query_error.as_deref()
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn accepted_command(&self) -> Option<&str> {
        self.accepted_command.as_deref()
    }

    pub fn visible(&self) -> impl ExactSizeIterator<Item = &HistoryEntry> + '_ {
        self.visible.iter().map(|&index| self.search.entry(index))
    }

    pub fn visible_len(&self) -> usize {
        self.visible.len()
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn selected(&self) -> Option<&HistoryEntry> {
        self.visible
            .get(self.selected_index)
            .map(|&index| self.search.entry(index))
    }

    pub fn visible_commands(&self) -> Vec<&str> {
        self.visible().map(|entry| entry.command.as_str()).collect()
    }

    pub fn search_stats(&self) -> SearchStats {
        self.search.stats()
    }

    pub fn handle_modal_next(&mut self) {
        self.move_modal_selection(true);
    }

    pub fn handle_modal_previous(&mut self) {
        self.move_modal_selection(false);
    }

    fn move_modal_selection(&mut self, next: bool) {
        let hooks = {
            let Some(menu) = self.actions_menu_mut() else {
                return;
            };
            let previous = menu.selected;
            if next {
                menu.next();
            } else {
                menu.previous();
            }
            if menu.selected == previous {
                return;
            }
            let on_leave = menu.items[previous].on_leave.clone();
            let on_select = menu.items[menu.selected].on_select.clone();
            (on_leave, on_select)
        };
        self.run_lifecycle_actions(&hooks.0);
        self.run_lifecycle_actions(&hooks.1);
    }
}
