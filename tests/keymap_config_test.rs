use cmdscope::{AppConfig, KeyAction};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn toml_can_remap_navigation_and_acceptance() {
    let config = AppConfig::from_toml(
        r#"
        [keys]
        select_next = ["ctrl-j", "down"]
        select_previous = "ctrl-k"
        accept = "ctrl-y"
        quit = ["esc", "ctrl-q"]
        "#,
    )
    .unwrap();
    let keymap = config.compile_keymap().unwrap();

    assert_eq!(
        keymap.action_for(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL)),
        Some(KeyAction::SelectNext)
    );
    assert_eq!(
        keymap.action_for(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL)),
        Some(KeyAction::Accept)
    );
    assert_eq!(
        keymap.action_for(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL)),
        Some(KeyAction::Quit)
    );
}

#[test]
fn hyphenated_page_key_aliases_are_reachable() {
    let config = AppConfig::from_toml(
        r#"
        [keys]
        context_expand = "page-up"
        context_shrink = "ctrl-page-down"
        "#,
    )
    .unwrap();
    let keymap = config.compile_keymap().unwrap();

    assert_eq!(
        keymap.action_for(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)),
        Some(KeyAction::ContextExpand)
    );
    assert_eq!(
        keymap.action_for(KeyEvent::new(KeyCode::PageDown, KeyModifiers::CONTROL)),
        Some(KeyAction::ContextShrink)
    );
}

#[test]
fn duplicate_bindings_fail_fast() {
    let config = AppConfig::from_toml(
        r#"
        [keys]
        global = "ctrl-x"
        pwd = "ctrl-x"
        "#,
    )
    .unwrap();

    let error = config.compile_keymap().unwrap_err().to_string();
    assert!(error.contains("assigned to both"), "{error}");
}

#[test]
fn invalid_bindings_fail_fast() {
    let config = AppConfig::from_toml(
        r#"
        [keys]
        context = "hyper-super-cabbage"
        "#,
    )
    .unwrap();

    let error = config.compile_keymap().unwrap_err().to_string();
    assert!(error.contains("unknown modifier"), "{error}");
}
