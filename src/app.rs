use crate::{
    HistoryEntry, HistoryStore, KeyAction, PwdMatchMode, SearchEngine, SearchMode, SearchScope,
    SearchStats,
};

/// Pure update messages accepted by [`AppModel::update`].
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
    ToggleMetadata,
    Accept,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ViewMode {
    Search,
    Context { anchor_id: String, radius: usize },
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
            KeyAction::SelectNext => Self::SelectNext,
            KeyAction::SelectPrevious => Self::SelectPrevious,
            KeyAction::Accept => Self::Accept,
            KeyAction::Quit => Self::Quit,
            KeyAction::Backspace => Self::Backspace,
        }
    }
}

/// Pure application state for the interactive picker.
pub struct AppModel {
    search: SearchEngine,
    current_pwd: Option<String>,
    git_root: Option<String>,
    query: String,
    search_mode: SearchMode,
    pwd_match_mode: PwdMatchMode,
    selected_index: usize,
    visible: Vec<usize>,
    view_mode: ViewMode,
    metadata_visible: bool,
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
        let search = SearchEngine::new(store, 200);
        let visible = search.results().to_vec();
        Self {
            search,
            current_pwd,
            git_root,
            query: String::new(),
            search_mode: SearchMode::All,
            pwd_match_mode,
            selected_index: 0,
            visible,
            view_mode: ViewMode::Search,
            metadata_visible: true,
            should_quit: false,
            accepted_command: None,
        }
    }

    pub fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Input(character) => {
                self.query.push(character);
                self.reset_search_results();
            }
            Msg::Backspace => {
                self.query.pop();
                self.reset_search_results();
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
                if let ViewMode::Context { anchor_id, .. } = &self.view_mode {
                    let anchor_id = anchor_id.clone();
                    self.leave_context();
                    self.refresh_results();
                    self.selected_index = self
                        .visible
                        .iter()
                        .position(|&index| self.search.entry(index).id == anchor_id)
                        .unwrap_or(0);
                } else if let Some(selected) = self.selected() {
                    self.view_mode = ViewMode::Context {
                        anchor_id: selected.id.clone(),
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
        self.search.set_query(&self.query);
        self.visible.clear();
        self.visible.extend_from_slice(self.search.results());
        if self.selected_index >= self.visible.len() {
            self.selected_index = self.visible.len().saturating_sub(1);
        }
    }

    fn refresh_context(&mut self) {
        let (anchor_id, radius) = match &self.view_mode {
            ViewMode::Search => return,
            ViewMode::Context { anchor_id, radius } => (anchor_id.clone(), *radius),
        };
        if let Some(context) = self.search.context_around(&anchor_id, radius) {
            self.visible = context;
            self.selected_index = self
                .visible
                .iter()
                .position(|&index| self.search.entry(index).id == anchor_id)
                .unwrap_or(0);
        } else {
            self.leave_context();
            self.refresh_results();
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
}
