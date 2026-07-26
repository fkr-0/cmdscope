use cmdscope::{HistoryEntry, HistoryStore, SearchEngine};
use skim::{fuzzy_matcher::FuzzyMatcher, prelude::SkimMatcherV2};
use std::{env, hint::black_box, time::Instant};

const QUERIES: &[&str] = &[
    "c",
    "ca",
    "car",
    "carg",
    "cargo",
    "cargo ",
    "cargo t",
    "cargo te",
    "cargo tes",
    "cargo test",
];

fn main() {
    let size = env::args()
        .nth(1)
        .as_deref()
        .unwrap_or("100000")
        .parse::<usize>()
        .expect("first argument must be a history size");
    let store = synthetic_store(size);

    println!("history_rows={size} result_limit=200");
    let legacy_total = benchmark_full_rescan(&store);
    let incremental_total = benchmark_incremental(store);
    let speedup = legacy_total.as_secs_f64() / incremental_total.as_secs_f64();

    println!("legacy_total_us={}", legacy_total.as_micros());
    println!("incremental_total_us={}", incremental_total.as_micros());
    println!("sequence_speedup={speedup:.2}x");
}

fn synthetic_store(size: usize) -> HistoryStore {
    HistoryStore::from_entries(
        (0..size)
            .map(|index| {
                let command = if index % 10 == 0 {
                    format!("cargo test --package package-{index}")
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

fn benchmark_full_rescan(store: &HistoryStore) -> std::time::Duration {
    let total = Instant::now();
    for query in QUERIES {
        let started = Instant::now();
        let matcher = SkimMatcherV2::default().smart_case().use_cache(true);
        let mut scored = store
            .entries()
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                matcher
                    .fuzzy_match(&entry.command, query)
                    .map(|score| (score, entry.timestamp, index))
            })
            .collect::<Vec<_>>();
        scored.sort_unstable_by(|left, right| {
            right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1))
        });
        black_box(scored.iter().take(200).count());
        println!(
            "legacy query={query:?} elapsed_us={} scanned={} matched={}",
            started.elapsed().as_micros(),
            store.len(),
            scored.len()
        );
    }
    total.elapsed()
}

fn benchmark_incremental(store: HistoryStore) -> std::time::Duration {
    let mut engine = SearchEngine::new(store, 200);
    let total = Instant::now();
    for query in QUERIES {
        let started = Instant::now();
        engine.set_query(query);
        black_box(engine.results().len());
        let stats = engine.stats();
        println!(
            "incremental query={query:?} elapsed_us={} scanned={} matched={} cache_hit={}",
            started.elapsed().as_micros(),
            stats.scanned,
            stats.matched,
            stats.cache_hit
        );
    }
    total.elapsed()
}
