use crate::{HistoryEntry, HistoryStore, QueryPlan, QueryStage, SearchScope};
use regex::Regex;
use skim::{fuzzy_matcher::FuzzyMatcher, prelude::SkimMatcherV2};
use std::{cmp::Reverse, collections::BinaryHeap};

const MAX_QUERY_LAYERS: usize = 32;

#[derive(Debug, Clone)]
struct QueryLayer {
    query: String,
    plan: QueryPlan,
    candidates: Vec<usize>,
    ranked: Vec<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HistoryEntry;

    #[test]
    fn query_layer_cache_is_bounded() {
        let store = HistoryStore::from_entries(vec![HistoryEntry::new(
            "1",
            1,
            0,
            "a".repeat(128),
            "/repo",
            "session",
            "host",
        )]);
        let mut engine = SearchEngine::new(store, 10);

        for length in 1..=128 {
            engine.set_query(&"a".repeat(length));
        }

        assert_eq!(engine.layers.len(), MAX_QUERY_LAYERS);
        assert_eq!(engine.query(), "a".repeat(128));
    }
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
/// - adding characters anywhere in a query scans only the previous query's matches;
/// - backspacing restores a cached query layer without rescanning;
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

    fn trim_query_cache(&mut self) {
        if self.layers.len() > MAX_QUERY_LAYERS {
            let remove = self.layers.len() - MAX_QUERY_LAYERS;
            self.layers.drain(1..=remove);
        }
    }

    pub fn set_scope(&mut self, scope: SearchScope) {
        if self.scope != scope {
            let query = self.query().to_string();
            self.scope = scope;
            self.reset_scope();
            if !query.is_empty() {
                self.try_set_query(&query)
                    .expect("retained query must remain valid after scope change");
            }
        }
    }

    pub fn scope(&self) -> &SearchScope {
        &self.scope
    }

    pub fn query(&self) -> &str {
        &self.layers.last().expect("base query layer").query
    }

    pub fn history_count(&self) -> usize {
        self.store.len()
    }

    pub fn entry(&self, index: usize) -> &HistoryEntry {
        self.store.entry(index)
    }

    pub fn set_query(&mut self, query: &str) {
        let _ = self.try_set_query(query);
    }

    pub fn try_set_query(&mut self, query: &str) -> anyhow::Result<()> {
        let current = self.layers.last().expect("base query layer");
        if current.query == query {
            self.stats = self.cache_hit_stats();
            return Ok(());
        }
        let plan = QueryPlan::parse(query)?;
        if let Some(position) = self.layers.iter().position(|layer| layer.query == query) {
            self.layers.truncate(position + 1);
            self.stats = self.cache_hit_stats();
            return Ok(());
        }

        let narrows_current = plan.narrows_from(&current.plan);
        let stage_offset = if narrows_current {
            current.plan.stages.len().saturating_sub(1)
        } else {
            self.layers.truncate(1);
            0
        };
        let source = &self.layers.last().expect("base query layer").candidates;
        let (layer, stats) = self.filter_layer(query, plan, stage_offset, source);
        self.layers.push(layer);
        self.trim_query_cache();
        self.stats = stats;
        Ok(())
    }

    pub fn results(&self) -> &[usize] {
        &self.layers.last().expect("base query layer").ranked
    }

    pub fn stats(&self) -> SearchStats {
        self.stats
    }

    pub fn context_around(&self, selected_id: &str, radius: usize) -> Option<Vec<usize>> {
        let selected_index = self.store.index_for_id(selected_id)?;
        self.context_around_index(selected_index, radius)
    }

    pub(crate) fn context_around_index(
        &self,
        selected_index: usize,
        radius: usize,
    ) -> Option<Vec<usize>> {
        let scoped = &self.layers.first().expect("base query layer").candidates;
        let position = scoped.binary_search(&selected_index).ok()?;
        let start = position.saturating_sub(radius);
        let end = position
            .saturating_add(radius)
            .saturating_add(1)
            .min(scoped.len());
        Some(scoped[start..end].to_vec())
    }

