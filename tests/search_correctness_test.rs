use cmdscope::{HistoryEntry, HistoryStore, SearchEngine, SearchScope};
use skim::{fuzzy_matcher::FuzzyMatcher, prelude::SkimMatcherV2};

fn store() -> HistoryStore {
    HistoryStore::from_entries(
        (0..500)
            .map(|index| {
                let command = match index % 7 {
                    0 => format!("cargo test package-{index}"),
                    1 => format!("Cargo Test Package-{index}"),
                    2 => format!("git commit -m message-{index}"),
                    3 => format!("printf ordinary-{index}"),
                    4 => format!("rg unicode-Ω-{index}"),
                    5 => format!("docker compose up service-{index}"),
                    _ => format!("systemctl --user status unit-{index}"),
                };
                HistoryEntry::new(
                    index.to_string(),
                    (index / 2) as i64,
                    0,
                    command,
                    if index % 3 == 0 { "/repo" } else { "/tmp" },
                    "session",
                    "host",
                )
            })
            .collect(),
    )
}

#[test]
fn maximum_result_limit_does_not_overflow_heap_capacity() {
    let store = store();
    let mut engine = SearchEngine::new(store.clone(), usize::MAX);

    engine.set_query("cargo");

    assert_eq!(
        engine.results(),
        reference(&store, "cargo", usize::MAX).as_slice()
    );
}

fn reference(store: &HistoryStore, query: &str, limit: usize) -> Vec<usize> {
    if query.is_empty() {
        return (0..store.len()).rev().take(limit).collect();
    }

    let matcher = SkimMatcherV2::default().smart_case().use_cache(true);
    let mut ranked = store
        .entries()
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            matcher
                .fuzzy_match(&entry.command, query)
                .map(|score| (score, entry.timestamp, index))
        })
        .collect::<Vec<_>>();
    ranked.sort_unstable_by(|left, right| right.cmp(left));
    ranked
        .into_iter()
        .take(limit)
        .map(|(_, _, index)| index)
        .collect()
}

#[test]
fn incremental_edits_match_a_fresh_full_rescan() {
    let store = store();
    let mut engine = SearchEngine::new(store.clone(), 37);

    for query in [
        "",
        "c",
        "ca",
        "cargo",
        "car",
        "Cargo",
        "Cargo T",
        "x",
        "",
        "printf",
        "pri",
        "unicode Ω",
        "unicode-Ω-4",
        "system status",
    ] {
        engine.set_query(query);
        assert_eq!(
            engine.results(),
            reference(&store, query, 37),
            "query={query:?}"
        );
    }
}

#[test]
fn empty_store_and_zero_result_limit_are_safe() {
    let mut empty = SearchEngine::new(HistoryStore::from_entries(Vec::new()), 200);
    empty.set_query("anything");
    assert!(empty.results().is_empty());
    assert_eq!(empty.stats().scanned, 0);

    let mut zero_limit = SearchEngine::new(store(), 0);
    zero_limit.set_scope(SearchScope::global());
    zero_limit.set_query("cargo");
    assert!(zero_limit.results().is_empty());
    assert!(zero_limit.stats().matched > 0);
}

#[test]
fn changing_scope_preserves_the_active_query() {
    let store = store();
    let mut engine = SearchEngine::new(store, 37);
    engine.set_query("cargo");

    engine.set_scope(cmdscope::SearchScope::pwd(
        Some("/repo"),
        cmdscope::PwdMatchMode::Exact,
    ));

    assert_eq!(engine.query(), "cargo");
    assert!(!engine.results().is_empty());
    assert!(
        engine
            .results()
            .iter()
            .all(|&index| engine.entry(index).cwd == "/repo")
    );
}
