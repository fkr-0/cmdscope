use crate::HistoryEntry;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    InsertExit,
    Append,
    AppendExit,
    AndAppend,
    OrAppend,
    Menu(String),
    Window(String),
    Wrap(String),
}
impl<'de> Deserialize<'de> for Action {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse_checked(&value).map_err(serde::de::Error::custom)
    }
}
impl Action {
    pub fn parse_checked(name: &str) -> anyhow::Result<Self> {
        let action = Self::parse(name);
        if matches!(action, Self::InsertExit) && name != "insert+exit" {
            anyhow::bail!("unknown action {name:?}")
        }
        match &action {
            Self::Menu(v) | Self::Window(v) | Self::Wrap(v) if v.is_empty() => {
                anyhow::bail!("action {name:?} requires a non-empty target")
            }
            _ => {}
        }
        Ok(action)
    }
    pub fn parse(name: &str) -> Self {
        match name {
            "insert+exit" => Self::InsertExit,
            "append" => Self::Append,
            "append+exit" => Self::AppendExit,
            "&&append" => Self::AndAppend,
            "||append" => Self::OrAppend,
            v if v.starts_with("menu:") => Self::Menu(v[5..].into()),
            v if v.starts_with("window:") => Self::Window(v[7..].into()),
            v if v.starts_with("wrap:") => Self::Wrap(v[5..].into()),
            _ => Self::InsertExit,
        }
    }
    pub fn is_safe_internal(&self) -> bool {
        matches!(
            self,
            Self::InsertExit
                | Self::Append
                | Self::AppendExit
                | Self::AndAppend
                | Self::OrAppend
                | Self::Menu(_)
                | Self::Window(_)
                | Self::Wrap(_)
        )
    }
    pub fn compose(&self, query: &str, selected: &HistoryEntry) -> String {
        match self {
            Self::InsertExit | Self::Append | Self::Wrap(_) | Self::Window(_) | Self::Menu(_) => {
                selected.command.clone()
            }
            Self::AppendExit => append(query, &selected.command, " "),
            Self::AndAppend => append(query, &selected.command, " && "),
            Self::OrAppend => append(query, &selected.command, " || "),
        }
    }
}
fn append(query: &str, command: &str, separator: &str) -> String {
    if query.is_empty() {
        command.to_string()
    } else {
        format!("{query}{separator}{command}")
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionContext<'a> {
    pub selected: Option<&'a HistoryEntry>,
    pub query: &'a str,
}
impl<'a> ActionContext<'a> {
    pub fn compose(&self, action: &Action) -> Option<String> {
        self.selected.map(|entry| action.compose(self.query, entry))
    }
}