    pub fn context_around_index_with_scope(
        &self,
        selected_index: usize,
        radius: usize,
        scope: &SearchScope,
    ) -> Option<Vec<usize>> {
        let scoped = self.store.indices_for_scope(scope);
        let position = scoped.binary_search(&selected_index).ok()?;
        let start = position.saturating_sub(radius);
        let end = position
            .saturating_add(radius)
            .saturating_add(1)
            .min(scoped.len());
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
            plan: QueryPlan::parse("").expect("empty query is valid"),
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

    fn filter_layer(
        &self,
        query: &str,
        plan: QueryPlan,
        stage_offset: usize,
        source: &[usize],
    ) -> (QueryLayer, SearchStats) {
        if plan.is_empty() {
            let ranked = source
                .iter()
                .rev()
                .take(self.limit)
                .copied()
                .collect::<Vec<_>>();
            let returned = ranked.len();
            return (
                QueryLayer {
                    query: query.to_string(),
                    plan,
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

        if plan.stages.len() == 1 && matches!(plan.last(), QueryStage::Fuzzy(_)) {
            let query_value = match plan.last() {
                QueryStage::Fuzzy(value) => value.as_str(),
                _ => unreachable!(),
            };
            let mut candidates = Vec::with_capacity(source.len().min(4096));
            let mut best =
                BinaryHeap::<Reverse<RankKey>>::with_capacity(self.limit.min(source.len()));
            for &index in source {
                let entry = self.store.entry(index);
                let Some(score) = self.matcher.fuzzy_match(&entry.command, query_value) else {
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
            let matched = candidates.len();
            return (
                QueryLayer {
                    query: query.to_string(),
                    plan,
                    candidates,
                    ranked: ranked.clone(),
                },
                SearchStats {
                    scope_candidates: self.layers[0].candidates.len(),
                    scanned: source.len(),
                    matched,
                    returned: ranked.len(),
                    cache_hit: false,
                },
            );
        }

        let mut candidates = source.to_vec();
        let mut scanned = 0;
        for stage in plan.stages.iter().skip(stage_offset) {
            let regex = match stage {
                QueryStage::Regex(pattern) => {
                    Some(Regex::new(pattern).expect("query parser validates regex"))
                }
                _ => None,
            };
            let fuzzy_query = match stage {
                QueryStage::Fuzzy(value) => Some(value.as_str()),
                _ => None,
            };
            let literal_query = match stage {
                QueryStage::Literal(value) => Some(value.as_str()),
                _ => None,
            };
            let input = std::mem::take(&mut candidates);
            scanned = input.len();
            candidates = input
                .into_iter()
                .filter(|&index| {
                    let entry = self.store.entry(index);
                    if let Some(value) = fuzzy_query {
                        self.matcher.fuzzy_match(&entry.command, value).is_some()
                    } else if let Some(value) = literal_query {
                        entry.command.contains(value)
                    } else {
                        regex
                            .as_ref()
                            .is_some_and(|expression| expression.is_match(&entry.command))
                    }
                })
                .collect();
        }

        let mut best =
            BinaryHeap::<Reverse<RankKey>>::with_capacity(self.limit.min(candidates.len()));
        for &index in &candidates {
            let entry = self.store.entry(index);
            let score = self
                .matcher
                .fuzzy_match(
                    &entry.command,
                    match plan.last() {
                        QueryStage::Fuzzy(value) => value.as_str(),
                        _ => "",
                    },
                )
                .unwrap_or(1);
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
            scanned,
            matched: candidates.len(),
            returned: ranked.len(),
            cache_hit: false,
        };
        (
            QueryLayer {
                query: query.to_string(),
                plan,
                candidates,
                ranked,
            },
            stats,
        )
    }
}
