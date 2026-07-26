use crate::{SearchEngine, SearchMode, SearchScope, scope::normalized_path_key};
use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use std::{collections::HashMap, path::Path, sync::Arc};

/// One shell-history row loaded from an Atuin-compatible `history` table.
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

fn simple_scope(mode: SearchMode, current_pwd: Option<&str>) -> SearchScope {
    match mode {
        SearchMode::All => SearchScope::global(),
        SearchMode::SamePwd => SearchScope::pwd(current_pwd, crate::PwdMatchMode::Exact),
        SearchMode::GitRoot => SearchScope::git_root(current_pwd),
    }
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

/// Immutable, shareable history corpus with lightweight path and id indexes.
#[derive(Debug, Clone)]
pub struct HistoryStore {
    entries: Arc<[HistoryEntry]>,
    cwd_index: Arc<HashMap<String, Arc<[usize]>>>,
    id_index: Arc<HashMap<String, usize>>,
}

impl HistoryStore {
    pub fn from_entries(mut entries: Vec<HistoryEntry>) -> Self {
        entries.sort_by_key(|entry| entry.timestamp);

        let mut cwd_index = HashMap::<String, Vec<usize>>::new();
        let mut id_index = HashMap::with_capacity(entries.len());
        for (index, entry) in entries.iter().enumerate() {
            cwd_index
                .entry(normalized_path_key(&entry.cwd))
                .or_default()
                .push(index);
            id_index.insert(entry.id.clone(), index);
        }

        Self {
            entries: Arc::from(entries),
            cwd_index: Arc::new(
                cwd_index
                    .into_iter()
                    .map(|(cwd, indices)| (cwd, Arc::from(indices)))
                    .collect(),
            ),
            id_index: Arc::new(id_index),
        }
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

    pub fn entry(&self, index: usize) -> &HistoryEntry {
        &self.entries[index]
    }

    pub(crate) fn index_for_id(&self, id: &str) -> Option<usize> {
        self.id_index.get(id).copied()
    }

    /// Compatibility one-shot search. Interactive use should reuse [`SearchEngine`].
    pub fn search(
        &self,
        query: &str,
        mode: SearchMode,
        current_pwd: Option<&str>,
        limit: usize,
    ) -> Vec<HistoryEntry> {
        let scope = simple_scope(mode, current_pwd);
        self.search_with_scope(query, &scope, limit)
    }

    pub fn search_with_scope(
        &self,
        query: &str,
        scope: &SearchScope,
        limit: usize,
    ) -> Vec<HistoryEntry> {
        let mut engine = SearchEngine::new(self.clone(), limit);
        engine.set_scope(scope.clone());
        engine.set_query(query);
        engine
            .results()
            .iter()
            .map(|&index| self.entry(index).clone())
            .collect()
    }

    pub fn context_around_with_scope(
        &self,
        selected_id: &str,
        radius: usize,
        scope: &SearchScope,
    ) -> Option<Vec<HistoryEntry>> {
        self.context_indices(selected_id, radius, scope)
            .map(|indices| {
                indices
                    .into_iter()
                    .map(|index| self.entry(index).clone())
                    .collect()
            })
    }

    pub fn context_around(
        &self,
        selected_id: &str,
        radius: usize,
        mode: SearchMode,
        current_pwd: Option<&str>,
    ) -> Option<Vec<HistoryEntry>> {
        let scope = simple_scope(mode, current_pwd);
        self.context_around_with_scope(selected_id, radius, &scope)
    }

    pub(crate) fn indices_for_scope(&self, scope: &SearchScope) -> Vec<usize> {
        match scope {
            SearchScope::Global => (0..self.entries.len()).collect(),
            SearchScope::Pwd { pwd: None, .. } | SearchScope::GitRoot { root: None } => Vec::new(),
            SearchScope::Pwd {
                pwd: Some(pwd),
                mode: crate::PwdMatchMode::Exact,
            } => self
                .cwd_index
                .get(pwd)
                .map(|indices| indices.to_vec())
                .unwrap_or_default(),
            SearchScope::Pwd { .. } | SearchScope::GitRoot { .. } => {
                let mut indices = self
                    .cwd_index
                    .iter()
                    .filter(|(cwd, _)| scope.matches_cwd(cwd))
                    .flat_map(|(_, indices)| indices.iter().copied())
                    .collect::<Vec<_>>();
                indices.sort_unstable();
                indices
            }
        }
    }

    pub(crate) fn context_indices(
        &self,
        selected_id: &str,
        radius: usize,
        scope: &SearchScope,
    ) -> Option<Vec<usize>> {
        let selected_index = self.index_for_id(selected_id)?;
        let scoped = self.indices_for_scope(scope);
        let position = scoped.binary_search(&selected_index).ok()?;
        let start = position.saturating_sub(radius);
        let end = position
            .saturating_add(radius)
            .saturating_add(1)
            .min(scoped.len());
        Some(scoped[start..end].to_vec())
    }
}
