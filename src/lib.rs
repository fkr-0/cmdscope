//! Fast, testable core for the `cmdscope` Atuin history picker.
//!
//! The crate is split into narrow layers:
//!
//! - [`history`] loads and indexes immutable history data;
//! - [`search`] implements incremental fzf-style candidate narrowing;
//! - [`app`] owns pure application state transitions;
//! - [`config`] and [`keymap`] compile TOML into allocation-free key dispatch;
//! - [`tui`] renders model state without owning behavior.

pub mod app;
pub mod config;
pub mod history;
pub mod keymap;
pub mod scope;
pub mod search;
pub mod tui;

pub use app::{AppModel, Msg};
pub use config::{AppConfig, HistoryColumn, KeyConfig, PwdConfig, UiConfig};
pub use history::{HistoryEntry, HistoryStore};
pub use keymap::{KeyAction, KeyChord, KeyMap};
pub use scope::{PwdMatchMode, SearchMode, SearchScope};
pub use search::{SearchEngine, SearchStats};
