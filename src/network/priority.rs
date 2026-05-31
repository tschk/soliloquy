//! Resource loading prioritization
//!
//! Prioritizes critical rendering path resources to improve
//! perceived page load performance.

use log::{debug, info};
use std::cmp::Ordering;

/// Resource priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourcePriority {
    /// Lowest priority - defer until idle
    Idle = 0,
    /// Low priority - load in background
    Low = 1,
    /// Medium priority - normal loading
    Medium = 2,
    /// High priority - important for rendering
    High = 3,
    /// Critical priority - blocking render
    Critical = 4,
}

/// Resource types with intrinsic priorities
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    /// HTML document
    Document,
    /// CSS stylesheet
    Stylesheet,
    /// JavaScript
    Script,
    /// Image
    Image,
    /// Font file
    Font,
    /// XHR/Fetch request
    Xhr,
    /// Media (video/audio)
    Media,
    /// Other resources
    Other,
}

impl ResourceKind {
    /// Get default priority for this resource type
    pub fn default_priority(&self) -> ResourcePriority {
        match self {
            ResourceKind::Document => ResourcePriority::Critical,
            ResourceKind::Stylesheet => ResourcePriority::Critical,
            ResourceKind::Script => ResourcePriority::High,
            ResourceKind::Font => ResourcePriority::High,
            ResourceKind::Xhr => ResourcePriority::Medium,
            ResourceKind::Image => ResourcePriority::Low,
            ResourceKind::Media => ResourcePriority::Low,
            ResourceKind::Other => ResourcePriority::Low,
        }
    }
}

/// Resource loading request
#[derive(Debug, Clone)]
pub struct ResourceRequest {
    /// Resource URL
    pub url: String,
    /// Resource type
    pub kind: ResourceKind,
    /// Priority (can be overridden from default)
    pub priority: ResourcePriority,
    /// Whether this resource is in viewport
    pub in_viewport: bool,
    /// Whether this resource is preloaded
    pub is_preload: bool,
    /// Request ID for tracking
    pub id: u64,
}

impl ResourceRequest {
    /// Create a new resource request with default priority
    pub fn new(id: u64, url: String, kind: ResourceKind) -> Self {
        let priority = kind.default_priority();
        ResourceRequest {
            url,
            kind,
            priority,
            in_viewport: false,
            is_preload: false,
            id,
        }
    }

    /// Override priority
    pub fn with_priority(mut self, priority: ResourcePriority) -> Self {
        self.priority = priority;
        self
    }

    /// Mark as in viewport
    pub fn in_viewport(mut self, in_viewport: bool) -> Self {
        self.in_viewport = in_viewport;
        self
    }

    /// Mark as preload
    pub fn preload(mut self) -> Self {
        self.is_preload = true;
        self
    }

    /// Calculate effective priority considering all factors
    pub fn effective_priority(&self) -> ResourcePriority {
        let mut priority = self.priority;

        // Boost priority for viewport resources
        if self.in_viewport && priority < ResourcePriority::High {
            priority = ResourcePriority::High;
        }

        // Boost priority for preload resources
        if self.is_preload && priority < ResourcePriority::High {
            priority = ResourcePriority::High;
        }

        priority
    }
}

impl PartialEq for ResourceRequest {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ResourceRequest {}

impl PartialOrd for ResourceRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ResourceRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        // Lower priority comes first (so higher priority ends up at the end for pop())
        self.effective_priority().cmp(&other.effective_priority())
    }
}

/// Resource priority queue
pub struct PriorityQueue {
    /// Pending requests
    requests: Vec<ResourceRequest>,
    /// Next request ID
    next_id: u64,
    /// Maximum concurrent requests
    max_concurrent: usize,
    /// Currently active requests
    active_count: usize,
    /// Whether the queue needs sorting
    needs_sort: bool,
}

impl PriorityQueue {
    /// Create a new priority queue
    pub fn new(max_concurrent: usize) -> Self {
        info!(
            "Creating resource priority queue (max concurrent: {})",
            max_concurrent
        );

        PriorityQueue {
            requests: Vec::new(),
            next_id: 1,
            max_concurrent,
            active_count: 0,
            needs_sort: false,
        }
    }

    /// Enqueue a resource request
    pub fn enqueue(&mut self, mut request: ResourceRequest) -> u64 {
        request.id = self.next_id;
        self.next_id += 1;
        let id = request.id;

        debug!(
            "Enqueued resource: {} ({:?}, {:?})",
            request.url,
            request.kind,
            request.effective_priority()
        );

        self.requests.push(request);
        self.needs_sort = true;

        id
    }

