//! App registry — tracks installed desktop apps and their state.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub command: String,
    pub icon: String,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppInstance {
    pub entry: AppEntry,
    pub pid: Option<u32>,
    pub state: AppState,
}

#[derive(Debug, Clone, Serialize)]
pub enum AppState {
    Running,
    Stopped,
    Crashed(String),
}

pub struct AppRegistry {
    entries: Vec<AppEntry>,
    instances: Mutex<HashMap<String, AppInstance>>,
}

impl AppRegistry {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            instances: Mutex::new(HashMap::new()),
        }
    }

    pub fn register(&mut self, entry: AppEntry) {
        self.entries.push(entry);
    }

    pub fn entries(&self) -> &[AppEntry] {
        &self.entries
    }

    pub fn launch(&self, id: &str) -> Option<AppInstance> {
        let entry = self.entries.iter().find(|e| e.id == id)?.clone();

        // ponytail: no subprocess spawning yet — returns stopped state.
        // Add std::process::Command spawning when app launching is wired.
        let instance = AppInstance {
            entry: entry.clone(),
            pid: None,
            state: AppState::Stopped,
        };

        let mut instances = self.instances.lock().unwrap();
        instances.insert(id.to_string(), instance.clone());
        Some(instance)
    }

    pub fn instances(&self) -> Vec<AppInstance> {
        self.instances
            .lock()
            .unwrap()
            .values()
            .map(|i| i.clone())
            .collect()
    }
}
