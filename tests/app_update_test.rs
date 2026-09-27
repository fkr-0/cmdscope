use cmdscope::{
    Action, AppConfig, AppModel, ColumnId, HistoryEntry, HistoryStore, Msg, SearchMode,
    SortDirection, SortField,
};

fn model() -> AppModel {
    AppModel::new(
        HistoryStore::from_entries(vec![
            HistoryEntry::new("1", 100, 0, "git status", "/repo", "s1", "host"),
            HistoryEntry::new("2", 110, 0, "cargo test", "/repo", "s1", "host"),
            HistoryEntry::new("3", 120, 0, "ls", "/tmp", "s1", "host"),
        ]),
        Some("/repo".to_string()),
    )
}

#[test]
fn actions_menu_composes_selected_command_without_executing_shell() {
    let mut model = model();
    model.update(Msg::OpenActions);
    assert!(model.modal_open());
    model.execute_menu_action();
    assert!(model.should_quit());
    assert_eq!(model.accepted_command(), Some("ls"));
}

#[test]
fn append_action_stays_in_picker_and_updates_query() {
    let mut model = model();
    model.update(Msg::OpenActions);
    model.handle_modal_next();
    model.execute_menu_action();
    assert!(!model.should_quit());
    assert_eq!(model.query(), "ls");
}

#[test]
fn opening_configured_menu_runs_initial_item_select_lifecycle() {
    let config = AppConfig::from_toml(
        r#"
        [ui.menus.actions]
        [[ui.menus.actions.items]]
        label = "first"
        action = "append"
        on_select = ["window:location"]

        [ui.windows.location]
        kind = "location"
        "#,
    )
    .unwrap();
    config.ui.validate_references().unwrap();
    let mut model = model();
    model.configure_interactions(config.ui.compile_menus().unwrap());
    model.configure_windows(config.ui.windows.clone());

    model.update(Msg::OpenActions);

    assert_eq!(model.active_window(), Some("location"));
}

#[test]
fn closing_configured_menu_runs_selected_item_leave_lifecycle() {
    let config = AppConfig::from_toml(
        r#"
        [ui.menus.actions]
        [[ui.menus.actions.items]]
        label = "first"
        action = "append"
        on_leave = ["window:location"]

        [ui.windows.location]
        kind = "location"
        "#,
    )
    .unwrap();
    config.ui.validate_references().unwrap();
    let mut model = model();
    model.configure_interactions(config.ui.compile_menus().unwrap());
    model.configure_windows(config.ui.windows.clone());
    model.update(Msg::OpenActions);

    model.close_modal();

    assert_eq!(model.active_window(), Some("location"));
}

#[test]
fn direct_item_activation_runs_selected_item_leave_lifecycle() {
    let config = AppConfig::from_toml(
        r#"
        [ui.menus.actions]
        [[ui.menus.actions.items]]
        label = "append"
        key = "a"
        action = "append"
        on_leave = ["window:location"]

        [ui.windows.location]
        kind = "location"
        "#,
    )
    .unwrap();
    config.ui.validate_references().unwrap();
    let mut model = model();
    model.configure_interactions(config.ui.compile_menus().unwrap());
    model.configure_windows(config.ui.windows.clone());
    model.update(Msg::OpenActions);

    model.execute_menu_item(0);

    assert_eq!(model.active_window(), Some("location"));
    assert_eq!(model.query(), "ls");
}

#[test]
fn configured_menu_navigation_runs_item_leave_then_select_lifecycle() {
    let config = AppConfig::from_toml(
        r#"
        [ui.menus.actions]
        next = "down"
        previous = "up"
        [[ui.menus.actions.items]]
        label = "first"
        action = "append"
        on_leave = ["window:location"]
        [[ui.menus.actions.items]]
        label = "second"
        action = "append"
        on_select = ["window:timeline"]

        [ui.windows.location]
        kind = "location"
        [ui.windows.timeline]
        kind = "timeline"
        "#,
    )
    .unwrap();
    config.ui.validate_references().unwrap();
    let mut model = model();
    model.configure_interactions(config.ui.compile_menus().unwrap());
    model.configure_windows(config.ui.windows.clone());
    model.update(Msg::OpenActions);

    model.handle_modal_next();

    assert_eq!(model.active_window(), Some("timeline"));
}

