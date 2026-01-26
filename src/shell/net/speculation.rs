//! Speculation Engine
//!
//! Implements speculation rules for prefetching and prerendering resources
//! based on user behavior, navigation patterns, and explicit rules.

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use glob::Pattern;

/// Speculation action type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpeculationAction {
    /// Prefetch resources (download but don't execute)
    Prefetch,
    /// Prerender page (fully load and render)
    Prerender,
}

/// URL pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum UrlPattern {
    /// Exact URL match
    Exact { url: String },
    /// URL prefix match
    Prefix { prefix: String },
    /// URL contains substring
    Contains { substring: String },
    /// Glob pattern match
    Glob { pattern: String },
}

impl UrlPattern {
    /// Check if URL matches this pattern
    pub fn matches(&self, url: &str) -> bool {
        match self {
            UrlPattern::Exact { url: pattern_url } => url == pattern_url,
            UrlPattern::Prefix { prefix } => url.starts_with(prefix),
            UrlPattern::Contains { substring } => url.contains(substring),
            UrlPattern::Glob { pattern } => {
                Pattern::new(pattern)
                    .map(|p| p.matches(url))
                    .unwrap_or(false)
            }
        }
    }
}

/// Individual speculation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeculationRule {
    /// Action to perform
    pub action: SpeculationAction,
    /// URL patterns to match
    pub patterns: Vec<UrlPattern>,
    /// Minimum probability threshold (0.0-1.0)
    #[serde(default = "default_probability")]
    pub min_probability: f32,
    /// Eagerness level: immediate, moderate, conservative
    #[serde(default = "default_eagerness")]
    pub eagerness: String,
}

fn default_probability() -> f32 {
    0.5
}

fn default_eagerness() -> String {
    "moderate".to_string()
}

impl SpeculationRule {
    /// Check if URL matches any pattern in this rule
    pub fn matches(&self, url: &str) -> bool {
        self.patterns.iter().any(|p| p.matches(url))
    }
}

/// Collection of speculation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeculationRules {
    pub rules: Vec<SpeculationRule>,
}

impl SpeculationRules {
    /// Parse speculation rules from JSON
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse speculation rules: {}", e))
    }

    /// Create empty rule set
    pub fn empty() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a rule
    pub fn add_rule(&mut self, rule: SpeculationRule) {
        self.rules.push(rule);
    }

    /// Get matching rules for a URL
    pub fn matching_rules(&self, url: &str) -> Vec<&SpeculationRule> {
        self.rules.iter().filter(|r| r.matches(url)).collect()
    }
}

/// Hover tracking for link prediction
#[derive(Debug)]
pub struct HoverTracker {
    /// Link URL -> (hover count, total hover duration)
    hover_data: HashMap<String, (u32, Duration)>,
    /// Currently hovered link
    current_hover: Option<(String, SystemTime)>,
}

impl HoverTracker {
    pub fn new() -> Self {
        Self {
            hover_data: HashMap::new(),
            current_hover: None,
        }
    }

    /// Record hover start
    pub fn hover_start(&mut self, url: &str) {
        self.current_hover = Some((url.to_string(), SystemTime::now()));
        debug!("Hover started: {}", url);
    }

    /// Record hover end
    pub fn hover_end(&mut self) {
        if let Some((url, start_time)) = self.current_hover.take() {
            let duration = SystemTime::now()
                .duration_since(start_time)
                .unwrap_or(Duration::from_secs(0));

            let entry = self.hover_data.entry(url.clone()).or_insert((0, Duration::from_secs(0)));
            entry.0 += 1;
            entry.1 += duration;

            debug!("Hover ended: {} (duration: {:?})", url, duration);
        }
    }

    /// Get hover probability for a URL (0.0-1.0)
    pub fn get_probability(&self, url: &str) -> f32 {
        if let Some((count, duration)) = self.hover_data.get(url) {
            // Simple heuristic: combine count and duration
            let count_score = (*count as f32) / 10.0; // Max out at 10 hovers
            let duration_score = duration.as_secs_f32() / 5.0; // Max out at 5 seconds
            
            ((count_score + duration_score) / 2.0).min(1.0)
        } else {
            0.0
        }
    }
}

impl Default for HoverTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Omnibox prediction based on history
#[derive(Debug)]
pub struct OmniboxPredictor {
    /// Navigation history (most recent first)
    history: VecDeque<String>,
    /// URL -> visit count
    visit_counts: HashMap<String, u32>,
    /// Maximum history size
    max_history: usize,
}