    /// Dequeue the next highest-priority request
    pub fn dequeue(&mut self) -> Option<ResourceRequest> {
        if self.active_count >= self.max_concurrent {
            return None;
        }

        // Sort only when dequeuing if needed
        if self.needs_sort {
            self.requests.sort();
            self.needs_sort = false;
        }

        if let Some(request) = self.requests.pop() {
            self.active_count += 1;
            debug!(
                "Dequeued resource: {} (priority: {:?})",
                request.url,
                request.effective_priority()
            );
            Some(request)
        } else {
            None
        }
    }

    /// Mark a request as completed
    pub fn complete(&mut self, _id: u64) {
        if self.active_count > 0 {
            self.active_count -= 1;
        }
    }

    /// Get pending request count
    pub fn pending_count(&self) -> usize {
        self.requests.len()
    }

    /// Get active request count
    pub fn active_count(&self) -> usize {
        self.active_count
    }

    /// Check if queue can accept more requests
    pub fn can_dequeue(&self) -> bool {
        self.active_count < self.max_concurrent && !self.requests.is_empty()
    }
}

impl Default for PriorityQueue {
    fn default() -> Self {
        Self::new(6) // HTTP/2 default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_kind_priorities() {
        assert!(ResourceKind::Document.default_priority() > ResourceKind::Image.default_priority());
        assert!(
            ResourceKind::Stylesheet.default_priority() > ResourceKind::Media.default_priority()
        );
    }

    #[test]
    fn test_resource_request() {
        let req = ResourceRequest::new(1, "style.css".to_string(), ResourceKind::Stylesheet);
        assert_eq!(req.priority, ResourcePriority::Critical);
        assert_eq!(req.kind, ResourceKind::Stylesheet);
    }

    #[test]
    fn test_priority_override() {
        let req = ResourceRequest::new(1, "image.jpg".to_string(), ResourceKind::Image)
            .with_priority(ResourcePriority::High);

        assert_eq!(req.priority, ResourcePriority::High);
    }

    #[test]
    fn test_viewport_boost() {
        let req =
            ResourceRequest::new(1, "image.jpg".to_string(), ResourceKind::Image).in_viewport(true);

        // Should boost to High
        assert_eq!(req.effective_priority(), ResourcePriority::High);
    }

    #[test]
    fn test_preload_boost() {
        let req = ResourceRequest::new(1, "font.woff2".to_string(), ResourceKind::Font)
            .with_priority(ResourcePriority::Medium)
            .preload();

        assert_eq!(req.effective_priority(), ResourcePriority::High);
    }

    #[test]
    fn test_priority_queue() {
        let mut queue = PriorityQueue::new(6);

        queue.enqueue(ResourceRequest::new(
            0,
            "image.jpg".to_string(),
            ResourceKind::Image,
        ));
        queue.enqueue(ResourceRequest::new(
            0,
            "style.css".to_string(),
            ResourceKind::Stylesheet,
        ));
        queue.enqueue(ResourceRequest::new(
            0,
            "script.js".to_string(),
            ResourceKind::Script,
        ));

        // Should dequeue stylesheet first (Critical priority)
        let first = queue.dequeue().unwrap();
        assert_eq!(first.kind, ResourceKind::Stylesheet);
    }

    #[test]
    fn test_max_concurrent() {
        let mut queue = PriorityQueue::new(2);

        queue.enqueue(ResourceRequest::new(
            0,
            "1.jpg".to_string(),
            ResourceKind::Image,
        ));
        queue.enqueue(ResourceRequest::new(
            0,
            "2.jpg".to_string(),
            ResourceKind::Image,
        ));
        queue.enqueue(ResourceRequest::new(
            0,
            "3.jpg".to_string(),
            ResourceKind::Image,
        ));

        // Should get 2 requests
        assert!(queue.dequeue().is_some());
        assert!(queue.dequeue().is_some());

        // Third should be blocked
        assert!(queue.dequeue().is_none());

        // Complete one
        queue.complete(1);

        // Now can dequeue third
        assert!(queue.dequeue().is_some());
    }

    #[test]
    fn test_queue_stats() {
        let mut queue = PriorityQueue::new(6);

        queue.enqueue(ResourceRequest::new(
            0,
            "1.jpg".to_string(),
            ResourceKind::Image,
        ));
        queue.enqueue(ResourceRequest::new(
            0,
            "2.jpg".to_string(),
            ResourceKind::Image,
        ));

        assert_eq!(queue.pending_count(), 2);
        assert_eq!(queue.active_count(), 0);

        queue.dequeue();

        assert_eq!(queue.pending_count(), 1);
        assert_eq!(queue.active_count(), 1);
    }

    #[test]
    fn test_request_ordering() {
        let low = ResourceRequest::new(1, "low".to_string(), ResourceKind::Image);
        let high = ResourceRequest::new(2, "high".to_string(), ResourceKind::Stylesheet);

        assert!(low < high); // Lower priority sorts first (higher at end for pop)
    }
}
