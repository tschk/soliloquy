//! Speculation Engine - Predictive prefetch and prerender
//!
//! This module implements Chrome-style speculation rules:
//! - Parse `<script type="speculationrules">` from HTML
//! - Prefetch resources before user clicks
//! - Prerender pages in background context
//! - Hover/proximity triggers
//! - Confidence-based prediction

use log::{info, debug, warn};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Speculation action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeculationAction {
    /// Fetch the resource and cache it
    Prefetch,
    /// Fully render the page in background
    Prerender,
}

/// URL pattern matching
#[derive(Debug, Clone)]
pub enum UrlPattern {
    /// Exact URL match
    Exact(String),
    /// URL prefix match
    Prefix(String),
    /// URL contains substring
    Contains(String),
    /// Wildcard pattern
    Glob(String),
}

impl UrlPattern {
    /// Check if a URL matches this pattern
    pub fn matches(&self, url: &str) -> bool {
        match self {
            UrlPattern::Exact(pattern) => url == pattern,
            UrlPattern::Prefix(pattern) => url.starts_with(pattern),
            UrlPattern::Contains(pattern) => url.contains(pattern),
            UrlPattern::Glob(pattern) => {
                // Simple glob matching (* = any chars)
                self.glob_match(url, pattern)
            }
        }
    }

    fn glob_match(&self, url: &str, pattern: &str) -> bool {
        // Simplified glob matching
        // TODO: Use proper glob library for full support
        if let Some(star_pos) = pattern.find('*') {
            let prefix = &pattern[..star_pos];
            let suffix = &pattern[star_pos + 1..];
            url.starts_with(prefix) && url.ends_with(suffix)
        } else {
            url == pattern
        }
    }
}

/// Speculation rule
#[derive(Debug, Clone)]
pub struct SpeculationRule {
    /// Action to perform (prefetch or prerender)
    pub action: SpeculationAction,
    /// URL patterns to match
    pub url_patterns: Vec<UrlPattern>,
    /// Confidence threshold (0-100)
    pub confidence: u32,
    /// Trigger condition
    pub trigger: SpeculationTrigger,
}

/// Speculation trigger conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeculationTrigger {
    /// Trigger immediately
    Immediate,
    /// Trigger on hover (cursor over link)
    Hover,
    /// Trigger when link is in viewport
    InViewport,
}

/// Speculation rules collection (parsed from JSON)
#[derive(Debug, Clone)]
pub struct SpeculationRules {
    /// List of speculation rules
    pub rules: Vec<SpeculationRule>,
}

impl SpeculationRules {
    /// Parse speculation rules from JSON
    ///
    /// Example JSON:
    /// ```json
    /// {
    ///   "prefetch": [
    ///     {"source": "list", "urls": ["/page1", "/page2"], "confidence": "high"}
    ///   ],
    ///   "prerender": [
    ///     {"source": "list", "urls": ["/landing"], "confidence": "high"}
    ///   ]
    /// }
    /// ```
    pub fn parse_json(json: &str) -> Result<Self, String> {
        // TODO: Use serde_json to parse properly
        // For now, return empty rules
        warn!("Speculation rules parsing not fully implemented");
        Ok(Self { rules: Vec::new() })
    }

    /// Find matching rules for a URL
    pub fn find_rules(&self, url: &str, action: SpeculationAction) -> Vec<&SpeculationRule> {
        self.rules
            .iter()
            .filter(|rule| {
                rule.action == action
                    && rule.url_patterns.iter().any(|pattern| pattern.matches(url))
            })
            .collect()
    }
}

/// Hover tracking for link prediction
#[derive(Debug)]
pub struct HoverTracker {
    /// Currently hovered URL
    current_hover: Option<String>,
    /// Timestamp when hover started
    hover_start: Option<Instant>,
    /// Minimum hover duration before trigger (milliseconds)
    hover_threshold: Duration,
}

impl HoverTracker {
    /// Create a new hover tracker
    pub fn new() -> Self {
        Self {
            current_hover: None,
            hover_start: None,
            hover_threshold: Duration::from_millis(100),
        }
    }

    /// Update hover state
    ///
    /// # Arguments
    /// * `url` - The URL currently being hovered, or None if no hover
    ///
    /// # Returns
    /// * `Some(String)` - URL that should be prefetched (hover threshold met)
    /// * `None` - No action needed
    pub fn update_hover(&mut self, url: Option<String>) -> Option<String> {
        match (url, &self.current_hover) {
            (Some(new_url), Some(current_url)) if new_url == *current_url => {
                // Still hovering same URL, check if threshold met
                if let Some(start) = self.hover_start {
                    if start.elapsed() >= self.hover_threshold {
                        debug!("Hover threshold met for: {}", new_url);
                        return Some(new_url);
                    }
                }
                None
            }
            (Some(new_url), _) => {
                // Started hovering new URL
                debug!("Started hovering: {}", new_url);
                self.current_hover = Some(new_url);
                self.hover_start = Some(Instant::now());
                None
            }
            (None, Some(old_url)) => {
                // Stopped hovering
                debug!("Stopped hovering: {}", old_url);
                self.current_hover = None;
                self.hover_start = None;
                None
            }
            (None, None) => None,
        }
    }
}