#[test]
fn direct_menu_item_activation_targets_item() {
    let config = AppConfig::from_toml(
        r#"
        [ui.menus.actions]
        [[ui.menus.actions.items]]
        label = "append"
        action = "append"
        [[ui.menus.actions.items]]
        label = "insert"
        key = "i"
        action = "insert+exit"
        "#,
    )
    .unwrap();
    config.ui.validate_references().unwrap();
    let mut model = model();
    model.configure_interactions(config.ui.compile_menus().unwrap());
    model.update(Msg::OpenActions);

    model.execute_menu_item(1);

    assert!(model.should_quit());
    assert_eq!(model.accepted_command(), Some("ls"));
}

#[test]
fn compose_actions_preserve_shell_text_without_execution() {
    let entry = model().selected().unwrap().clone();
    assert_eq!(
        Action::AndAppend.compose("git status", &entry),
        "git status && ls"
    );
    assert_eq!(
        Action::OrAppend.compose("git status", &entry),
        "git status || ls"
    );
    assert_eq!(
        Action::AppendExit.compose("git status", &entry),
        "git status ls"
    );
}

#[test]
fn query_editor_inserts_and_deletes_unicode_at_the_cursor() {
    let mut model = model();
    model.update(Msg::Input('a'));
    model.update(Msg::Input('c'));
    model.update(Msg::CursorLeft);
    model.update(Msg::Input('ö'));

    assert_eq!(model.query(), "aöc");
    assert_eq!(model.query_before_cursor(), "aö");

    model.update(Msg::Backspace);
    model.update(Msg::Delete);

    assert_eq!(model.query(), "a");
    assert_eq!(model.query_cursor(), 1);
}

#[test]
fn query_editor_supports_word_deletion_clear_and_normalized_paste() {
    let mut model = model();
    model.update(Msg::Paste("git\tstatus\n--short\u{7}".to_string()));

    assert_eq!(model.query(), "git status --short");

    model.update(Msg::DeleteWord);
    assert_eq!(model.query(), "git status ");
    model.update(Msg::ClearQuery);
    assert_eq!(model.query(), "");
    assert_eq!(model.query_cursor(), 0);
}

#[test]
fn cursor_navigation_leaves_context_without_losing_the_anchor() {
    let mut model = model();
    model.update(Msg::Input('l'));
    model.update(Msg::ToggleContext);
    let selected = model.selected().map(|entry| entry.id.clone());

    model.update(Msg::CursorStart);

    assert!(!model.in_context_mode());
    assert_eq!(model.query_cursor(), 0);
    assert_eq!(model.selected().map(|entry| entry.id.clone()), selected);
}

#[test]
fn duplicate_ids_do_not_move_context_away_from_the_selected_entry() {
    let mut model = AppModel::new(
        HistoryStore::from_entries(vec![
            HistoryEntry::new("dup", 100, 0, "old match", "/repo", "s", "h"),
            HistoryEntry::new("middle", 200, 0, "neighbor", "/repo", "s", "h"),
            HistoryEntry::new("dup", 300, 0, "new other", "/repo", "s", "h"),
        ]),
        Some("/repo".to_string()),
    );
    for character in "old match".chars() {
        model.update(Msg::Input(character));
    }

    model.update(Msg::ToggleContext);

    assert_eq!(
        model.selected().map(|entry| entry.command.as_str()),
        Some("old match")
    );
    assert_eq!(model.visible_commands(), vec!["old match", "neighbor"]);
}

#[test]
fn leaving_context_restores_the_anchor_selection() {
    let mut model = AppModel::new(
        HistoryStore::from_entries(vec![
            HistoryEntry::new("1", 100, 0, "alpha match", "/repo", "s", "h"),
            HistoryEntry::new("2", 110, 0, "neighbor", "/repo", "s", "h"),
            HistoryEntry::new("3", 120, 0, "beta match", "/repo", "s", "h"),
        ]),
        Some("/repo".to_string()),
    );
    for character in "match".chars() {
        model.update(Msg::Input(character));
    }
    model.update(Msg::SelectNext);
    assert_eq!(model.selected().map(|entry| entry.id.as_str()), Some("1"));

    model.update(Msg::ToggleContext);
    assert_eq!(model.selected().map(|entry| entry.id.as_str()), Some("1"));
    model.update(Msg::ToggleContext);

    assert_eq!(model.selected().map(|entry| entry.id.as_str()), Some("1"));
}

