//! Resource Loader
//!
//! HTTP resource loader with redirect handling, compression support,
//! and integration with connection manager and QUIC transport.

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use log::{debug, warn, error};
use url::Url;
use reqwest::{Client, Method};

/// HTTP methods
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
    PATCH,
}

impl HttpMethod {
    pub fn as_str(&self) -> &str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::HEAD => "HEAD",
            HttpMethod::OPTIONS => "OPTIONS",
            HttpMethod::PATCH => "PATCH",
        }
    }
}

/// Resource request
#[derive(Debug, Clone)]
pub struct ResourceRequest {
    /// Request URL
    pub url: String,
    /// HTTP method
    pub method: HttpMethod,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body (for POST, PUT, etc.)
    pub body: Option<Vec<u8>>,
    /// Request timeout
    pub timeout: Duration,
}

impl ResourceRequest {
    /// Create a GET request
    pub fn get(url: &str) -> Self {
        Self {
            url: url.to_string(),
            method: HttpMethod::GET,
            headers: HashMap::new(),
            body: None,
            timeout: Duration::from_secs(30),
        }
    }

    /// Create a POST request
    pub fn post(url: &str, body: Vec<u8>) -> Self {
        Self {
            url: url.to_string(),
            method: HttpMethod::POST,
            headers: HashMap::new(),
            body: Some(body),
            timeout: Duration::from_secs(30),
        }
    }

    /// Add a header
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

/// Resource response
#[derive(Debug, Clone)]
pub struct ResourceResponse {
    /// HTTP status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Vec<u8>,
    /// Final URL (after redirects)
    pub final_url: String,
    /// Request duration
    pub duration: Duration,
}

impl ResourceResponse {
    /// Check if response is successful (2xx status)
    pub fn is_success(&self) -> bool {
        self.status_code >= 200 && self.status_code < 300
    }

    /// Check if response is a redirect (3xx status)
    pub fn is_redirect(&self) -> bool {
        self.status_code >= 300 && self.status_code < 400
    }

    /// Get response body as UTF-8 string
    pub fn text(&self) -> Result<String, String> {
        String::from_utf8(self.body.clone())
            .map_err(|e| format!("Invalid UTF-8: {}", e))
    }

    /// Get Location header for redirects
    pub fn location(&self) -> Option<&String> {
        self.headers.get("location")
            .or_else(|| self.headers.get("Location"))
    }
}

/// Resource loader with connection management
pub struct ResourceLoader {
    /// User agent string
    user_agent: String,
    /// Maximum redirects to follow
    max_redirects: usize,
    /// Accept-Encoding header value
    accept_encoding: String,
    /// HTTP client
    client: Client,
}

impl ResourceLoader {
    /// Create a new resource loader
    pub fn new() -> Self {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Failed to create HTTP client");

        Self {
            user_agent: "Soliloquy/0.1.0 (Alpine; Servo)".to_string(),
            max_redirects: 10,
            accept_encoding: "gzip, deflate, br".to_string(),
            client,
        }
    }

    /// Set user agent
    pub fn with_user_agent(mut self, user_agent: &str) -> Self {
        self.user_agent = user_agent.to_string();
        self
    }

    /// Set maximum redirects
    pub fn with_max_redirects(mut self, max_redirects: usize) -> Self {
        self.max_redirects = max_redirects;
        self
    }

