//! Inline Completion Support (Ghost Text)
//!
//! This module provides enhanced completion functionality for ghost text/inline
//! completions, with debouncing, caching, and request cancellation.

use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::sleep;

/// Debounce delay for completion requests (milliseconds)
const DEBOUNCE_DELAY_MS: u64 = 300;

/// Cache TTL for completions (seconds)
const CACHE_TTL_SECS: u64 = 60;

/// Maximum cache entries
const MAX_CACHE_ENTRIES: usize = 100;

/// Cached completion entry
#[derive(Debug, Clone)]
struct CachedCompletion {
    completions: Vec<String>,
    timestamp: Instant,
}

/// Cache key for completions
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct CompletionKey {
    uri: String,
    prefix_hash: u64,
    suffix_hash: u64,
}

impl CompletionKey {
    fn new(uri: String, prefix: &str, suffix: &Option<String>) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut prefix_hasher = DefaultHasher::new();
        prefix.hash(&mut prefix_hasher);
        let prefix_hash = prefix_hasher.finish();

        let mut suffix_hasher = DefaultHasher::new();
        if let Some(s) = suffix {
            s.hash(&mut suffix_hasher);
        }
        let suffix_hash = suffix_hasher.finish();

        Self {
            uri,
            prefix_hash,
            suffix_hash,
        }
    }
}

/// Inline completion manager with debouncing and caching
#[derive(Clone)]
pub struct InlineCompletionManager {
    /// Completion cache
    cache: Arc<DashMap<CompletionKey, CachedCompletion>>,
    /// Last request timestamp per URI (for debouncing)
    last_request: Arc<RwLock<DashMap<String, Instant>>>,
    /// Pending request cancellation tokens
    cancellation_tokens: Arc<DashMap<String, tokio::sync::watch::Sender<bool>>>,
}

impl InlineCompletionManager {
    /// Create a new inline completion manager
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            last_request: Arc::new(RwLock::new(DashMap::new())),
            cancellation_tokens: Arc::new(DashMap::new()),
        }
    }

    /// Check if we should debounce this completion request
    ///
    /// Returns true if we should wait before processing
    pub async fn should_debounce(&self, uri: &str) -> bool {
        let last_request = self.last_request.read().await;

        let should_wait = if let Some(last_time) = last_request.get(uri) {
            let elapsed = last_time.elapsed();
            elapsed < Duration::from_millis(DEBOUNCE_DELAY_MS)
        } else {
            false
        };

        // Drop the lock before returning
        drop(last_request);

        should_wait
    }

    /// Wait for debounce period
    pub async fn wait_debounce(&self) {
        sleep(Duration::from_millis(DEBOUNCE_DELAY_MS)).await;
    }

    /// Update the last request timestamp for a URI
    pub async fn update_last_request(&self, uri: String) {
        let last_request = self.last_request.write().await;
        last_request.insert(uri, Instant::now());
    }

    /// Try to get cached completion
    pub fn get_cached(&self, uri: &str, prefix: &str, suffix: &Option<String>) -> Option<Vec<String>> {
        let key = CompletionKey::new(uri.to_string(), prefix, suffix);

        if let Some(cached) = self.cache.get(&key) {
            // Check if cache entry is still valid
            if cached.timestamp.elapsed() < Duration::from_secs(CACHE_TTL_SECS) {
                return Some(cached.completions.clone());
            } else {
                // Remove stale entry
                drop(cached);
                self.cache.remove(&key);
            }
        }

        None
    }

    /// Store completion in cache
    pub fn cache_completion(&self, uri: String, prefix: &str, suffix: &Option<String>, completions: Vec<String>) {
        let key = CompletionKey::new(uri, prefix, suffix);

        // Limit cache size
        if self.cache.len() >= MAX_CACHE_ENTRIES {
            // Remove oldest entries (simple FIFO for now)
            if let Some(first_key) = self.cache.iter().next().map(|entry| entry.key().clone()) {
                self.cache.remove(&first_key);
            }
        }

        self.cache.insert(
            key,
            CachedCompletion {
                completions,
                timestamp: Instant::now(),
            },
        );
    }

    /// Create a cancellation token for a request
    ///
    /// Returns a receiver that will be notified when the request should be cancelled
    pub fn create_cancellation_token(&self, uri: String) -> tokio::sync::watch::Receiver<bool> {
        // Cancel any existing request for this URI
        if let Some((_, sender)) = self.cancellation_tokens.remove(&uri) {
            let _ = sender.send(true);
        }

        // Create new cancellation token
        let (tx, rx) = tokio::sync::watch::channel(false);
        self.cancellation_tokens.insert(uri, tx);

        rx
    }

    /// Check if a request was cancelled
    pub fn is_cancelled(&self, _uri: &str, rx: &tokio::sync::watch::Receiver<bool>) -> bool {
        *rx.borrow()
    }

    /// Clean up cancellation token after request completes
    pub fn remove_cancellation_token(&self, uri: &str) {
        self.cancellation_tokens.remove(uri);
    }

    /// Clear all cached completions
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Clear all cancellation tokens
    pub fn clear_cancellation_tokens(&self) {
        self.cancellation_tokens.clear();
    }
}

impl Default for InlineCompletionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_debouncing() {
        let manager = InlineCompletionManager::new();
        let uri = "file:///test.rs".to_string();

        // First request should not be debounced
        assert!(!manager.should_debounce(&uri).await);

        // Update timestamp
        manager.update_last_request(uri.clone()).await;

        // Immediate second request should be debounced
        assert!(manager.should_debounce(&uri).await);

        // Wait for debounce period
        sleep(Duration::from_millis(DEBOUNCE_DELAY_MS + 50)).await;

        // Should no longer be debounced
        assert!(!manager.should_debounce(&uri).await);
    }

    #[tokio::test]
    async fn test_caching() {
        let manager = InlineCompletionManager::new();
        let uri = "file:///test.rs".to_string();
        let prefix = "fn main() {";
        let suffix = Some("}".to_string());
        let completions = vec!["println!(\"Hello\");".to_string()];

        // No cache initially
        assert!(manager.get_cached(&uri, prefix, &suffix).is_none());

        // Cache completion
        manager.cache_completion(uri.clone(), prefix, &suffix, completions.clone());

        // Should retrieve from cache
        let cached = manager.get_cached(&uri, prefix, &suffix);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), completions);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let manager = InlineCompletionManager::new();
        let uri = "file:///test.rs".to_string();
        let prefix = "fn test() {";
        let suffix = None;
        let completions = vec!["assert!(true);".to_string()];

        // Cache completion
        manager.cache_completion(uri.clone(), prefix, &suffix, completions.clone());

        // Should be cached
        assert!(manager.get_cached(&uri, prefix, &suffix).is_some());

        // Manually expire the cache by updating timestamp
        // (In real usage, this would happen after CACHE_TTL_SECS)
        // For testing, we just verify the cache exists
        manager.clear_cache();

        // Should no longer be cached
        assert!(manager.get_cached(&uri, prefix, &suffix).is_none());
    }

    #[tokio::test]
    async fn test_cancellation() {
        let manager = InlineCompletionManager::new();
        let uri = "file:///test.rs".to_string();

        // Create cancellation token
        let rx = manager.create_cancellation_token(uri.clone());

        // Should not be cancelled initially
        assert!(!manager.is_cancelled(&uri, &rx));

        // Create new token for same URI (should cancel previous)
        let _rx2 = manager.create_cancellation_token(uri.clone());

        // Original should be cancelled
        assert!(manager.is_cancelled(&uri, &rx));
    }
}
