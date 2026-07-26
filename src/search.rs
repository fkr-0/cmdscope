use crate::{HistoryEntry, HistoryStore, SearchScope};
use skim::{fuzzy_matcher::FuzzyMatcher, prelude::SkimMatcherV2};
use std::{cmp::Reverse, collections::BinaryHeap};

#[derive(Debug, Clone)]
struct QueryLayer {
    query: String,
    candidates: Vec<usize>,
    ranked: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct RankKey {
    score: i64,
    timestamp: i64,
    index: usize,
}

/// Instrumentation from the most recent query update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SearchStats {
    pub scope_candidates: usize,
    pub scanned: usize,
    pub matched: usize,
    pub returned: usize,
    pub cache_hit: bool,
}

/// Persistent fzf-style filtering session.
///
/// It combines Skim's dynamic-programming fzf-compatible scorer with the
/// responsiveness techniques that matter in an interactive picker:
///
/// - a scope index is built only when scope changes;
/// - extending a query scans only the previous query's matches;
///
/// - backspacing restores a cached prefix layer without rescanning;
/// - a bounded heap ranks only the best visible results instead of sorting all
///   matches;
/// - results are history indices, so no command strings are cloned per key.
pub struct SearchEngine {
    store: HistoryStore,
    scope: SearchScope,
    layers: Vec<QueryLayer>,
    matcher: SkimMatcherV2,
    limit: usize,
    stats: SearchStats,
}

impl SearchEngine {
    pub fn new(store: HistoryStore, limit: usize) -> Self {
        let mut engine = Self {
            store,
            scope: SearchScope::global(),
            layers: Vec::new(),
            matcher: SkimMatcherV2::default().smart_case().use_cache(true),
            limit,
            stats: SearchStats::default(),
        };
        engine.reset_scope();
        engine
    }

    pub fn set_scope(&mut self, scope: SearchScope) {
        if self.scope != scope {
            self.scope = scope;
            self.reset_scope();
        }
    }

    pub fn scope(&self) -> &SearchScope {
        &self.scope
    }

    pub fn query(&self) -> &str {
        &self.layers.last().expect("base query layer").query
    }

    pub fn set_query(&mut self, query: &str) {
        let current = self.query();
        if current == query {
            self.stats = self.cache_hit_stats();
            return;
        }

        if let Some(position) = self.layers.iter().position(|layer| layer.query == query) {
            self.layers.truncate(position + 1);
            self.stats = self.cache_hit_stats();
            return;
        }

        let extends_current = query.starts_with(current);
        if !extends_current {
            self.layers.truncate(1);
        }
        let source = &self.layers.last().expect("base query layer").candidates;
        let (layer, stats) = self.filter_layer(query, source);
        self.layers.push(layer);
        self.stats = stats;
    }

    pub fn results(&self) -> &[usize] {
        &self.layers.last().expect("base query layer").ranked
    }

    pub fn stats(&self) -> SearchStats {
        self.stats
    }

    pub fn entry(&self, index: usize) -> &HistoryEntry {
        self.store.entry(index)
    }

    pub fn context_around(&self, selected_id: &str, radius: usize) -> Option<Vec<usize>> {
        let selected_index = self.store.index_for_id(selected_id)?;
        let scoped = &self.layers.first().expect("base query layer").candidates;
        let position = scoped.binary_search(&selected_index).ok()?;
        let start = position.saturating_sub(radius);
        let end = (position + radius + 1).min(scoped.len());
        Some(scoped[start..end].to_vec())
    }

    fn reset_scope(&mut self) {
        let candidates = self.store.indices_for_scope(&self.scope);
        let ranked = candidates
            .iter()
            .rev()
            .take(self.limit)
            .copied()
            .collect::<Vec<_>>();
        self.stats = SearchStats {
            scope_candidates: candidates.len(),
            scanned: 0,
            matched: candidates.len(),
            returned: ranked.len(),
            cache_hit: false,
        };
        self.layers.clear();
        self.layers.push(QueryLayer {
            query: String::new(),
            candidates,
            ranked,
        });
    }

    fn cache_hit_stats(&self) -> SearchStats {
        SearchStats {
            scope_candidates: self.layers[0].candidates.len(),
            scanned: 0,
            matched: self.layers.last().expect("query layer").candidates.len(),
            returned: self.results().len(),
            cache_hit: true,
        }
    }

    fn filter_layer(&self, query: &str, source: &[usize]) -> (QueryLayer, SearchStats) {
        if query.is_empty() {
            let ranked = source
                .iter()
                .rev()
                .take(self.limit)
                .copied()
                .collect::<Vec<_>>();
            let returned = ranked.len();
            return (
                QueryLayer {
                    query: String::new(),
                    candidates: source.to_vec(),
                    ranked,
                },
                SearchStats {
                    scope_candidates: self.layers[0].candidates.len(),
                    scanned: 0,
                    matched: source.len(),
                    returned,
                    cache_hit: true,
                },
            );
        }

        let mut candidates = Vec::with_capacity(source.len().min(4096));
        let mut best = BinaryHeap::<Reverse<RankKey>>::with_capacity(self.limit + 1);
        for &index in source {
            let entry = self.store.entry(index);
            let Some(score) = self.matcher.fuzzy_match(&entry.command, query) else {
                continue;
            };
            candidates.push(index);
            if self.limit == 0 {
                continue;
            }
            let key = RankKey {
                score,
                timestamp: entry.timestamp,
                index,
            };
            if best.len() < self.limit {
                best.push(Reverse(key));
            } else if best.peek().is_some_and(|worst| key > worst.0) {
                best.pop();
                best.push(Reverse(key));
            }
        }

        let mut ranked = best.into_iter().map(|item| item.0).collect::<Vec<_>>();
        ranked.sort_unstable_by(|left, right| right.cmp(left));
        let ranked = ranked
            .into_iter()
            .map(|item| item.index)
            .collect::<Vec<_>>();
        let stats = SearchStats {
            scope_candidates: self.layers[0].candidates.len(),
            scanned: source.len(),
            matched: candidates.len(),
            returned: ranked.len(),
            cache_hit: false,
        };
        (
            QueryLayer {
                query: query.to_string(),
                candidates,
                ranked,
            },
            stats,
        )
    }
}