impl OmniboxPredictor {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: VecDeque::new(),
            visit_counts: HashMap::new(),
            max_history,
        }
    }

    /// Record a navigation
    pub fn record_navigation(&mut self, url: &str) {
        // Add to history
        self.history.push_front(url.to_string());
        if self.history.len() > self.max_history {
            self.history.pop_back();
        }

        // Increment visit count
        *self.visit_counts.entry(url.to_string()).or_insert(0) += 1;

        debug!("Recorded navigation: {}", url);
    }

    /// Predict URLs matching a prefix
    pub fn predict(&self, prefix: &str) -> Vec<(String, f32)> {
        let mut predictions: Vec<(String, f32)> = self
            .visit_counts
            .iter()
            .filter(|(url, _)| url.starts_with(prefix))
            .map(|(url, count)| {
                // Calculate score based on visit count and recency
                let recency_bonus = if let Some(pos) = self.history.iter().position(|u| u == url) {
                    1.0 / (pos as f32 + 1.0)
                } else {
                    0.0
                };

                let score = (*count as f32 / 100.0 + recency_bonus).min(1.0);
                (url.clone(), score)
            })
            .collect();

        predictions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        predictions
    }

    /// Get top N predictions
    pub fn top_predictions(&self, prefix: &str, n: usize) -> Vec<String> {
        self.predict(prefix)
            .into_iter()
            .take(n)
            .map(|(url, _)| url)
            .collect()
    }
}

impl Default for OmniboxPredictor {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Main speculation engine
pub struct SpeculationEngine {
    /// Active speculation rules
    rules: SpeculationRules,
    /// Hover tracker
    hover_tracker: HoverTracker,
    /// Omnibox predictor
    omnibox_predictor: OmniboxPredictor,
    /// Currently prefetched URLs
    prefetched: HashSet<String>,
    /// Currently prerendered URLs
    prerendered: HashSet<String>,
    /// Maximum prefetch queue size
    max_prefetch: usize,
    /// Maximum prerender queue size
    max_prerender: usize,
}

impl SpeculationEngine {
    /// Create a new speculation engine
    pub fn new() -> Self {
        Self {
            rules: SpeculationRules::empty(),
            hover_tracker: HoverTracker::new(),
            omnibox_predictor: OmniboxPredictor::new(1000),
            prefetched: HashSet::new(),
            prerendered: HashSet::new(),
            max_prefetch: 10,
            max_prerender: 1,
        }
    }

    /// Load speculation rules
    pub fn load_rules(&mut self, rules: SpeculationRules) {
        info!("Loaded {} speculation rules", rules.rules.len());
        self.rules = rules;
    }

    /// Parse and load rules from JSON
    pub fn load_rules_from_json(&mut self, json: &str) -> Result<(), String> {
        let rules = SpeculationRules::from_json(json)?;
        self.load_rules(rules);
        Ok(())
    }

    /// Record hover event
    pub fn on_hover_start(&mut self, url: &str) {
        self.hover_tracker.hover_start(url);
        self.evaluate_speculation(url);
    }

    /// Record hover end event
    pub fn on_hover_end(&mut self) {
        self.hover_tracker.hover_end();
    }

    /// Record navigation
    pub fn on_navigation(&mut self, url: &str) {
        self.omnibox_predictor.record_navigation(url);
    }

    /// Evaluate if URL should be speculated
    fn evaluate_speculation(&mut self, url: &str) {
        let actions: Vec<_> = self.rules.matching_rules(url)
            .into_iter()
            .map(|r| (r.action.clone(), r.min_probability))
            .collect();

        if actions.is_empty() {
            return;
        }

        let probability = self.hover_tracker.get_probability(url);

        for (action, min_probability) in actions {
            if probability >= min_probability {
                match action {
                    SpeculationAction::Prefetch => {
                        self.trigger_prefetch(url);
                    }
                    SpeculationAction::Prerender => {
                        self.trigger_prerender(url);
                    }
                }
            }
        }
    }

    /// Trigger prefetch for a URL
    fn trigger_prefetch(&mut self, url: &str) {
        if self.prefetched.contains(url) {
            return;
        }

        if self.prefetched.len() >= self.max_prefetch {
            // Remove oldest entry (simple FIFO)
            if let Some(oldest) = self.prefetched.iter().next().cloned() {
                self.prefetched.remove(&oldest);
                debug!("Evicted prefetch: {}", oldest);
            }
        }

        self.prefetched.insert(url.to_string());
        info!("Triggering prefetch: {}", url);
        
        // TODO: Actually trigger prefetch via ResourceLoader
    }

    /// Trigger prerender for a URL
    fn trigger_prerender(&mut self, url: &str) {
        if self.prerendered.contains(url) {
            return;
        }

        if self.prerendered.len() >= self.max_prerender {
            // Remove all prerendered (only keep one)
            self.prerendered.clear();
        }

        self.prerendered.insert(url.to_string());
        info!("Triggering prerender: {}", url);
        
        // TODO: Actually trigger prerender
    }

    /// Check if URL is already prefetched
    pub fn is_prefetched(&self, url: &str) -> bool {
        self.prefetched.contains(url)
    }

    /// Check if URL is already prerendered
    pub fn is_prerendered(&self, url: &str) -> bool {
        self.prerendered.contains(url)
    }

    /// Get omnibox predictions
    pub fn predict_omnibox(&self, prefix: &str, n: usize) -> Vec<String> {
        self.omnibox_predictor.top_predictions(prefix, n)
    }

    /// Clear all speculation state
    pub fn clear(&mut self) {
        self.prefetched.clear();
        self.prerendered.clear();
        info!("Cleared speculation state");
    }

