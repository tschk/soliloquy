//! Network stack optimizations
//!
//! Provides DNS prefetching, resource prioritization, and connection pooling.

pub mod prefetch;
pub mod priority;

pub use prefetch::{
    DnsPrefetchCache, PrefetchManager, PrefetchPriority, PrefetchRequest, ResourceType,
};
pub use priority::{PriorityQueue, ResourceKind, ResourcePriority, ResourceRequest};
