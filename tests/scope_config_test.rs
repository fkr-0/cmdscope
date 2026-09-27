use cmdscope::{
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
fn unix_backslashes_remain_literal_and_case_sensitive() {
    let store = HistoryStore::from_entries(vec![
        HistoryEntry::new("1", 100, 0, "literal", r"/tmp/Foo\Bar", "s", "h"),
        HistoryEntry::new("2", 110, 0, "different", r"/tmp/foo/bar", "s", "h"),
    ]);
    let scope = SearchScope::pwd(Some(r"/tmp/Foo\Bar"), PwdMatchMode::Exact);

    let commands = store
        .search_with_scope("", &scope, 10)
        .into_iter()
        .map(|entry| entry.command)
        .collect::<Vec<_>>();

    assert_eq!(commands, vec!["literal"]);
}

#[test]
fn filesystem_root_scope_includes_descendants() {
    let store = HistoryStore::from_entries(vec![
        HistoryEntry::new("1", 100, 0, "root", "/", "s", "h"),
        HistoryEntry::new("2", 110, 0, "nested", "/var/tmp", "s", "h"),
    ]);
    let scope = SearchScope::pwd(Some("/"), PwdMatchMode::IncludeSubdirs);

    let commands = store
        .search_with_scope("", &scope, 10)
        .into_iter()
        .map(|entry| entry.command)
        .collect::<Vec<_>>();

    assert_eq!(commands, vec!["nested", "root"]);
}

#[test]
fn windows_unicode_paths_match_case_insensitively() {
    let store = HistoryStore::from_entries(vec![
        HistoryEntry::new("1", 100, 0, "unicode", r"C:\Üser\Projekt", "s", "h"),
        HistoryEntry::new("2", 110, 0, "other", r"C:\Üser\Else", "s", "h"),
    ]);
    let scope = SearchScope::pwd(Some("c:/üser/projekt"), PwdMatchMode::Exact);

    let commands = store
        .search_with_scope("", &scope, 10)
        .into_iter()
        .map(|entry| entry.command)
        .collect::<Vec<_>>();

    assert_eq!(commands, vec!["unicode"]);
}

#[test]
fn windows_paths_match_across_separator_and_case_variants() {
    let store = HistoryStore::from_entries(vec![
        HistoryEntry::new("1", 100, 0, "root", r"C:\\Repo", "s", "h"),
        HistoryEntry::new("2", 110, 0, "nested", r"c:\\repo\\src", "s", "h"),
        HistoryEntry::new("3", 120, 0, "prefix", r"C:\\Repository", "s", "h"),
    ]);
    let scope = SearchScope::pwd(Some("C:/REPO/"), PwdMatchMode::IncludeSubdirs);

    let commands = store
        .search_with_scope("", &scope, 10)
        .into_iter()
        .map(|entry| entry.command)
        .collect::<Vec<_>>();

    assert_eq!(commands, vec!["nested", "root"]);
}

#[test]
fn empty_recorded_pwd_is_not_treated_as_filesystem_root() {
    let store = HistoryStore::from_entries(vec![
        HistoryEntry::new("1", 100, 0, "empty", "", "s", "h"),
        HistoryEntry::new("2", 110, 0, "root", "/", "s", "h"),
    ]);
    let scope = SearchScope::pwd(Some("/"), PwdMatchMode::Exact);

    let commands = store
        .search_with_scope("", &scope, 10)
        .into_iter()
        .map(|entry| entry.command)
        .collect::<Vec<_>>();

    assert_eq!(commands, vec!["root"]);
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
fn scope_change_stats_report_the_required_rescan() {
    let mut model = AppModel::new_with_environment(
        store(),
        Some("/repo".to_string()),
        Some("/repo".to_string()),
        PwdMatchMode::Exact,
    );
    for character in "cmd".chars() {
        model.update(Msg::Input(character));
    }

    model.update(Msg::ShowPwd);

    assert!(!model.search_stats().cache_hit);
    assert_eq!(model.search_stats().scanned, 1);
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
fn mount_point_scopes_respect_path_component_boundaries() {
    let store = HistoryStore::from_entries(vec![
        HistoryEntry::new("1", 100, 0, "mount-root", "/mnt/data", "s", "h"),
        HistoryEntry::new("2", 110, 0, "mount-child", "/mnt/data/project", "s", "h"),
        HistoryEntry::new("3", 120, 0, "prefix-only", "/mnt/database", "s", "h"),
    ]);
    let scope = SearchScope::pwd(Some("/mnt/data"), PwdMatchMode::IncludeSubdirs);

    let commands = store
        .search_with_scope("", &scope, 10)
        .into_iter()
        .map(|entry| entry.command)
        .collect::<Vec<_>>();

    assert_eq!(commands, vec!["mount-child", "mount-root"]);
}

#[test]
fn symlink_like_aliases_are_not_silently_equated_to_physical_paths() {
    let store = HistoryStore::from_entries(vec![
        HistoryEntry::new("1", 100, 0, "physical", "/srv/project", "s", "h"),
        HistoryEntry::new("2", 110, 0, "alias", "/home/me/project-link", "s", "h"),
    ]);

    let physical = SearchScope::pwd(Some("/srv/project"), PwdMatchMode::Exact);
    let alias = SearchScope::pwd(Some("/home/me/project-link"), PwdMatchMode::Exact);

    assert_eq!(
        store
            .search_with_scope("", &physical, 10)
            .into_iter()
            .map(|entry| entry.command)
            .collect::<Vec<_>>(),
        vec!["physical"]
    );
    assert_eq!(
        store
            .search_with_scope("", &alias, 10)
            .into_iter()
            .map(|entry| entry.command)
            .collect::<Vec<_>>(),
        vec!["alias"]
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
        select_next = ["up", "ctrl-j"]

        [pwd]
        mode = "subdirs"
        "#,
    )
    .unwrap();

    assert_eq!(config.keys.global, vec!["ctrl-g"]);
    assert_eq!(config.keys.context, vec!["ctrl-o"]);
    assert_eq!(config.keys.select_next, vec!["up", "ctrl-j"]);
    assert_eq!(config.pwd.mode, PwdMatchMode::IncludeSubdirs);
    config.compile_keymap().unwrap();
}