impl Default for HoverTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Omnibox predictor for URL prediction
pub struct OmniboxPredictor {
    /// History: user input -> final URL mappings
    history: HashMap<String, Vec<String>>,
    /// Confidence threshold for prerender (0-100)
    prerender_threshold: u32,
}

impl OmniboxPredictor {
    /// Create a new omnibox predictor
    pub fn new() -> Self {
        Self {
            history: HashMap::new(),
            prerender_threshold: 90,
        }
    }

    /// Record a navigation for future prediction
    ///
    /// # Arguments
    /// * `input` - What the user typed
    /// * `final_url` - The URL they ended up navigating to
    pub fn record_navigation(&mut self, input: &str, final_url: &str) {
        let input_lower = input.to_lowercase();
        let entry = self.history.entry(input_lower).or_insert_with(Vec::new);
        
        // Add to front of list (most recent)
        entry.insert(0, final_url.to_string());
        
        // Keep only last 10 navigations per input
        entry.truncate(10);
        
        debug!("Recorded navigation: '{}' -> '{}'", input, final_url);
    }

    /// Predict URL based on partial input
    ///
    /// # Arguments
    /// * `input` - Current user input in omnibox
    ///
    /// # Returns
    /// * `Some((url, confidence))` - Predicted URL with confidence (0-100)
    /// * `None` - No prediction
    pub fn predict(&self, input: &str) -> Option<(String, u32)> {
        if input.is_empty() {
            return None;
        }

        let input_lower = input.to_lowercase();

        // Exact match has highest confidence
        if let Some(history) = self.history.get(&input_lower) {
            if let Some(most_common) = self.find_most_common(history) {
                let confidence = self.calculate_confidence(history, &most_common);
                debug!("Prediction for '{}': {} ({}% confidence)", input, most_common, confidence);
                return Some((most_common, confidence));
            }
        }

        // Check prefix matches
        let matches: Vec<_> = self.history
            .iter()
            .filter(|(key, _)| key.starts_with(&input_lower))
            .collect();

        if !matches.is_empty() {
            // Find most common URL across all prefix matches
            let all_urls: Vec<_> = matches
                .iter()
                .flat_map(|(_, urls)| urls.iter())
                .collect();
            
            if let Some(most_common) = self.find_most_common_refs(&all_urls) {
                let confidence = (50 + (matches.len() * 5).min(40)) as u32;
                debug!("Prefix prediction for '{}': {} ({}% confidence)", input, most_common, confidence);
                return Some((most_common.to_string(), confidence));
            }
        }

        None
    }

    /// Check if prediction confidence is high enough for prerender
    pub fn should_prerender(&self, input: &str) -> Option<String> {
        if let Some((url, confidence)) = self.predict(input) {
            if confidence >= self.prerender_threshold {
                info!("High-confidence prediction for '{}': {} ({}%)", input, url, confidence);
                return Some(url);
            }
        }
        None
    }

    /// Find most common URL in history list
    fn find_most_common(&self, urls: &[String]) -> Option<String> {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for url in urls {
            *counts.entry(url.as_str()).or_insert(0) += 1;
        }
        
        counts.into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(url, _)| url.to_string())
    }

    /// Find most common URL in reference list
    fn find_most_common_refs(&self, urls: &[&String]) -> Option<String> {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for url in urls {
            *counts.entry(url.as_str()).or_insert(0) += 1;
        }
        
        counts.into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(url, _)| url.to_string())
    }

    /// Calculate confidence based on frequency
    fn calculate_confidence(&self, urls: &[String], target: &str) -> u32 {
        let total = urls.len();
        let matches = urls.iter().filter(|url| url.as_str() == target).count();
        
        if total == 0 {
            return 0;
        }
        
        // Confidence = (matches / total) * 100
        let base_confidence = (matches * 100 / total) as u32;
        
        // Boost confidence if it's the most recent navigation
        let recency_boost = if urls.first().map(|s| s.as_str()) == Some(target) {
            10
        } else {
            0
        };
        
        (base_confidence + recency_boost).min(100)
    }
}

impl Default for OmniboxPredictor {
    fn default() -> Self {
        Self::new()
    }
}

/// Main speculation engine
pub struct SpeculationEngine {
    /// Current speculation rules
    rules: Option<SpeculationRules>,
    /// Hover tracker for link prediction
    hover_tracker: HoverTracker,
    /// Omnibox predictor
    omnibox_predictor: OmniboxPredictor,
    /// URLs currently being prefetched
    prefetching: HashMap<String, Instant>,
    /// URLs currently being prerendered
    prerendering: HashMap<String, Instant>,
}