#[test]
fn unavailable_pwd_and_git_scopes_do_not_leak_global_results() {
    let mut model = AppModel::new_with_environment(
        HistoryStore::from_entries(vec![HistoryEntry::new(
            "1",
            100,
            0,
            "secret command",
            "/repo",
            "s",
            "h",
        )]),
        None,
        None,
        cmdscope::PwdMatchMode::Exact,
    );

    model.update(Msg::ShowPwd);
    assert_eq!(model.visible_len(), 0);
    model.update(Msg::ShowGitRoot);
    assert_eq!(model.visible_len(), 0);
    model.update(Msg::ShowGlobal);
    assert_eq!(model.visible_commands(), vec!["secret command"]);
}

#[test]
fn typing_leaves_context_and_resumes_incremental_search() {
    let mut model = model();
    model.update(Msg::Input('l'));
    model.update(Msg::ToggleContext);

    assert!(model.in_context_mode());
    model.update(Msg::Input('s'));

    assert!(!model.in_context_mode());
    assert_eq!(model.query(), "ls");
    assert_eq!(model.visible_commands(), vec!["ls"]);
}

#[test]
fn typing_updates_query_and_results() {
    let mut model = model();

    model.update(Msg::Input('g'));
    model.update(Msg::Input('i'));
    model.update(Msg::Input('t'));

    assert_eq!(model.query(), "git");
    assert_eq!(model.visible_commands(), vec!["git status"]);
}

#[test]
fn model_retains_total_history_count_while_filtering() {
    let mut model = model();

    for character in "git".chars() {
        model.update(Msg::Input(character));
    }

    assert_eq!(model.history_count(), 3);
    assert_eq!(model.visible_len(), 1);
}

#[test]
fn toggling_pwd_filter_changes_search_scope() {
    let mut model = model();

    model.update(Msg::TogglePwdFilter);

    assert_eq!(model.search_mode(), SearchMode::SamePwd);
    assert_eq!(model.visible_commands(), vec!["cargo test", "git status"]);
}

#[test]
fn context_mode_clears_query_dependence_but_keeps_selected_result() {
    let mut model = model();
    model.update(Msg::Input('l'));
    model.update(Msg::SelectNext);
    model.update(Msg::ToggleContext);

    assert!(model.in_context_mode());
    assert_eq!(model.visible_commands(), vec!["cargo test", "ls"]);
}

#[test]
fn column_customization_reorders_toggles_and_tracks_persistent_revision() {
    let mut model = model();
    assert_eq!(model.focused_column(), ColumnId::Date);
    assert_eq!(model.ui_state_revision(), 0);

    model.update(Msg::ColumnNext);
    assert_eq!(model.focused_column(), ColumnId::Pwd);
    assert_eq!(model.ui_state_revision(), 0);

    model.update(Msg::ColumnMoveLeft);
    assert_eq!(
        model.columns().normalized_order(),
        vec![
            ColumnId::Pwd,
            ColumnId::Date,
            ColumnId::Exit,
            ColumnId::Duration,
        ]
    );
    assert_eq!(model.ui_state_revision(), 1);

    model.update(Msg::ColumnToggle);
    assert!(!model.columns().pwd);
    assert_eq!(model.ui_state_revision(), 2);
}

#[test]
fn sorting_preserves_selected_identity_while_direction_changes() {
    let mut model = model();
    let selected = model.selected_id().unwrap().to_string();

    model.update(Msg::SortByColumn);
    assert_eq!(model.sort_field(), SortField::Date);
    assert_eq!(model.sort_direction(), SortDirection::Descending);
    assert_eq!(
        model.visible_commands(),
        vec!["ls", "cargo test", "git status"]
    );
    assert_eq!(model.selected_id(), Some(selected.as_str()));

    model.update(Msg::SortDirection);
    assert_eq!(model.sort_direction(), SortDirection::Ascending);
    assert_eq!(
        model.visible_commands(),
        vec!["git status", "cargo test", "ls"]
    );
    assert_eq!(model.selected_id(), Some(selected.as_str()));
}

#[test]
fn accepting_selection_records_command_for_shell_insertion() {
    let mut model = model();
    model.update(Msg::Input('g'));
    model.update(Msg::Input('i'));
    model.update(Msg::Input('t'));
    model.update(Msg::Accept);

    assert!(model.should_quit());
    assert_eq!(model.accepted_command(), Some("git status"));
}
