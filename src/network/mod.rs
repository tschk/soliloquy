//! Network stack optimizations
//!
//! Provides DNS prefetching, resource prioritization, and connection pooling.

pub mod prefetch;
pub mod priority;

pub use prefetch::{
    PrefetchManager, PrefetchRequest, PrefetchPriority, ResourceType, DnsPrefetchCache,
};
pub use priority::{
    PriorityQueue, ResourceRequest, ResourcePriority, ResourceKind,
};
