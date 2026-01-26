//! Resource Loader - Fetch resources with redirect handling
//!
//! This module provides HTTP resource loading:
//! - Fetch HTML, CSS, JavaScript, images, etc.
//! - Handle HTTP redirects (301, 302, 307, 308)
//! - Content negotiation and compression
//! - Cache-Control header handling

use log::{info, debug, warn};
use std::collections::HashMap;
use std::time::Instant;

/// HTTP method
#[derive(Debug, Clone, PartialEq)]
pub enum HttpMethod {
    GET,
    POST,
    HEAD,
    PUT,
    DELETE,
}

/// Resource request
#[derive(Debug, Clone)]
pub struct ResourceRequest {
    /// Request URL
    pub url: String,
    /// HTTP method
    pub method: HttpMethod,
    /// HTTP headers
    pub headers: HashMap<String, String>,
    /// Request body (for POST, PUT)
    pub body: Option<Vec<u8>>,
    /// Request priority (higher = more important)
    pub priority: u32,
}

impl ResourceRequest {
    /// Create a new GET request
    pub fn get(url: &str) -> Self {
        Self {
            url: url.to_string(),
            method: HttpMethod::GET,
            headers: HashMap::new(),
            body: None,
            priority: 50, // Medium priority
        }
    }

    /// Add a header to the request
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Set request priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }
}

/// Resource response
#[derive(Debug, Clone)]
pub struct ResourceResponse {
    /// Response URL (may differ from request due to redirects)
    pub url: String,
    /// HTTP status code
    pub status: u16,
    /// HTTP headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Vec<u8>,
    /// Time taken to fetch
    pub duration: std::time::Duration,
}

impl ResourceResponse {
    /// Get a header value
    pub fn get_header(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }

    /// Check if response is successful (2xx)
    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    /// Check if response is a redirect (3xx)
    pub fn is_redirect(&self) -> bool {
        self.status >= 300 && self.status < 400
    }

    /// Get redirect location
    pub fn redirect_location(&self) -> Option<&String> {
        self.get_header("location")
    }

    /// Get response body as UTF-8 string
    pub fn body_as_string(&self) -> Result<String, String> {
        String::from_utf8(self.body.clone())
            .map_err(|e| format!("Invalid UTF-8: {}", e))
    }
}

/// Resource loader for fetching web resources
pub struct ResourceLoader {
    /// User-Agent header
    user_agent: String,
    /// Maximum number of redirects to follow
    max_redirects: u32,
    /// Accept-Encoding header (compression support)
    accept_encoding: String,
}

impl ResourceLoader {
    /// Create a new resource loader
    pub fn new() -> Self {
        Self {
            user_agent: "Soliloquy/0.1 (Servo; Zircon)".to_string(),
            max_redirects: 10,
            accept_encoding: "gzip, deflate, br".to_string(),
        }
    }

    /// Fetch a resource with redirect handling
    ///
    /// # Arguments
    /// * `request` - The resource request
    ///
    /// # Returns
    /// * `Ok(ResourceResponse)` - Successful response
    /// * `Err(String)` - Fetch failure
    pub fn fetch(&self, mut request: ResourceRequest) -> Result<ResourceResponse, String> {
        let start = Instant::now();
        info!("Fetching resource: {}", request.url);

        // Add default headers
        if !request.headers.contains_key("User-Agent") {
            request.headers.insert("User-Agent".to_string(), self.user_agent.clone());
        }
        if !request.headers.contains_key("Accept-Encoding") {
            request.headers.insert("Accept-Encoding".to_string(), self.accept_encoding.clone());
        }

        // Follow redirects
        let mut redirect_count = 0;
        let mut current_url = request.url.clone();

        loop {
            debug!("Requesting: {}", current_url);

            // TODO: Actually perform HTTP request
            // This would use hyper, reqwest, or curl binding
            // For now, return placeholder response
            
            let response = self.perform_request(&current_url, &request)?;

            if response.is_redirect() {
                if redirect_count >= self.max_redirects {
                    return Err(format!("Too many redirects ({})", redirect_count));
                }

                if let Some(location) = response.redirect_location() {
                    info!("Following redirect to: {}", location);
                    current_url = self.resolve_redirect_url(&current_url, location)?;
                    redirect_count += 1;
                    continue;
                } else {
                    return Err("Redirect response missing Location header".to_string());
                }
            }

            if response.is_success() {
                let duration = start.elapsed();
                info!("Resource fetched successfully in {:?}: {}", duration, current_url);
                return Ok(ResourceResponse {
                    url: current_url,
                    status: response.status,
                    headers: response.headers,
                    body: response.body,
                    duration,
                });
            }

            return Err(format!("HTTP error {}: {}", response.status, current_url));
        }
    }

