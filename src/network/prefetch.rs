//! DNS and resource prefetching system
//!
//! Implements predictive prefetching based on user interactions
//! to reduce perceived latency.

use log::{debug, info};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

/// Prefetch priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrefetchPriority {
    /// Low priority - prefetch during heavy idle
    Low = 0,
    /// Medium priority - prefetch during normal idle
    Medium = 1,
    /// High priority - prefetch immediately
    High = 2,
}

/// Type of prefetch resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    /// DNS resolution
    Dns,
    /// HTTP connection
    Connection,
    /// Complete resource
    Resource,
}

/// Prefetch request
#[derive(Debug, Clone)]
pub struct PrefetchRequest {
    /// Resource URL or hostname
    pub url: String,
    /// Type of prefetch
    pub resource_type: ResourceType,
    /// Priority level
    pub priority: PrefetchPriority,
    /// Requested timestamp
    pub requested_at: Instant,
}

impl PrefetchRequest {
    /// Create a new prefetch request
    pub fn new(url: String, resource_type: ResourceType, priority: PrefetchPriority) -> Self {
        PrefetchRequest {
            url,
            resource_type,
            priority,
            requested_at: Instant::now(),
        }
    }

    /// Get age of this request
    pub fn age(&self) -> std::time::Duration {
        self.requested_at.elapsed()
    }
}

/// Prefetch manager
pub struct PrefetchManager {
    /// Pending prefetch requests
    pending: Vec<PrefetchRequest>,
    /// Completed prefetches (for deduplication)
    completed: HashSet<String>,
    /// Link hover timestamps for prediction
    hover_history: HashMap<String, Vec<Instant>>,
    /// Enable predictive prefetching
    predictive_enabled: bool,
}

impl PrefetchManager {
    /// Create a new prefetch manager
    pub fn new() -> Self {
        PrefetchManager {
            pending: Vec::new(),
            completed: HashSet::new(),
            hover_history: HashMap::new(),
            predictive_enabled: true,
        }
    }

    /// Request a prefetch
    pub fn request_prefetch(
        &mut self,
        url: String,
        resource_type: ResourceType,
        priority: PrefetchPriority,
    ) {
        // Check if already completed or pending
        if self.completed.contains(&url) {
            debug!("Prefetch already completed: {}", url);
            return;
        }

        if self
            .pending
            .iter()
            .any(|r| r.url == url && r.resource_type == resource_type)
        {
            debug!("Prefetch already pending: {}", url);
            return;
        }

        let request = PrefetchRequest::new(url.clone(), resource_type, priority);
        self.pending.push(request);

        info!(
            "Queued prefetch: {} ({:?}, {:?})",
            url, resource_type, priority
        );
    }

    /// Record link hover for predictive prefetching
    pub fn record_hover(&mut self, url: String) {
        if !self.predictive_enabled || url.is_empty() {
            return;
        }

        let history = self.hover_history.entry(url.clone()).or_default();
        history.push(Instant::now());

        // If hovered multiple times or for extended period, prefetch
        if history.len() >= 2 {
            debug!("Predictive prefetch triggered for: {}", url);
            self.request_prefetch(url, ResourceType::Dns, PrefetchPriority::Medium);
        }
    }

    /// Get next prefetch request to execute
    pub fn next_request(&mut self) -> Option<PrefetchRequest> {
        if self.pending.is_empty() {
            return None;
        }

        // Sort by priority (lowest first) so highest is at the end
        self.pending.sort_by(|a, b| a.priority.cmp(&b.priority));

        // Take highest priority request in O(1)
        self.pending.pop()
    }

    /// Mark a prefetch as completed
    pub fn mark_completed(&mut self, url: String) {
        self.completed.insert(url.clone());
        debug!("Prefetch completed: {}", url);
    }

    /// Get pending request count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Enable/disable predictive prefetching
    pub fn set_predictive(&mut self, enabled: bool) {
        self.predictive_enabled = enabled;
        info!(
            "Predictive prefetching: {}",
            if enabled { "enabled" } else { "disabled" }
        );
    }

    /// Clear completed prefetches (for memory management)
    pub fn clear_completed(&mut self) {
        self.completed.clear();
        self.hover_history.clear();
    }
}

impl Default for PrefetchManager {
    fn default() -> Self {
        Self::new()
    }
}

/// DNS prefetch cache
pub struct DnsPrefetchCache {
    /// Cached DNS resolutions
    cache: HashMap<String, Vec<String>>,
    /// Prefetch queue
    queue: Vec<String>,
}

impl DnsPrefetchCache {
    /// Create a new DNS prefetch cache
    pub fn new() -> Self {
        DnsPrefetchCache {
            cache: HashMap::new(),
            queue: Vec::new(),
        }
    }

