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
fn backtab_is_always_treated_as_shift_tab() {
    let config = AppConfig::from_toml(
        r#"
        [keys]
        toggle_scope = "shift-tab"
        "#,
    )
    .unwrap();
    let keymap = config.compile_keymap().unwrap();

    assert_eq!(
        keymap.action_for(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE)),
        Some(KeyAction::ToggleScope)
    );
    assert_eq!(
        keymap.action_for(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
        None
    );
}

#[test]
fn raw_control_character_bindings_are_rejected() {
    let config = AppConfig::from_toml("[keys]\ncontext = \"\\u001b\"\n").unwrap();

    let error = config.compile_keymap().unwrap_err().to_string();
    assert!(error.contains("control characters"), "{error}");
}

#[test]
fn unknown_config_fields_fail_instead_of_falling_back_to_defaults() {
    let error = AppConfig::from_toml("[keys]\nselect_nxt = \"down\"\n").unwrap_err();
    let error = format!("{error:#}");

    assert!(error.contains("unknown field"), "{error}");
    assert!(error.contains("select_nxt"), "{error}");
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
