use cmdscope::{HistoryEntry, HistoryStore, SearchEngine, SearchScope};

fn large_store(size: usize) -> HistoryStore {
    HistoryStore::from_entries(
        (0..size)
            .map(|index| {
                let command = if index % 10 == 0 {
                    format!("cargo test package-{index}")
                } else {
                    format!("printf ordinary-command-{index}")
                };
                HistoryEntry::new(
                    index.to_string(),
                    index as i64,
                    0,
                    command,
                    "/repo",
                    "session",
                    "host",
                )
            })
            .collect(),
    )
}

#[test]
fn extending_query_only_scans_previous_matches() {
    let mut engine = SearchEngine::new(large_store(20_000), 200);
    engine.set_scope(SearchScope::global());

    engine.set_query("g");
    let broad = engine.stats();
    engine.set_query("go");
    let narrow = engine.stats();

    assert_eq!(broad.scanned, 20_000);
    assert!(narrow.scanned <= broad.matched);
    assert!(narrow.scanned < broad.scanned);
}

#[test]
fn inserting_inside_query_only_scans_previous_matches() {
    let mut engine = SearchEngine::new(large_store(20_000), 200);
    engine.set_scope(SearchScope::global());

    engine.set_query("ct");
    let before_insert = engine.stats();
    engine.set_query("cat");
    let after_insert = engine.stats();

    assert_eq!(after_insert.scanned, before_insert.matched);
    assert!(after_insert.scanned < after_insert.scope_candidates);
}

#[test]
fn backspace_restores_cached_prefix_without_rescanning() {
    let mut engine = SearchEngine::new(large_store(20_000), 200);
    engine.set_query("c");
    let expected = engine.results().to_vec();
    engine.set_query("ca");
    engine.set_query("c");

    assert_eq!(engine.results(), expected);
    assert!(engine.stats().cache_hit);
    assert_eq!(engine.stats().scanned, 0);
}

#[test]
fn top_k_ranking_remains_score_then_recency_ordered() {
    let store = HistoryStore::from_entries(vec![
        HistoryEntry::new("1", 100, 0, "git status", "/repo", "s", "h"),
        HistoryEntry::new("2", 200, 0, "git status", "/repo", "s", "h"),
        HistoryEntry::new("3", 300, 0, "git stash", "/repo", "s", "h"),
    ]);
    let mut engine = SearchEngine::new(store, 2);
    engine.set_query("git status");

    let commands = engine
        .results()
        .iter()
        .map(|&index| engine.entry(index).id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(commands, vec!["2", "1"]);
}
