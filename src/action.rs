use crate::HistoryEntry;

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

impl Action {
    pub fn parse(name: &str) -> Self {
        match name {
            "insert+exit" => Self::InsertExit,
            "append" => Self::Append,
            "append+exit" => Self::AppendExit,
            "&&append" => Self::AndAppend,
            "||append" => Self::OrAppend,
            value if value.starts_with("menu:") => Self::Menu(value[5..].to_string()),
            value if value.starts_with("window:") => Self::Window(value[7..].to_string()),
            value if value.starts_with("wrap:") => Self::Wrap(value[5..].to_string()),
            _ => Self::InsertExit,
        }
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