    /// Get statistics
    pub fn stats(&self) -> SpeculationStats {
        SpeculationStats {
            rules_count: self.rules.rules.len(),
            prefetched_count: self.prefetched.len(),
            prerendered_count: self.prerendered.len(),
            history_size: self.omnibox_predictor.history.len(),
        }
    }
}

impl Default for SpeculationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Speculation engine statistics
#[derive(Debug, Clone)]
pub struct SpeculationStats {
    pub rules_count: usize,
    pub prefetched_count: usize,
    pub prerendered_count: usize,
    pub history_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_pattern_exact() {
        let pattern = UrlPattern::Exact {
            url: "https://example.com".to_string(),
        };
        assert!(pattern.matches("https://example.com"));
        assert!(!pattern.matches("https://example.com/page"));
    }

    #[test]
    fn test_url_pattern_prefix() {
        let pattern = UrlPattern::Prefix {
            prefix: "https://example.com".to_string(),
        };
        assert!(pattern.matches("https://example.com"));
        assert!(pattern.matches("https://example.com/page"));
        assert!(!pattern.matches("https://other.com"));
    }

    #[test]
    fn test_url_pattern_contains() {
        let pattern = UrlPattern::Contains {
            substring: "example".to_string(),
        };
        assert!(pattern.matches("https://example.com"));
        assert!(pattern.matches("https://test.example.org"));
        assert!(!pattern.matches("https://other.com"));
    }

    #[test]
    fn test_url_pattern_glob() {
        let pattern = UrlPattern::Glob {
            pattern: "https://*.example.com/*".to_string(),
        };
        assert!(pattern.matches("https://www.example.com/page"));
        assert!(pattern.matches("https://api.example.com/data"));
    }

    #[test]
    fn test_speculation_rule_matches() {
        let rule = SpeculationRule {
            action: SpeculationAction::Prefetch,
            patterns: vec![
                UrlPattern::Prefix {
                    prefix: "https://example.com".to_string(),
                },
            ],
            min_probability: 0.5,
            eagerness: "moderate".to_string(),
        };

        assert!(rule.matches("https://example.com/page"));
        assert!(!rule.matches("https://other.com"));
    }

    #[test]
    fn test_speculation_rules_from_json() {
        let json = r#"{
            "rules": [
                {
                    "action": "prefetch",
                    "patterns": [
                        {"type": "prefix", "prefix": "https://example.com"}
                    ],
                    "min_probability": 0.5,
                    "eagerness": "moderate"
                }
            ]
        }"#;

        let rules = SpeculationRules::from_json(json).unwrap();
        assert_eq!(rules.rules.len(), 1);
        assert_eq!(rules.rules[0].action, SpeculationAction::Prefetch);
    }

    #[test]
    fn test_hover_tracker() {
        let mut tracker = HoverTracker::new();
        
        tracker.hover_start("https://example.com");
        std::thread::sleep(Duration::from_millis(100));
        tracker.hover_end();

        let prob = tracker.get_probability("https://example.com");
        assert!(prob > 0.0);
    }

    #[test]
    fn test_omnibox_predictor() {
        let mut predictor = OmniboxPredictor::new(100);
        
        predictor.record_navigation("https://example.com/page1");
        predictor.record_navigation("https://example.com/page2");
        predictor.record_navigation("https://other.com");

        let predictions = predictor.predict("https://example.com");
        assert_eq!(predictions.len(), 2);
    }

    #[test]
    fn test_speculation_engine_prefetch() {
        let mut engine = SpeculationEngine::new();
        
        let mut rules = SpeculationRules::empty();
        rules.add_rule(SpeculationRule {
            action: SpeculationAction::Prefetch,
            patterns: vec![UrlPattern::Prefix {
                prefix: "https://example.com".to_string(),
            }],
            min_probability: 0.0,
            eagerness: "immediate".to_string(),
        });
        engine.load_rules(rules);

        engine.on_hover_start("https://example.com/page");
        engine.on_hover_end();

        assert!(engine.is_prefetched("https://example.com/page"));
    }

    #[test]
    fn test_speculation_engine_stats() {
        let mut engine = SpeculationEngine::new();
        engine.on_navigation("https://example.com");
        
        let stats = engine.stats();
        assert_eq!(stats.history_size, 1);
    }

    #[test]
    fn test_speculation_engine_clear() {
        let mut engine = SpeculationEngine::new();
        engine.trigger_prefetch("https://example.com");
        
        assert!(engine.is_prefetched("https://example.com"));
        
        engine.clear();
        assert!(!engine.is_prefetched("https://example.com"));
    }

    #[test]
    fn test_max_prefetch_limit() {
        let mut engine = SpeculationEngine::new();
        engine.max_prefetch = 2;

        engine.trigger_prefetch("url1");
        engine.trigger_prefetch("url2");
        engine.trigger_prefetch("url3");

        assert_eq!(engine.prefetched.len(), 2);
    }

    #[test]
    fn test_max_prerender_limit() {
        let mut engine = SpeculationEngine::new();
        engine.max_prerender = 1;

        engine.trigger_prerender("url1");
        engine.trigger_prerender("url2");

        assert_eq!(engine.prerendered.len(), 1);
        assert!(engine.is_prerendered("url2"));
    }
}