    /// Perform the actual HTTP request (placeholder)
    fn perform_request(
        &self,
        url: &str,
        request: &ResourceRequest,
    ) -> Result<ResourceResponse, String> {
        // TODO: Implement actual HTTP request
        // This is a placeholder that returns fake responses

        warn!("HTTP request not fully implemented, returning placeholder");

        let body = format!("<html><head><title>Placeholder</title></head><body><h1>Placeholder for {}</h1></body></html>", url);
        
        Ok(ResourceResponse {
            url: url.to_string(),
            status: 200,
            headers: {
                let mut headers = HashMap::new();
                headers.insert("content-type".to_string(), "text/html".to_string());
                headers.insert("content-length".to_string(), body.len().to_string());
                headers
            },
            body: body.into_bytes(),
            duration: std::time::Duration::from_millis(10),
        })
    }

    /// Resolve a redirect URL (handle relative URLs)
    fn resolve_redirect_url(&self, base: &str, location: &str) -> Result<String, String> {
        // If location is absolute, use it directly
        if location.starts_with("http://") || location.starts_with("https://") {
            return Ok(location.to_string());
        }

        // Handle relative URLs
        // TODO: Use proper URL parsing library (url crate)
        if location.starts_with('/') {
            // Absolute path
            if let Some(origin_end) = base.find("://") {
                if let Some(path_start) = base[origin_end + 3..].find('/') {
                    let origin = &base[..origin_end + 3 + path_start];
                    return Ok(format!("{}{}", origin, location));
                } else {
                    return Ok(format!("{}{}", base, location));
                }
            }
        }

        // Relative path - resolve against base
        if let Some(last_slash) = base.rfind('/') {
            let base_dir = &base[..last_slash + 1];
            return Ok(format!("{}{}", base_dir, location));
        }

        Err(format!("Failed to resolve redirect URL: {}", location))
    }
}

impl Default for ResourceLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_request_creation() {
        let request = ResourceRequest::get("https://example.com");
        assert_eq!(request.url, "https://example.com");
        assert_eq!(request.method, HttpMethod::GET);
        assert_eq!(request.priority, 50);
    }

    #[test]
    fn test_resource_request_with_header() {
        let request = ResourceRequest::get("https://example.com")
            .with_header("Accept", "text/html");
        assert_eq!(request.headers.get("Accept").unwrap(), "text/html");
    }

    #[test]
    fn test_resource_request_with_priority() {
        let request = ResourceRequest::get("https://example.com")
            .with_priority(100);
        assert_eq!(request.priority, 100);
    }

    #[test]
    fn test_resource_response_is_success() {
        let response = ResourceResponse {
            url: "https://example.com".to_string(),
            status: 200,
            headers: HashMap::new(),
            body: vec![],
            duration: std::time::Duration::from_secs(1),
        };
        assert!(response.is_success());
        assert!(!response.is_redirect());
    }

    #[test]
    fn test_resource_response_is_redirect() {
        let mut headers = HashMap::new();
        headers.insert("location".to_string(), "https://example.org".to_string());
        
        let response = ResourceResponse {
            url: "https://example.com".to_string(),
            status: 301,
            headers,
            body: vec![],
            duration: std::time::Duration::from_secs(1),
        };
        assert!(response.is_redirect());
        assert!(!response.is_success());
        assert_eq!(response.redirect_location().unwrap(), "https://example.org");
    }

    #[test]
    fn test_resource_loader_creation() {
        let loader = ResourceLoader::new();
        assert_eq!(loader.max_redirects, 10);
        assert!(loader.user_agent.contains("Soliloquy"));
    }

    #[test]
    fn test_resource_loader_fetch() {
        let loader = ResourceLoader::new();
        let request = ResourceRequest::get("https://example.com");
        let result = loader.fetch(request);
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert_eq!(response.status, 200);
        assert!(!response.body.is_empty());
    }

    #[test]
    fn test_resolve_redirect_absolute_url() {
        let loader = ResourceLoader::new();
        let result = loader.resolve_redirect_url(
            "https://example.com/page",
            "https://example.org/other"
        );
        assert_eq!(result.unwrap(), "https://example.org/other");
    }

    #[test]
    fn test_resolve_redirect_absolute_path() {
        let loader = ResourceLoader::new();
        let result = loader.resolve_redirect_url(
            "https://example.com/old/page",
            "/new/page"
        );
        assert_eq!(result.unwrap(), "https://example.com/new/page");
    }

    #[test]
    fn test_response_body_as_string() {
        let response = ResourceResponse {
            url: "https://example.com".to_string(),
            status: 200,
            headers: HashMap::new(),
            body: "Hello, World!".as_bytes().to_vec(),
            duration: std::time::Duration::from_secs(1),
        };
        assert_eq!(response.body_as_string().unwrap(), "Hello, World!");
    }
}
