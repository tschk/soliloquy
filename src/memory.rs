// ponytail: stub — replace with rv8::optimizations when rv8 linkage lands
// Another agent handles real rv8 integration.

#[derive(Debug, Default, Clone)]
pub struct TabStats {
    pub total_tabs: usize,
    pub active_tabs: usize,
    pub frozen_tabs: usize,
    pub discarded_tabs: usize,
    pub total_memory_bytes: u64,
    pub active_memory_bytes: u64,
}

#[derive(Debug)]
pub struct TabResidencyManager {
    max_tabs: usize,
}

impl TabResidencyManager {
    pub fn new() -> Self {
        TabResidencyManager { max_tabs: 32 }
    }

    pub fn new_with_max(max: usize) -> Self {
        TabResidencyManager { max_tabs: max }
    }

    pub fn max_tabs(&self) -> usize {
        self.max_tabs
    }

    pub fn freeze_tab(&mut self, _tab_id: u64) {
        // stub: real impl in rv8
    }

    pub fn discard_tab(&mut self, _tab_id: u64) {
        // stub: real impl in rv8
    }
}

impl Default for TabResidencyManager {
    fn default() -> Self {
        Self::new()
    }
}
