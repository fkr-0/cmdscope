use terminal_history::{
    AppConfig, AppModel, HistoryEntry, HistoryStore, Msg, PwdMatchMode, SearchMode, SearchScope,
};

fn store() -> HistoryStore {
    HistoryStore::from_entries(vec![
        HistoryEntry::new("1", 100, 0, "root cmd", "/repo", "s1", "host"),
        HistoryEntry::new("2", 110, 0, "src cmd", "/repo/src", "s1", "host"),
        HistoryEntry::new("3", 120, 0, "other cmd", "/other", "s1", "host"),
        HistoryEntry::new("4", 130, 0, "nested cmd", "/repo/src/bin", "s1", "host"),
    ])
}

#[test]
fn pwd_scope_can_match_exact_current_directory_only() {
    let scope = SearchScope::pwd(Some("/repo"), PwdMatchMode::Exact);

    let commands: Vec<_> = store()
        .search_with_scope("", &scope, 10)
        .into_iter()
        .map(|entry| entry.command)
        .collect();

    assert_eq!(commands, vec!["root cmd"]);
}

#[test]
fn pwd_scope_can_include_current_directory_subdirectories() {
    let scope = SearchScope::pwd(Some("/repo"), PwdMatchMode::IncludeSubdirs);

    let commands: Vec<_> = store()
        .search_with_scope("", &scope, 10)
        .into_iter()
        .map(|entry| entry.command)
        .collect();

    assert_eq!(commands, vec!["nested cmd", "src cmd", "root cmd"]);
}

#[test]
fn git_root_scope_includes_repository_tree() {
    let scope = SearchScope::git_root(Some("/repo"));

    let commands: Vec<_> = store()
        .search_with_scope("", &scope, 10)
        .into_iter()
        .map(|entry| entry.command)
        .collect();

    assert_eq!(commands, vec!["nested cmd", "src cmd", "root cmd"]);
}

#[test]
fn app_toggles_global_pwd_git_root_and_pwd_match_mode() {
    let mut model = AppModel::new_with_environment(
        store(),
        Some("/repo".to_string()),
        Some("/repo".to_string()),
        PwdMatchMode::Exact,
    );

    model.update(Msg::ShowPwd);
    assert_eq!(model.search_mode(), SearchMode::SamePwd);
    assert_eq!(model.pwd_match_mode(), PwdMatchMode::Exact);
    assert_eq!(model.visible_commands(), vec!["root cmd"]);

    model.update(Msg::TogglePwdMatchMode);
    assert_eq!(model.pwd_match_mode(), PwdMatchMode::IncludeSubdirs);
    assert_eq!(
        model.visible_commands(),
        vec!["nested cmd", "src cmd", "root cmd"]
    );

    model.update(Msg::ShowGitRoot);
    assert_eq!(model.search_mode(), SearchMode::GitRoot);
    assert_eq!(
        model.visible_commands(),
        vec!["nested cmd", "src cmd", "root cmd"]
    );

    model.update(Msg::ShowGlobal);
    assert_eq!(model.search_mode(), SearchMode::All);
    assert_eq!(
        model.visible_commands(),
        vec!["nested cmd", "other cmd", "src cmd", "root cmd"]
    );
}

#[test]
fn context_review_can_expand_and_shrink_around_selection() {
    let mut model = AppModel::new_with_environment(
        store(),
        Some("/repo".into()),
        Some("/repo".into()),
        PwdMatchMode::IncludeSubdirs,
    );
    model.update(Msg::ShowGitRoot);
    model.update(Msg::SelectNext); // src cmd
    model.update(Msg::ToggleContext);
    assert_eq!(model.context_radius(), 1);
    assert_eq!(
        model.visible_commands(),
        vec!["root cmd", "src cmd", "nested cmd"]
    );

    model.update(Msg::ContextExpand);
    assert_eq!(model.context_radius(), 2);
    assert_eq!(
        model.visible_commands(),
        vec!["root cmd", "src cmd", "nested cmd"]
    );

    model.update(Msg::ContextShrink);
    assert_eq!(model.context_radius(), 1);
    assert_eq!(
        model.visible_commands(),
        vec!["root cmd", "src cmd", "nested cmd"]
    );
}

#[test]
fn config_loads_shortcuts_and_pwd_semantics_from_toml() {
    let config = AppConfig::from_toml(
        r#"
        [keys]
        global = "ctrl-g"
        pwd = "ctrl-p"
        git_root = "ctrl-r"
        context = "ctrl-o"
        context_expand = "alt-]"
        context_shrink = "alt-["
        toggle_pwd_mode = "ctrl-s"

        [pwd]
        mode = "subdirs"
        "#,
    )
    .unwrap();

    assert_eq!(config.keys.global, "ctrl-g");
    assert_eq!(config.keys.context, "ctrl-o");
    assert_eq!(config.pwd.mode, PwdMatchMode::IncludeSubdirs);
}
