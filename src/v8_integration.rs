// ponytail: stub — replace with rv8::js when rv8 linkage lands
// Another agent handles real rv8 integration.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcType {
    Minor,
    Major,
}

#[derive(Debug)]
pub struct GcScheduler {
    gc_count: u64,
}

impl GcScheduler {
    pub fn new() -> Self {
        GcScheduler { gc_count: 0 }
    }

    pub fn request_gc(&mut self, _gc_type: GcType) {
        self.gc_count += 1;
    }

    pub fn gc_count(&self) -> u64 {
        self.gc_count
    }
}

impl Default for GcScheduler {
    fn default() -> Self {
        Self::new()
    }
}
