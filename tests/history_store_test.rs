use cmdscope::{HistoryEntry, HistoryStore, SearchMode};

fn seed_store() -> HistoryStore {
    let entries = vec![
        HistoryEntry::new("1", 100, 0, "git status", "/repo", "s1", "host"),
        HistoryEntry::new("2", 110, 0, "cargo test", "/repo", "s1", "host"),
        HistoryEntry::new("3", 120, 0, "ls", "/tmp", "s1", "host"),
        HistoryEntry::new("4", 130, 1, "git commit", "/repo", "s2", "host"),
        HistoryEntry::new("5", 140, 0, "rg ratatui", "/repo", "s2", "host"),
    ];
    HistoryStore::from_entries(entries)
}

#[test]
fn unavailable_scopes_return_no_history() {
    let store = seed_store();

    assert!(store.search("", SearchMode::SamePwd, None, 10).is_empty());
    assert!(
        store
            .search_with_scope("", &cmdscope::SearchScope::git_root(None), 10)
            .is_empty()
    );
}

#[test]
fn maximum_context_radius_cannot_overflow() {
    let store = seed_store();
    let context = store
        .context_around("3", usize::MAX, SearchMode::All, None)
        .unwrap();

    assert_eq!(context.len(), store.len());
}

#[test]
fn searches_commands_newest_first_with_skim_fuzzy_matching() {
    let store = seed_store();

    let results = store.search("git", SearchMode::All, None, 10);

    let commands: Vec<_> = results.iter().map(|entry| entry.command.as_str()).collect();
    assert_eq!(commands, vec!["git commit", "git status"]);
}

#[test]
fn can_limit_results_to_same_pwd() {
    let store = seed_store();

    let results = store.search("", SearchMode::SamePwd, Some("/tmp"), 10);

    let commands: Vec<_> = results.iter().map(|entry| entry.command.as_str()).collect();
    assert_eq!(commands, vec!["ls"]);
}

#[test]
fn context_ignores_active_filter_and_returns_time_neighbors() {
    let store = seed_store();

    let context = store.context_around("3", 1, SearchMode::All, None).unwrap();

    let commands: Vec<_> = context.iter().map(|entry| entry.command.as_str()).collect();
    assert_eq!(commands, vec!["cargo test", "ls", "git commit"]);
}

#[test]
fn context_can_be_limited_to_same_pwd() {
    let store = seed_store();

    let context = store
        .context_around("4", 1, SearchMode::SamePwd, Some("/repo"))
        .unwrap();

    let commands: Vec<_> = context.iter().map(|entry| entry.command.as_str()).collect();
    assert_eq!(commands, vec!["cargo test", "git commit", "rg ratatui"]);
}

#[test]
fn context_crosses_session_boundaries_in_chronological_order() {
    let store = HistoryStore::from_entries(vec![
        HistoryEntry::new("late", 300, 0, "third", "/repo", "session-c", "h"),
        HistoryEntry::new("early", 100, 0, "first", "/repo", "session-a", "h"),
        HistoryEntry::new("middle", 200, 0, "second", "/repo", "session-b", "h"),
    ]);

    let context = store
        .context_around("middle", 1, SearchMode::All, None)
        .unwrap();

    assert_eq!(
        context
            .iter()
            .map(|entry| (
                entry.command.as_str(),
                entry.session.as_str(),
                entry.timestamp
            ))
            .collect::<Vec<_>>(),
        vec![
            ("first", "session-a", 100),
            ("second", "session-b", 200),
            ("third", "session-c", 300),
        ]
    );
}

#[test]
fn duplicate_ids_are_rejected_as_ambiguous_for_id_based_context() {
    let store = HistoryStore::from_entries(vec![
        HistoryEntry::new("dup", 100, 0, "first", "/repo", "s", "h"),
        HistoryEntry::new("dup", 200, 0, "second", "/repo", "s", "h"),
    ]);

    assert!(
        store
            .context_around("dup", 1, SearchMode::All, None)
            .is_none()
    );
}