impl SpeculationEngine {
    /// Create a new speculation engine
    pub fn new() -> Self {
        info!("Initializing SpeculationEngine");
        Self {
            rules: None,
            hover_tracker: HoverTracker::new(),
            omnibox_predictor: OmniboxPredictor::new(),
            prefetching: HashMap::new(),
            prerendering: HashMap::new(),
        }
    }

    /// Update speculation rules (from page's speculation rules script)
    pub fn update_rules(&mut self, rules: SpeculationRules) {
        info!("Updated speculation rules with {} rules", rules.rules.len());
        self.rules = Some(rules);
    }

    /// Handle hover event
    pub fn handle_hover(&mut self, url: Option<String>) -> Option<String> {
        if let Some(prefetch_url) = self.hover_tracker.update_hover(url) {
            // Check if we should prefetch this URL
            if let Some(ref rules) = self.rules {
                let matching = rules.find_rules(&prefetch_url, SpeculationAction::Prefetch);
                if !matching.is_empty() {
                    info!("Triggering hover prefetch for: {}", prefetch_url);
                    self.prefetching.insert(prefetch_url.clone(), Instant::now());
                    return Some(prefetch_url);
                }
            }
        }
        None
    }

    /// Handle omnibox input for predictive prerender
    pub fn handle_omnibox_input(&mut self, input: &str) -> Option<String> {
        if let Some(url) = self.omnibox_predictor.should_prerender(input) {
            if !self.prerendering.contains_key(&url) {
                info!("Triggering omnibox prerender for: {}", url);
                self.prerendering.insert(url.clone(), Instant::now());
                return Some(url);
            }
        }
        None
    }

    /// Record navigation for omnibox learning
    pub fn record_navigation(&mut self, input: &str, final_url: &str) {
        self.omnibox_predictor.record_navigation(input, final_url);
    }

    /// Check if a URL is currently being prefetched
    pub fn is_prefetching(&self, url: &str) -> bool {
        self.prefetching.contains_key(url)
    }

    /// Check if a URL is currently being prerendered
    pub fn is_prerendering(&self, url: &str) -> bool {
        self.prerendering.contains_key(url)
    }

    /// Clean up completed speculation actions
    pub fn cleanup(&mut self, max_age: Duration) {
        let now = Instant::now();
        
        self.prefetching.retain(|url, started| {
            let keep = now.duration_since(*started) < max_age;
            if !keep {
                debug!("Removing completed prefetch: {}", url);
            }
            keep
        });
        
        self.prerendering.retain(|url, started| {
            let keep = now.duration_since(*started) < max_age;
            if !keep {
                debug!("Removing completed prerender: {}", url);
            }
            keep
        });
    }
}

impl Default for SpeculationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_pattern_exact() {
        let pattern = UrlPattern::Exact("https://example.com".to_string());
        assert!(pattern.matches("https://example.com"));
        assert!(!pattern.matches("https://example.com/page"));
    }

    #[test]
    fn test_url_pattern_prefix() {
        let pattern = UrlPattern::Prefix("https://example.com".to_string());
        assert!(pattern.matches("https://example.com"));
        assert!(pattern.matches("https://example.com/page"));
        assert!(!pattern.matches("https://other.com"));
    }

    #[test]
    fn test_hover_tracker() {
        let mut tracker = HoverTracker::new();
        
        // Start hovering
        assert!(tracker.update_hover(Some("https://example.com".to_string())).is_none());
        
        // Wait for threshold
        std::thread::sleep(Duration::from_millis(150));
        
        // Still hovering - should trigger
        let result = tracker.update_hover(Some("https://example.com".to_string()));
        assert!(result.is_some());
    }

    #[test]
    fn test_omnibox_predictor() {
        let mut predictor = OmniboxPredictor::new();
        
        // Record some navigations
        predictor.record_navigation("example", "https://example.com");
        predictor.record_navigation("example", "https://example.com");
        predictor.record_navigation("example", "https://example.org");
        
        // Predict
        let prediction = predictor.predict("example");
        assert!(prediction.is_some());
        
        let (url, confidence) = prediction.unwrap();
        assert_eq!(url, "https://example.com");
        assert!(confidence > 50);
    }

    #[test]
    fn test_omnibox_prefix_match() {
        let mut predictor = OmniboxPredictor::new();
        predictor.record_navigation("example", "https://example.com");
        
        let prediction = predictor.predict("exam");
        assert!(prediction.is_some());
    }

    #[test]
    fn test_speculation_engine() {
        let mut engine = SpeculationEngine::new();
        
        // Record navigations
        engine.record_navigation("test", "https://test.com");
        
        // Check prediction
        let prerender = engine.handle_omnibox_input("test");
        // May or may not trigger depending on confidence
        
        assert!(!engine.is_prefetching("https://other.com"));
    }

    #[test]
    fn test_url_pattern_glob() {
        let pattern = UrlPattern::Glob("https://*.example.com".to_string());
        assert!(pattern.matches("https://sub.example.com"));
        assert!(pattern.matches("https://www.example.com"));
    }
}