    /// Fetch a resource
    ///
    /// # Arguments
    /// * `request` - Resource request
    ///
    /// # Returns
    /// Result containing ResourceResponse or error message
    pub async fn fetch(&self, mut request: ResourceRequest) -> Result<ResourceResponse, String> {
        let start_time = SystemTime::now();
        let mut redirect_count = 0;

        // Add default headers
        if !request.headers.contains_key("user-agent") {
            request.headers.insert("user-agent".to_string(), self.user_agent.clone());
        }
        if !request.headers.contains_key("accept-encoding") {
            request.headers.insert("accept-encoding".to_string(), self.accept_encoding.clone());
        }

        let mut current_url = request.url.clone();

        loop {
            debug!("Fetching {} {}", request.method.as_str(), current_url);

            // Perform request
            let response = self.perform_request(&current_url, &request).await?;

            // Handle redirects
            if response.is_redirect() && redirect_count < self.max_redirects {
                if let Some(location) = response.location() {
                    debug!("Following redirect to {}", location);
                    
                    current_url = self.resolve_redirect_url(&current_url, location)?;
                    redirect_count += 1;
                    
                    // For 303 See Other, change method to GET
                    if response.status_code == 303 {
                        request.method = HttpMethod::GET;
                        request.body = None;
                    }
                    
                    continue;
                } else {
                    return Err(format!("Redirect without Location header (status {})", response.status_code));
                }
            }

            // Calculate duration
            let duration = SystemTime::now()
                .duration_since(start_time)
                .unwrap_or(Duration::from_secs(0));

            return Ok(ResourceResponse {
                status_code: response.status_code,
                headers: response.headers,
                body: response.body,
                final_url: current_url,
                duration,
            });
        }
    }

    /// Perform actual HTTP request
    async fn perform_request(
        &self,
        url: &str,
        request: &ResourceRequest,
    ) -> Result<ResourceResponse, String> {
        let req_method = match request.method {
            HttpMethod::GET => Method::GET,
            HttpMethod::POST => Method::POST,
            HttpMethod::PUT => Method::PUT,
            HttpMethod::DELETE => Method::DELETE,
            HttpMethod::HEAD => Method::HEAD,
            HttpMethod::OPTIONS => Method::OPTIONS,
            HttpMethod::PATCH => Method::PATCH,
        };

        debug!("Performing {} request to {}", request.method.as_str(), url);

        let mut req_builder = self.client.request(req_method, url);

        // Add headers
        for (key, value) in &request.headers {
            req_builder = req_builder.header(key, value);
        }

        // Add body if present
        if let Some(body) = &request.body {
            req_builder = req_builder.body(body.clone());
        }

        // Add timeout
        req_builder = req_builder.timeout(request.timeout);

        let start_time = SystemTime::now();

        // Send request
        let response = req_builder.send().await
            .map_err(|e| format!("Request failed: {}", e))?;

        let status_code = response.status().as_u16();
        let final_url = response.url().to_string();

        let headers: HashMap<String, String> = response.headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let body_bytes = response.bytes().await
            .map_err(|e| format!("Failed to read response body: {}", e))?
            .to_vec();

        let duration = SystemTime::now()
            .duration_since(start_time)
            .unwrap_or(Duration::from_secs(0));

        Ok(ResourceResponse {
            status_code,
            headers,
            body: body_bytes,
            final_url,
            duration,
        })
    }

