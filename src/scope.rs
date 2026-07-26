use serde::Deserialize;

/// How a directory-constrained search compares recorded working directories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PwdMatchMode {
    #[default]
    Exact,
    #[serde(alias = "subdirs")]
    IncludeSubdirs,
}

impl PwdMatchMode {
    /// Stable short label used by the TUI.
    pub fn label(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::IncludeSubdirs => "subdirs",
        }
    }

    pub(crate) fn toggle(self) -> Self {
        match self {
            Self::Exact => Self::IncludeSubdirs,
            Self::IncludeSubdirs => Self::Exact,
        }
    }
}

/// User-facing search mode selected by shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchMode {
    All,
    SamePwd,
    GitRoot,
}

impl SearchMode {
    /// Stable short label used by status text.
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "global",
            Self::SamePwd => "pwd",
            Self::GitRoot => "git root",
        }
    }
}

/// Concrete scope used by the history index and search engine.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
            pwd: pwd.map(normalized_path_key),
            mode,
        }
    }

    pub fn git_root(root: Option<&str>) -> Self {
        Self::GitRoot {
            root: root.map(normalized_path_key),
        }
    }

    pub(crate) fn matches_cwd(&self, cwd: &str) -> bool {
        match self {
            Self::Global => true,
            Self::Pwd {
                pwd: Some(pwd),
                mode,
            } => path_matches(pwd, cwd, *mode),
            Self::Pwd { pwd: None, .. } => true,
            Self::GitRoot { root: Some(root) } => {
                path_matches(root, cwd, PwdMatchMode::IncludeSubdirs)
            }
            Self::GitRoot { root: None } => true,
        }
    }
}

pub(crate) fn normalized_path_key(path: &str) -> String {
    trim_trailing_slashes(path).to_string()
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
