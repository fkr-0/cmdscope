use crate::config::KeyConfig;
use anyhow::{Result, bail};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{collections::HashMap, fmt, str::FromStr};

/// A semantic action produced by the compiled key map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyAction {
    ShowGlobal,
    ShowPwd,
    ShowGitRoot,
    ToggleScope,
    TogglePwdMode,
    ToggleContext,
    ContextExpand,
    ContextShrink,
    ToggleMetadata,
    SelectNext,
    SelectPrevious,
    Accept,
    Quit,
    Backspace,
}

impl KeyAction {
    fn name(self) -> &'static str {
        match self {
            Self::ShowGlobal => "global",
            Self::ShowPwd => "pwd",
            Self::ShowGitRoot => "git_root",
            Self::ToggleScope => "toggle_scope",
            Self::TogglePwdMode => "toggle_pwd_mode",
            Self::ToggleContext => "context",
            Self::ContextExpand => "context_expand",
            Self::ContextShrink => "context_shrink",
            Self::ToggleMetadata => "toggle_metadata",
            Self::SelectNext => "select_next",
            Self::SelectPrevious => "select_previous",
            Self::Accept => "accept",
            Self::Quit => "quit",
            Self::Backspace => "backspace",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ChordCode {
    Char(char),
    Enter,
    Esc,
    Tab,
    Backspace,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
    Insert,
}

/// Parsed, normalized key chord such as `ctrl-g` or `alt-]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChord {
    code: ChordCode,
    modifiers: KeyModifiers,
}

impl KeyChord {
    pub fn from_event(event: KeyEvent) -> Option<Self> {
        let mut modifiers = supported_modifiers(event.modifiers);
        let code = match event.code {
            KeyCode::Char(character) => {
                ChordCode::Char(normalize_modified_char(character, event.modifiers))
            }
            KeyCode::Enter => ChordCode::Enter,
            KeyCode::Esc => ChordCode::Esc,
            KeyCode::Tab => ChordCode::Tab,
            KeyCode::BackTab => {
                modifiers.insert(KeyModifiers::SHIFT);
                ChordCode::Tab
            }
            KeyCode::Backspace => ChordCode::Backspace,
            KeyCode::Up => ChordCode::Up,
            KeyCode::Down => ChordCode::Down,
            KeyCode::Left => ChordCode::Left,
            KeyCode::Right => ChordCode::Right,
            KeyCode::Home => ChordCode::Home,
            KeyCode::End => ChordCode::End,
            KeyCode::PageUp => ChordCode::PageUp,
            KeyCode::PageDown => ChordCode::PageDown,
            KeyCode::Delete => ChordCode::Delete,
            KeyCode::Insert => ChordCode::Insert,
            _ => return None,
        };
        Some(Self { code, modifiers })
    }
}

impl FromStr for KeyChord {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self> {
        let token = input
            .trim()
            .to_ascii_lowercase()
            .replace("page-up", "pageup")
            .replace("page-down", "pagedown");
        if token.is_empty() {
            bail!("key chord must not be empty");
        }

        let mut parts = token.split('-').collect::<Vec<_>>();
        let key = parts
            .pop()
            .ok_or_else(|| anyhow::anyhow!("missing key in chord {input:?}"))?;
        let mut modifiers = KeyModifiers::empty();
        for modifier in parts {
            match modifier {
                "ctrl" | "control" => modifiers.insert(KeyModifiers::CONTROL),
                "alt" | "meta" => modifiers.insert(KeyModifiers::ALT),
                "shift" => modifiers.insert(KeyModifiers::SHIFT),
                other => bail!("unknown modifier {other:?} in key chord {input:?}"),
            }
        }

