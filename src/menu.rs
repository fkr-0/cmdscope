use crate::{Action, config::MenuConfig};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    pub label: String,
    pub key: Option<String>,
    pub action: Action,
    pub on_select: Vec<Action>,
    pub on_leave: Vec<Action>,
}
impl MenuItem {
    fn from_config(c: &crate::config::MenuItemConfig) -> Self {
        Self {
            label: c.label.clone(),
            key: c.key.clone(),
            action: c.action.clone(),
            on_select: c.on_select.clone(),
            on_leave: c.on_leave.clone(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu {
    pub name: String,
    pub items: Vec<MenuItem>,
    pub selected: usize,
    pub confirm: Vec<crate::keymap::KeyChord>,
    pub next: Vec<crate::keymap::KeyChord>,
    pub previous: Vec<crate::keymap::KeyChord>,
    pub cancel: Vec<crate::keymap::KeyChord>,
    pub on_open: Vec<Action>,
    pub on_leave: Vec<Action>,
}
impl Menu {
    pub fn new(name: impl Into<String>, items: Vec<MenuItem>) -> Self {
        Self {
            name: name.into(),
            items,
            selected: 0,
            confirm: Vec::new(),
            next: Vec::new(),
            previous: Vec::new(),
            cancel: Vec::new(),
            on_open: Vec::new(),
            on_leave: Vec::new(),
        }
    }
    pub fn from_config(c: &MenuConfig) -> anyhow::Result<Self> {
        let parse = |v: &[String]| {
            v.iter()
                .map(|x| x.parse())
                .collect::<anyhow::Result<Vec<_>>>()
        };
        Ok(Self {
            name: c.name.clone(),
            items: c.items.iter().map(MenuItem::from_config).collect(),
            selected: 0,
            confirm: parse(&c.confirm)?,
            next: parse(&c.next)?,
            previous: parse(&c.previous)?,
            cancel: parse(&c.cancel)?,
            on_open: c.on_open.clone(),
            on_leave: c.on_leave.clone(),
        })
    }
    pub fn selected_item(&self) -> Option<&MenuItem> {
        self.items.get(self.selected)
    }
    pub fn next(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + 1).min(self.items.len() - 1)
        }
    }
    pub fn previous(&mut self) {
        self.selected = self.selected.saturating_sub(1)
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
            item("inspect", "window:timeline"),
        ],
    )
}
fn item(label: &str, action: &str) -> MenuItem {
    MenuItem {
        label: label.into(),
        key: None,
        action: Action::parse(action),
        on_select: Vec::new(),
        on_leave: Vec::new(),
    }
}