    /// Queue a hostname for DNS prefetch
    pub fn prefetch(&mut self, hostname: String) {
        if !self.cache.contains_key(&hostname) && !self.queue.contains(&hostname) {
            debug!("Queueing DNS prefetch: {}", hostname);
            self.queue.push(hostname);
        }
    }

    /// Get next hostname to resolve
    pub fn next_hostname(&mut self) -> Option<String> {
        if self.queue.is_empty() {
            None
        } else {
            Some(self.queue.remove(0))
        }
    }

    /// Store resolved addresses
    pub fn store_resolution(&mut self, hostname: String, addresses: Vec<String>) {
        debug!("Cached DNS resolution for {}: {:?}", hostname, addresses);
        self.cache.insert(hostname, addresses);
    }

    /// Get cached resolution
    pub fn get_resolution(&self, hostname: &str) -> Option<&[String]> {
        self.cache.get(hostname).map(|v| v.as_slice())
    }

    /// Clear cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.queue.clear();
    }
}

impl Default for DnsPrefetchCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefetch_request() {
        let req = PrefetchRequest::new(
            "https://example.com".to_string(),
            ResourceType::Resource,
            PrefetchPriority::High,
        );

        assert_eq!(req.priority, PrefetchPriority::High);
        assert_eq!(req.resource_type, ResourceType::Resource);
    }

    #[test]
    fn test_prefetch_request_empty_url() {
        let req = PrefetchRequest::new("".to_string(), ResourceType::Dns, PrefetchPriority::Low);

        assert_eq!(req.url, "");
        assert_eq!(req.resource_type, ResourceType::Dns);
        assert_eq!(req.priority, PrefetchPriority::Low);
    }

    #[test]
    fn test_prefetch_manager() {
        let mut manager = PrefetchManager::new();

        manager.request_prefetch(
            "https://example.com".to_string(),
            ResourceType::Dns,
            PrefetchPriority::Medium,
        );

        assert_eq!(manager.pending_count(), 1);

        let req = manager.next_request();
        assert!(req.is_some());
        assert_eq!(req.unwrap().url, "https://example.com");
    }

    #[test]
    fn test_prefetch_deduplication() {
        let mut manager = PrefetchManager::new();

        manager.request_prefetch(
            "https://example.com".to_string(),
            ResourceType::Dns,
            PrefetchPriority::Medium,
        );

        manager.request_prefetch(
            "https://example.com".to_string(),
            ResourceType::Dns,
            PrefetchPriority::High,
        );

        // Should only have one request
        assert_eq!(manager.pending_count(), 1);
    }

    #[test]
    fn test_prefetch_priority_sorting() {
        let mut manager = PrefetchManager::new();

        manager.request_prefetch("low".to_string(), ResourceType::Dns, PrefetchPriority::Low);
        manager.request_prefetch(
            "high".to_string(),
            ResourceType::Dns,
            PrefetchPriority::High,
        );
        manager.request_prefetch(
            "medium".to_string(),
            ResourceType::Dns,
            PrefetchPriority::Medium,
        );

        let first = manager.next_request().unwrap();
        assert_eq!(first.url, "high");
        assert_eq!(first.priority, PrefetchPriority::High);
    }

    #[test]
    fn test_hover_prediction() {
        let mut manager = PrefetchManager::new();

        manager.record_hover("https://example.com".to_string());
        manager.record_hover("https://example.com".to_string());

        // Should trigger prefetch
        assert!(manager.pending_count() > 0);
    }

    #[test]
    fn test_hover_empty_string_ignored() {
        let mut manager = PrefetchManager::new();

        manager.record_hover("".to_string());
        manager.record_hover("".to_string());

        assert_eq!(manager.pending_count(), 0);
    }

    #[test]
    fn test_dns_prefetch_cache() {
        let mut cache = DnsPrefetchCache::new();

        cache.prefetch("example.com".to_string());
        assert_eq!(cache.next_hostname(), Some("example.com".to_string()));

        cache.store_resolution("example.com".to_string(), vec!["93.184.216.34".to_string()]);

        let resolution = cache.get_resolution("example.com");
        assert_eq!(resolution, Some(&["93.184.216.34".to_string()][..]));
    }

    #[test]
    fn test_completed_marking() {
        let mut manager = PrefetchManager::new();

        manager.request_prefetch(
            "https://example.com".to_string(),
            ResourceType::Dns,
            PrefetchPriority::High,
        );

        // Consume the pending request
        manager.next_request();

        manager.mark_completed("https://example.com".to_string());

        // Should not add duplicate since it's completed
        manager.request_prefetch(
            "https://example.com".to_string(),
            ResourceType::Dns,
            PrefetchPriority::High,
        );

        assert_eq!(manager.pending_count(), 0);
    }
}
