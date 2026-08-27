use super::fee_stats::HorizonFeeStats;
use super::parser::parse_fee_stats;
use crate::error::DevkitError;
use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Network presets for Horizon endpoints.
#[derive(Debug, Clone)]
pub enum Network {
    Testnet,
    Mainnet,
    Custom(String),
}

impl Network {
    pub fn base_url(&self) -> &str {
        match self {
            Network::Testnet => "https://horizon-testnet.stellar.org",
            Network::Mainnet => "https://horizon.stellar.org",
            Network::Custom(url) => url.as_str(),
        }
    }
}

/// Connection pool for reusing HTTP connections.
pub struct ConnectionPool {
    clients: Vec<Client>,
    next: std::sync::atomic::AtomicUsize,
    pool_size: usize,
    hits: std::sync::atomic::AtomicU64,
    misses: std::sync::atomic::AtomicU64,
}

impl ConnectionPool {
    pub fn new(pool_size: usize, timeout: Duration) -> Self {
        let clients: Vec<Client> = (0..pool_size)
            .map(|_| {
                Client::builder()
                    .timeout(timeout)
                    .pool_idle_timeout(timeout)
                    .build()
                    .expect("failed to create HTTP client")
            })
            .collect();
        Self {
            clients,
            next: std::sync::atomic::AtomicUsize::new(0),
            pool_size,
            hits: std::sync::atomic::AtomicU64::new(0),
            misses: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn get(&self) -> &Client {
        let idx = self.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % self.pool_size;
        if idx > 0 {
            self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        } else {
            self.misses
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        &self.clients[idx]
    }

    pub fn hits(&self) -> u64 {
        self.hits.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn misses(&self) -> u64 {
        self.misses.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Cached fee stats with TTL and hit/miss counters.
struct CacheEntry {
    stats: HorizonFeeStats,
    fetched_at: Instant,
}

pub struct FeeStatsCache {
    entry: RwLock<Option<CacheEntry>>,
    ttl: Duration,
    hits: std::sync::atomic::AtomicU64,
    misses: std::sync::atomic::AtomicU64,
}

impl FeeStatsCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            entry: RwLock::new(None),
            ttl,
            hits: std::sync::atomic::AtomicU64::new(0),
            misses: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub async fn get(&self) -> Option<HorizonFeeStats> {
        let entry = self.entry.read().await;
        if let Some(ref e) = *entry {
            if e.fetched_at.elapsed() < self.ttl {
                self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Some(e.stats.clone());
            }
        }
        self.misses
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        None
    }

    pub async fn insert(&self, stats: HorizonFeeStats) {
        let mut entry = self.entry.write().await;
        *entry = Some(CacheEntry {
            stats,
            fetched_at: Instant::now(),
        });
    }

    pub fn cache_hits(&self) -> u64 {
        self.hits.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn cache_misses(&self) -> u64 {
        self.misses.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn age_secs(&self) -> Option<u64> {
        // This needs async access, return None as placeholder
        None
    }
}

/// Typed Horizon client with connection pooling, caching, and network selection.
pub struct HorizonClient {
    pub base_url: String,
    pub timeout_ms: u64,
    pool: Arc<ConnectionPool>,
    cache: Arc<FeeStatsCache>,
}

impl HorizonClient {
    pub fn new(base_url: String) -> Self {
        let timeout = Duration::from_millis(10_000);
        Self {
            base_url,
            timeout_ms: 10_000,
            pool: Arc::new(ConnectionPool::new(4, timeout)),
            cache: Arc::new(FeeStatsCache::new(Duration::from_secs(5))),
        }
    }

    pub fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    pub fn with_network(network: Network) -> Self {
        Self::new(network.base_url().to_string())
    }

    pub fn with_pool_size(mut self, size: usize) -> Self {
        self.pool = Arc::new(ConnectionPool::new(
            size,
            Duration::from_millis(self.timeout_ms),
        ));
        self
    }

    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache = Arc::new(FeeStatsCache::new(ttl));
        self
    }

    pub fn pool(&self) -> &ConnectionPool {
        &self.pool
    }

    pub fn cache(&self) -> &FeeStatsCache {
        &self.cache
    }

    pub async fn fetch_fee_stats(&self) -> Result<HorizonFeeStats, DevkitError> {
        if let Some(cached) = self.cache.get().await {
            return Ok(cached);
        }

        let url = format!("{}/fee_stats", self.base_url);
        let client = self.pool.get();
        let resp = client
            .get(&url)
            .timeout(Duration::from_millis(self.timeout_ms))
            .send()
            .await
            .map_err(|e| DevkitError::Protocol(format!("network error: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(DevkitError::Protocol(format!("HTTP {status}")));
        }

        let body = resp
            .text()
            .await
            .map_err(|e| DevkitError::Protocol(format!("response read error: {e}")))?;

        let stats = parse_fee_stats(&body)?;
        self.cache.insert(stats.clone()).await;
        Ok(stats)
    }
}