        let code = match key {
            "enter" | "return" => ChordCode::Enter,
            "esc" | "escape" => ChordCode::Esc,
            "tab" => ChordCode::Tab,
            "backspace" => ChordCode::Backspace,
            "up" => ChordCode::Up,
            "down" => ChordCode::Down,
            "left" => ChordCode::Left,
            "right" => ChordCode::Right,
            "home" => ChordCode::Home,
            "end" => ChordCode::End,
            "pageup" | "page-up" => ChordCode::PageUp,
            "pagedown" | "page-down" => ChordCode::PageDown,
            "delete" | "del" => ChordCode::Delete,
            "insert" | "ins" => ChordCode::Insert,
            "space" => ChordCode::Char(' '),
            "minus" => ChordCode::Char('-'),
            value => {
                let mut characters = value.chars();
                let character = characters
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing key in chord {input:?}"))?;
                if characters.next().is_some() {
                    bail!("unknown key {value:?} in key chord {input:?}");
                }
                if character.is_control() {
                    bail!("control characters must use a named key in chord {input:?}");
                }
                ChordCode::Char(character)
            }
        };

        Ok(Self { code, modifiers })
    }
}

impl fmt::Display for KeyChord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            write!(formatter, "ctrl-")?;
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            write!(formatter, "alt-")?;
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            write!(formatter, "shift-")?;
        }
        match self.code {
            ChordCode::Char(' ') => write!(formatter, "space"),
            ChordCode::Char('-') => write!(formatter, "minus"),
            ChordCode::Char(character) => write!(formatter, "{character}"),
            ChordCode::Enter => write!(formatter, "enter"),
            ChordCode::Esc => write!(formatter, "esc"),
            ChordCode::Tab => write!(formatter, "tab"),
            ChordCode::Backspace => write!(formatter, "backspace"),
            ChordCode::Up => write!(formatter, "up"),
            ChordCode::Down => write!(formatter, "down"),
            ChordCode::Left => write!(formatter, "left"),
            ChordCode::Right => write!(formatter, "right"),
            ChordCode::Home => write!(formatter, "home"),
            ChordCode::End => write!(formatter, "end"),
            ChordCode::PageUp => write!(formatter, "pageup"),
            ChordCode::PageDown => write!(formatter, "pagedown"),
            ChordCode::Delete => write!(formatter, "delete"),
            ChordCode::Insert => write!(formatter, "insert"),
        }
    }
}

/// Startup-compiled dispatch table. Event handling performs no string allocation.
#[derive(Debug, Clone)]
pub struct KeyMap {
    actions: HashMap<KeyChord, KeyAction>,
}

impl KeyMap {
    pub fn from_config(config: &KeyConfig) -> Result<Self> {
        let mut actions = HashMap::new();
        for (action, configured) in [
            (KeyAction::ShowGlobal, config.global.as_slice()),
            (KeyAction::ShowPwd, config.pwd.as_slice()),
            (KeyAction::ShowGitRoot, config.git_root.as_slice()),
            (KeyAction::ToggleScope, config.toggle_scope.as_slice()),
            (KeyAction::TogglePwdMode, config.toggle_pwd_mode.as_slice()),
            (KeyAction::ToggleContext, config.context.as_slice()),
            (KeyAction::ContextExpand, config.context_expand.as_slice()),
            (KeyAction::ContextShrink, config.context_shrink.as_slice()),
            (KeyAction::ToggleMetadata, config.toggle_metadata.as_slice()),
            (KeyAction::SelectNext, config.select_next.as_slice()),
            (KeyAction::SelectPrevious, config.select_previous.as_slice()),
            (KeyAction::Accept, config.accept.as_slice()),
            (KeyAction::Quit, config.quit.as_slice()),
            (KeyAction::Backspace, config.backspace.as_slice()),
        ] {
            if configured.is_empty() {
                bail!(
                    "key action {} must have at least one binding",
                    action.name()
                );
            }
            for token in configured {
                let chord = token.parse::<KeyChord>()?;
                if let Some(previous) = actions.insert(chord, action)
                    && previous != action
                {
                    bail!(
                        "key chord {chord} is assigned to both {} and {}",
                        previous.name(),
                        action.name()
                    );
                }
            }
        }
        Ok(Self { actions })
    }

    pub fn action_for(&self, event: KeyEvent) -> Option<KeyAction> {
        KeyChord::from_event(event).and_then(|chord| self.actions.get(&chord).copied())
    }
}

fn normalize_modified_char(character: char, modifiers: KeyModifiers) -> char {
    if modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT) {
        character.to_ascii_lowercase()
    } else {
        character
    }
}

fn supported_modifiers(modifiers: KeyModifiers) -> KeyModifiers {
    modifiers & (KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT)
}