    /// Resolve redirect URL (handle relative URLs)
    fn resolve_redirect_url(&self, base_url: &str, location: &str) -> Result<String, String> {
        // Parse base URL
        let base = Url::parse(base_url)
            .map_err(|e| format!("Invalid base URL: {}", e))?;

        // Parse location (might be relative)
        let resolved = if location.starts_with("http://") || location.starts_with("https://") {
            // Absolute URL
            Url::parse(location)
                .map_err(|e| format!("Invalid redirect URL: {}", e))?
        } else {
            // Relative URL
            base.join(location)
                .map_err(|e| format!("Failed to resolve redirect URL: {}", e))?
        };

        Ok(resolved.to_string())
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
    fn test_http_method_as_str() {
        assert_eq!(HttpMethod::GET.as_str(), "GET");
        assert_eq!(HttpMethod::POST.as_str(), "POST");
        assert_eq!(HttpMethod::PUT.as_str(), "PUT");
    }

    #[test]
    fn test_resource_request_get() {
        let request = ResourceRequest::get("https://example.com");
        assert_eq!(request.method, HttpMethod::GET);
        assert_eq!(request.url, "https://example.com");
        assert!(request.body.is_none());
    }

    #[test]
    fn test_resource_request_post() {
        let body = b"test data".to_vec();
        let request = ResourceRequest::post("https://example.com", body.clone());
        assert_eq!(request.method, HttpMethod::POST);
        assert_eq!(request.body, Some(body));
    }

    #[test]
    fn test_resource_request_with_header() {
        let request = ResourceRequest::get("https://example.com")
            .with_header("Authorization", "Bearer token");
        assert_eq!(request.headers.get("Authorization"), Some(&"Bearer token".to_string()));
    }

    #[test]
    fn test_resource_request_with_timeout() {
        let request = ResourceRequest::get("https://example.com")
            .with_timeout(Duration::from_secs(60));
        assert_eq!(request.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_resource_response_is_success() {
        let response = ResourceResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: vec![],
            final_url: "https://example.com".to_string(),
            duration: Duration::from_secs(1),
        };
        assert!(response.is_success());

        let response_404 = ResourceResponse {
            status_code: 404,
            headers: HashMap::new(),
            body: vec![],
            final_url: "https://example.com".to_string(),
            duration: Duration::from_secs(1),
        };
        assert!(!response_404.is_success());
    }

    #[test]
    fn test_resource_response_is_redirect() {
        let response = ResourceResponse {
            status_code: 301,
            headers: HashMap::new(),
            body: vec![],
            final_url: "https://example.com".to_string(),
            duration: Duration::from_secs(1),
        };
        assert!(response.is_redirect());
    }

    #[test]
    fn test_resource_response_text() {
        let response = ResourceResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: b"Hello, World!".to_vec(),
            final_url: "https://example.com".to_string(),
            duration: Duration::from_secs(1),
        };
        assert_eq!(response.text().unwrap(), "Hello, World!");
    }

    #[test]
    fn test_resource_response_location() {
        let mut headers = HashMap::new();
        headers.insert("Location".to_string(), "https://example.com/new".to_string());
        
        let response = ResourceResponse {
            status_code: 301,
            headers,
            body: vec![],
            final_url: "https://example.com".to_string(),
            duration: Duration::from_secs(1),
        };
        
        assert_eq!(response.location(), Some(&"https://example.com/new".to_string()));
    }

    #[tokio::test]
    async fn test_resource_loader_fetch() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0u8; 1024];
                let _ = stream.read(&mut buffer);
                let response = b"HTTP/1.1 200 OK\r\nContent-Length: 13\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nHello, world!";
                let _ = stream.write_all(response);
            }
        });

        let loader = ResourceLoader::new();
        let request = ResourceRequest::get(&format!("http://{}/", addr));
        let response = loader.fetch(request).await.unwrap();
        
        assert!(response.is_success());

        let _ = server.join();
    }

    #[test]
    fn test_resolve_redirect_url_absolute() {
        let loader = ResourceLoader::new();
        let result = loader.resolve_redirect_url(
            "https://example.com/path",
            "https://other.com/newpath"
        ).unwrap();
        
        assert_eq!(result, "https://other.com/newpath");
    }

    #[test]
    fn test_resolve_redirect_url_relative() {
        let loader = ResourceLoader::new();
        let result = loader.resolve_redirect_url(
            "https://example.com/path/page.html",
            "../other.html"
        ).unwrap();
        
        assert_eq!(result, "https://example.com/other.html");
    }

    #[test]
    fn test_resource_loader_with_user_agent() {
        let loader = ResourceLoader::new()
            .with_user_agent("Custom/1.0");
        assert_eq!(loader.user_agent, "Custom/1.0");
    }

    #[test]
    fn test_resource_loader_with_max_redirects() {
        let loader = ResourceLoader::new()
            .with_max_redirects(5);
        assert_eq!(loader.max_redirects, 5);
    }
}
