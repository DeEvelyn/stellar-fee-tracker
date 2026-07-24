use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::types::RecommendResponse;

type CacheKey = (u32, String, String);

struct CacheEntry {
    value: RecommendResponse,
    expires_at: Instant,
}

pub struct RecommendationCache {
    entries: HashMap<CacheKey, CacheEntry>,
    ttl: Duration,
}

impl RecommendationCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            entries: HashMap::new(),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    pub fn get(&self, key: &CacheKey) -> Option<&RecommendResponse> {
        self.entries.get(key).and_then(|e| {
            if e.expires_at > Instant::now() {
                Some(&e.value)
            } else {
                None
            }
        })
    }

    pub fn set(&mut self, key: CacheKey, value: RecommendResponse) {
        self.entries.insert(
            key,
            CacheEntry {
                value,
                expires_at: Instant::now() + self.ttl,
            },
        );
    }

    pub fn invalidate(&mut self, key: &CacheKey) {
        self.entries.remove(key);
    }

    pub fn invalidate_all(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recommendation::types::RecommendResponse;

    fn make_response() -> RecommendResponse {
        RecommendResponse {
            recommended_fee: "100".to_string(),
            fee_in_stroops: 100,
            estimated_wait_ledgers: 1,
            confidence: 0.95,
            network_condition: "normal".to_string(),
            alternatives: vec![],
        }
    }

    fn make_key() -> CacheKey {
        (2, "standard".to_string(), "normal".to_string())
    }

    #[test]
    fn miss_on_empty_cache() {
        let cache = RecommendationCache::new(10);
        assert!(cache.get(&make_key()).is_none());
    }

    #[test]
    fn hit_after_set() {
        let mut cache = RecommendationCache::new(10);
        cache.set(make_key(), make_response());
        assert!(cache.get(&make_key()).is_some());
    }

    #[test]
    fn miss_after_invalidate() {
        let mut cache = RecommendationCache::new(10);
        cache.set(make_key(), make_response());
        cache.invalidate(&make_key());
        assert!(cache.get(&make_key()).is_none());
    }

    #[test]
    fn hit_after_invalidate_other_key() {
        let mut cache = RecommendationCache::new(10);
        cache.set(make_key(), make_response());
        cache.invalidate(&(99, "other".to_string(), "unknown".to_string()));
        assert!(cache.get(&make_key()).is_some());
    }

    #[test]
    fn expired_entry_returns_none() {
        let mut cache = RecommendationCache::new(0);
        cache.set(make_key(), make_response());
        assert!(cache.get(&make_key()).is_none());
    }

    #[test]
    fn invalidate_all_clears_entries() {
        let mut cache = RecommendationCache::new(10);
        cache.set(make_key(), make_response());
        cache.set((1, "fast".to_string(), "rising".to_string()), make_response());
        cache.invalidate_all();
        assert!(cache.get(&make_key()).is_none());
    }
}
