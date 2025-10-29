//! Response Cache for MCP Coordinator
//!
//! Provides TTL-based caching of MCP responses with automatic cleanup.

use crate::mcp::McpResponse;
use dashmap::DashMap;
use std::time::{Duration, Instant};

/// TTL-based response cache
pub struct ResponseCache {
    /// Cache entries: (server_name, request_hash) -> (response, expiry_time)
    cache: DashMap<String, (McpResponse, Instant)>,

    /// Default TTL for cache entries
    default_ttl: Duration,

    /// Hit counter
    hits: std::sync::atomic::AtomicU64,

    /// Miss counter
    misses: std::sync::atomic::AtomicU64,
}

impl ResponseCache {
    /// Create a new response cache with default TTL
    pub fn new(default_ttl_seconds: u64) -> Self {
        Self {
            cache: DashMap::new(),
            default_ttl: Duration::from_secs(default_ttl_seconds),
            hits: std::sync::atomic::AtomicU64::new(0),
            misses: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Get a cached response if it exists and hasn't expired
    pub fn get(&self, key: &str) -> Option<McpResponse> {
        if let Some(entry) = self.cache.get(key) {
            let (response, expiry) = entry.value();

            // Check if expired
            if expiry > &Instant::now() {
                self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Some(response.clone());
            } else {
                // Remove expired entry
                drop(entry);
                self.cache.remove(key);
            }
        }

        self.misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        None
    }

    /// Set a cache entry with custom TTL
    pub fn set(&self, key: String, response: McpResponse, ttl_seconds: Option<u64>) {
        let ttl = ttl_seconds.map(Duration::from_secs).unwrap_or(self.default_ttl);
        let expiry = Instant::now() + ttl;

        self.cache.insert(key, (response, expiry));
    }

    /// Get cache hit count
    pub fn hits(&self) -> u64 {
        self.hits.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get cache miss count
    pub fn misses(&self) -> u64 {
        self.misses.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get cache size
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Clean up expired entries
    pub fn cleanup_expired(&self) -> usize {
        let now = Instant::now();
        let mut removed = 0;

        self.cache.retain(|_, (_, expiry)| {
            let keep = *expiry > now;
            if !keep {
                removed += 1;
            }
            keep
        });

        if removed > 0 {
            log::debug!("Cleaned up {} expired cache entries", removed);
        }

        removed
    }

    /// Clear all cache entries
    pub fn clear(&self) {
        let count = self.cache.len();
        self.cache.clear();
        log::info!("Cleared cache ({} entries)", count);
    }

    /// Generate cache key from server name and request
    pub fn make_key(server_name: &str, request: &crate::mcp::McpRequest) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        request.request_type.hash(&mut hasher);
        request.uri.hash(&mut hasher);
        request.position.line.hash(&mut hasher);
        request.position.character.hash(&mut hasher);

        if let Some(ctx) = &request.context {
            ctx.hash(&mut hasher);
        }

        format!("{}:{}", server_name, hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::Position;

    fn make_test_response() -> McpResponse {
        McpResponse {
            suggestions: vec!["test".to_string()],
            documentation: Some("Test doc".to_string()),
            confidence: Some(0.9),
        }
    }

    #[test]
    fn test_cache_creation() {
        let cache = ResponseCache::new(300);
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
        assert_eq!(cache.hits(), 0);
        assert_eq!(cache.misses(), 0);
    }

    #[test]
    fn test_cache_set_and_get() {
        let cache = ResponseCache::new(300);
        let response = make_test_response();

        cache.set("test_key".to_string(), response.clone(), None);
        assert_eq!(cache.len(), 1);

        let retrieved = cache.get("test_key");
        assert!(retrieved.is_some());
        assert_eq!(cache.hits(), 1);

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.suggestions, response.suggestions);
    }

    #[test]
    fn test_cache_miss() {
        let cache = ResponseCache::new(300);
        let result = cache.get("nonexistent");

        assert!(result.is_none());
        assert_eq!(cache.misses(), 1);
    }

    #[test]
    fn test_cache_expiry() {
        let cache = ResponseCache::new(1); // 1 second TTL
        let response = make_test_response();

        // Set with very short TTL
        cache.set("test_key".to_string(), response, Some(0)); // 0 seconds = immediate expiry

        // Should be expired immediately
        std::thread::sleep(std::time::Duration::from_millis(10));
        let result = cache.get("test_key");
        assert!(result.is_none());
        assert_eq!(cache.misses(), 1);
    }

    #[test]
    fn test_cleanup_expired() {
        let cache = ResponseCache::new(1);
        let response = make_test_response();

        // Add multiple entries with 0 TTL
        cache.set("key1".to_string(), response.clone(), Some(0));
        cache.set("key2".to_string(), response.clone(), Some(0));
        cache.set("key3".to_string(), response, Some(300)); // This one doesn't expire

        assert_eq!(cache.len(), 3);

        // Wait and cleanup
        std::thread::sleep(std::time::Duration::from_millis(10));
        let removed = cache.cleanup_expired();

        assert_eq!(removed, 2);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_clear() {
        let cache = ResponseCache::new(300);
        let response = make_test_response();

        cache.set("key1".to_string(), response.clone(), None);
        cache.set("key2".to_string(), response, None);
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_make_key() {
        let request1 = crate::mcp::McpRequest {
            request_type: "completion".to_string(),
            uri: "file:///test.rs".to_string(),
            position: Position { line: 10, character: 5 },
            context: Some("fn main()".to_string()),
        };

        let request2 = crate::mcp::McpRequest {
            request_type: "completion".to_string(),
            uri: "file:///test.rs".to_string(),
            position: Position { line: 10, character: 5 },
            context: Some("fn main()".to_string()),
        };

        let key1 = ResponseCache::make_key("server1", &request1);
        let key2 = ResponseCache::make_key("server1", &request2);

        // Same request should produce same key
        assert_eq!(key1, key2);

        // Different server should produce different key
        let key3 = ResponseCache::make_key("server2", &request1);
        assert_ne!(key1, key3);
    }
}
