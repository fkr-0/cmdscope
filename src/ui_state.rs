use crate::{ColumnConfig, ColumnId};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    #[default]
    Relevance,
    Date,
    Pwd,
    Exit,
    Duration,
    Command,
}

impl SortField {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Relevance => "relevance",
            Self::Date => "date",
            Self::Pwd => "pwd",
            Self::Exit => "exit",
            Self::Duration => "duration",
            Self::Command => "command",
        }
    }

    pub const fn from_column(column: ColumnId) -> Self {
        match column {
            ColumnId::Date => Self::Date,
            ColumnId::Pwd => Self::Pwd,
            ColumnId::Exit => Self::Exit,
            ColumnId::Duration => Self::Duration,
        }
    }
}

impl std::fmt::Display for SortField {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for SortField {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "relevance" | "rank" => Ok(Self::Relevance),
            "date" | "time" | "timestamp" => Ok(Self::Date),
            "pwd" | "cwd" => Ok(Self::Pwd),
            "exit" | "status" => Ok(Self::Exit),
            "duration" => Ok(Self::Duration),
            "command" | "cmd" => Ok(Self::Command),
            other => anyhow::bail!(
                "unknown sort field {other:?}; expected relevance, date, pwd, exit, duration, or command"
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    Ascending,
    #[default]
    Descending,
}

impl SortDirection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ascending => "ascending",
            Self::Descending => "descending",
        }
    }

    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Ascending => "↑",
            Self::Descending => "↓",
        }
    }

    pub const fn toggle(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

impl std::fmt::Display for SortDirection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for SortDirection {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "asc" | "ascending" | "up" => Ok(Self::Ascending),
            "desc" | "descending" | "down" => Ok(Self::Descending),
            other => anyhow::bail!(
                "unknown sort direction {other:?}; expected ascending/asc or descending/desc"
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiState {
    #[serde(default = "ui_state_version")]
    pub version: u8,
    pub order: Vec<ColumnId>,
    pub visible: Vec<ColumnId>,
    #[serde(default)]
    pub sort_by: SortField,
    #[serde(default)]
    pub sort_direction: SortDirection,
}

const fn ui_state_version() -> u8 {
    1
}

impl Default for UiState {
    fn default() -> Self {
        Self::from_columns(&ColumnConfig::default())
    }
}

impl UiState {
    pub fn from_columns(columns: &ColumnConfig) -> Self {
        Self {
            version: ui_state_version(),
            order: columns.normalized_order(),
            visible: columns.visible(),
            sort_by: columns.sort_by,
            sort_direction: columns.sort_direction,
        }
    }

    pub fn apply_to(&self, columns: &mut ColumnConfig) -> Result<()> {
        if self.version != ui_state_version() {
            anyhow::bail!(
                "unsupported cmdscope UI state version {}; expected {}",
                self.version,
                ui_state_version()
            );
        }
        columns.order = normalize_order(&self.order)?;
        columns.date = self.visible.contains(&ColumnId::Date);
        columns.pwd = self.visible.contains(&ColumnId::Pwd);
        columns.exit = self.visible.contains(&ColumnId::Exit);
        columns.duration = self.visible.contains(&ColumnId::Duration);
        columns.sort_by = self.sort_by;
        columns.sort_direction = self.sort_direction;
        Ok(())
    }

    pub fn load_optional(path: impl AsRef<Path>) -> Result<Option<Self>> {
        let path = path.as_ref();
        match fs::read_to_string(path) {
            Ok(input) => {
                let state = toml::from_str(&input)
                    .with_context(|| format!("failed to parse UI state {}", path.display()))?;
                Ok(Some(state))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => {
                Err(error).with_context(|| format!("failed to read UI state {}", path.display()))
            }
        }
    }

    pub fn save_atomic(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create UI state directory {}", parent.display()))?;
        let input = toml::to_string_pretty(self).context("failed to serialize UI state")?;
        let temporary = temporary_path(path);
        fs::write(&temporary, input)
            .with_context(|| format!("failed to write UI state {}", temporary.display()))?;
        if let Err(error) = fs::rename(&temporary, path) {
            let _ = fs::remove_file(&temporary);
            return Err(error)
                .with_context(|| format!("failed to replace UI state {}", path.display()));
        }
        Ok(())
    }
}

fn normalize_order(order: &[ColumnId]) -> Result<Vec<ColumnId>> {
    let mut normalized = Vec::with_capacity(ColumnId::ALL.len());
    for &column in order {
        if normalized.contains(&column) {
            anyhow::bail!("column {column} appears more than once in persisted UI state");
        }
        normalized.push(column);
    }
    for column in ColumnId::ALL {
        if !normalized.contains(&column) {
            normalized.push(column);
        }
    }
    Ok(normalized)
}

fn temporary_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("ui-state.toml");
    path.with_file_name(format!(".{file_name}.tmp-{}", std::process::id()))
}
