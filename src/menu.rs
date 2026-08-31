use crate::action::Action;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    pub label: String,
    pub key: Option<String>,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu {
    pub name: String,
    pub items: Vec<MenuItem>,
    pub selected: usize,
}

impl Menu {
    pub fn new(name: impl Into<String>, items: Vec<MenuItem>) -> Self {
        Self {
            name: name.into(),
            items,
            selected: 0,
        }
    }

    pub fn selected_item(&self) -> Option<&MenuItem> {
        self.items.get(self.selected)
    }

    pub fn next(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + 1).min(self.items.len() - 1);
        }
    }

    pub fn previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }
}

pub fn default_actions_menu() -> Menu {
    Menu::new(
        "actions",
        vec![
            item("insert + exit", "insert+exit"),
            item("append", "append"),
            item("append + exit", "append+exit"),
            item("&& append", "&&append"),
            item("|| append", "||append"),
        ],
    )
}

fn item(label: &str, action: &str) -> MenuItem {
    MenuItem {
        label: label.to_string(),
        key: None,
        action: Action::parse(action),
    }
}
