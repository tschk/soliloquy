//! Navigation history and controller

/// A single navigation entry
#[derive(Debug, Clone)]
pub struct NavigationEntry {
    pub url: String,
    pub title: Option<String>,
    pub timestamp: std::time::Instant,
}

/// Navigation history and control for a tab
pub struct NavigationController {
    entries: Vec<NavigationEntry>,
    current_index: usize,
}

impl NavigationController {
    /// Create a new navigation controller
    pub fn new(initial_url: String) -> Self {
        NavigationController {
            entries: vec![NavigationEntry {
                url: initial_url,
                title: None,
                timestamp: std::time::Instant::now(),
            }],
            current_index: 0,
        }
    }

    /// Push a new navigation entry
    pub fn push(&mut self, url: String) {
        // Remove forward history
        self.entries.truncate(self.current_index + 1);

        // Add new entry
        self.entries.push(NavigationEntry {
            url,
            title: None,
            timestamp: std::time::Instant::now(),
        });
        self.current_index = self.entries.len() - 1;
    }

    /// Go back in history
    pub fn go_back(&mut self) -> Option<String> {
        if self.can_go_back() {
            self.current_index -= 1;
            Some(self.entries[self.current_index].url.clone())
        } else {
            None
        }
    }

    /// Go forward in history
    pub fn go_forward(&mut self) -> Option<String> {
        if self.can_go_forward() {
            self.current_index += 1;
            Some(self.entries[self.current_index].url.clone())
        } else {
            None
        }
    }

    /// Can go back?
    pub fn can_go_back(&self) -> bool {
        self.current_index > 0
    }

    /// Can go forward?
    pub fn can_go_forward(&self) -> bool {
        self.current_index < self.entries.len() - 1
    }

    /// Get current entry
    pub fn current(&self) -> Option<&NavigationEntry> {
        self.entries.get(self.current_index)
    }

    /// Update title of current entry
    pub fn set_current_title(&mut self, title: String) {
        if let Some(entry) = self.entries.get_mut(self.current_index) {
            entry.title = Some(title);
        }
    }

    /// Get all entries
    pub fn entries(&self) -> &[NavigationEntry] {
        &self.entries
    }

    /// Get current index
    pub fn current_index(&self) -> usize {
        self.current_index
    }
}
