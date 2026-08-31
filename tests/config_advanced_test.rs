use cmdscope::{Action, AppConfig, AppModel, ColumnId, HistoryEntry, HistoryStore, Msg};

#[test]
fn advanced_columns_and_nested_menus_parse_and_compile() {
    let config = AppConfig::from_toml(
        r#"
        [ui.columns]
        order = ["pwd", "date", "exit", "duration"]

        [ui.column.pwd]
        width = 20
        min_width = 8
        max_width = 30
        align = "right"
        truncation = "start"
        priority = 7

        [ui.menus.actions]
        [[ui.menus.actions.items]]
        label = "more"
        action = "menu:more"

        [ui.menus.more]
        confirm = "ctrl-y"
        [[ui.menus.more.items]]
        label = "timeline"
        action = "window:timeline"

        [ui.windows.timeline]
        kind = "timeline"
        [ui.windows.timeline.keymap]
        close = "ctrl-q"

        [ui.wraps.stderr]
        template = "{command} 2>&1"
        "#,
    )
    .unwrap();
    assert_eq!(
        config.ui.effective_columns().visible(),
        vec![ColumnId::Pwd, ColumnId::Date]
    );
    config.ui.validate_references().unwrap();
    assert_eq!(config.ui.compile_menus().unwrap().len(), 2);
}

#[test]
fn invalid_references_fail_startup_validation() {
    let config = AppConfig::from_toml(
        r#"
        [ui.menus.actions]
        [[ui.menus.actions.items]]
        label = "bad"
        action = "menu:missing"
        "#,
    )
    .unwrap();
    let error = config.ui.validate_references().unwrap_err().to_string();
    assert!(error.contains("menu \"actions\""), "{error}");
}

#[test]
fn configured_wrap_action_composes_without_execution() {
    let config = AppConfig::from_toml(
        r#"
        [ui.wraps.stderr]
        template = "{command} 2>&1"
        "#,
    )
    .unwrap();
    config.ui.validate_references().unwrap();
    let mut model = AppModel::new(
        HistoryStore::from_entries(vec![HistoryEntry::new(
            "1", 100, 0, "ls", "/repo", "s", "h",
        )]),
        Some("/repo".into()),
    );
    model.configure_wraps(config.ui.wraps.clone());
    model.update(Msg::OpenActions);
    if let Some(menu) = model.actions_menu_mut() {
        menu.items.push(cmdscope::menu::MenuItem {
            label: "wrap".into(),
            key: None,
            action: Action::Wrap("stderr".into()),
            on_select: Vec::new(),
            on_leave: Vec::new(),
        });
        menu.selected = menu.items.len() - 1;
    }
    model.execute_menu_action();
    assert_eq!(model.accepted_command(), Some("ls 2>&1"));
}
