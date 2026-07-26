use cmdscope::{AppModel, HistoryEntry, HistoryStore, Msg, SearchMode};

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
fn accepting_selection_records_command_for_shell_insertion() {
    let mut model = model();
    model.update(Msg::Input('g'));
    model.update(Msg::Input('i'));
    model.update(Msg::Input('t'));
    model.update(Msg::Accept);

    assert!(model.should_quit());
    assert_eq!(model.accepted_command(), Some("git status"));
}
