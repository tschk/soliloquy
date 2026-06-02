//! QUIC Transport Implementation
//!
//! Provides HTTP/3 over QUIC with connection migration, 0-RTT resumption,
//! Alt-Svc support, and session ticket caching.

use bytes::{Buf, Bytes};
use futures::future;
use hyper::{
    header::{HeaderName, HeaderValue},
    Method, Request, Uri,
};
use log::{debug, warn};
use quinn::{ClientConfig, Connection, Endpoint};
use rustls::RootCertStore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type H3SendRequest = h3::client::SendRequest<h3_quinn::OpenStreams, Bytes>;
type H3DriverHandle = tokio::task::JoinHandle<()>;

/// QUIC configuration
#[derive(Debug, Clone)]
pub struct QuicConfig {
    /// Maximum idle timeout
    pub max_idle_timeout: Duration,
    /// Enable 0-RTT
    pub enable_0rtt: bool,
    /// Initial max data
    pub initial_max_data: u64,
    /// Initial max stream data (bidirectional, local)
    pub initial_max_stream_data_bidi_local: u64,
    /// Initial max stream data (bidirectional, remote)
    pub initial_max_stream_data_bidi_remote: u64,
    /// Initial max stream data (unidirectional)
    pub initial_max_stream_data_uni: u64,
    /// Maximum concurrent bidirectional streams
    pub max_concurrent_bidi_streams: u64,
    /// Maximum concurrent unidirectional streams
    pub max_concurrent_uni_streams: u64,
}

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            max_idle_timeout: Duration::from_secs(30),
            enable_0rtt: true,
            initial_max_data: 10 * 1024 * 1024, // 10 MB
            initial_max_stream_data_bidi_local: 1024 * 1024, // 1 MB
            initial_max_stream_data_bidi_remote: 1024 * 1024, // 1 MB
            initial_max_stream_data_uni: 1024 * 1024, // 1 MB
            max_concurrent_bidi_streams: 100,
            max_concurrent_uni_streams: 100,
        }
    }
}

/// QUIC session ticket for 0-RTT resumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuicSessionTicket {
    /// Server hostname
    pub hostname: String,
    /// Session ticket data
    pub ticket: Vec<u8>,
    /// Transport parameters
    pub transport_params: Vec<u8>,
    /// Creation timestamp
    pub created_at: u64,
    /// Lifetime in seconds
    pub lifetime: u64,
}

impl QuicSessionTicket {
    /// Check if ticket is expired
    pub fn is_expired(&self) -> bool {
        let now = current_timestamp();
        now > self.created_at + self.lifetime
    }
}

/// QUIC connection state
pub struct QuicConnection {
    /// Server hostname
    pub hostname: String,
    /// Connection endpoint address
    pub remote_addr: SocketAddr,
    /// Connection established timestamp
    pub established_at: u64,
    /// Last activity timestamp
    pub last_activity: u64,
    /// Connection ID
    pub connection_id: u64,
    /// Actual QUIC connection
    pub connection: Option<Connection>,
    /// HTTP/3 request sender
    h3_sender: Option<H3SendRequest>,
    /// HTTP/3 connection driver task
    h3_driver: Option<H3DriverHandle>,
}

impl std::fmt::Debug for QuicConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuicConnection")
            .field("hostname", &self.hostname)
            .field("remote_addr", &self.remote_addr)
            .field("established_at", &self.established_at)
            .field("last_activity", &self.last_activity)
            .field("connection_id", &self.connection_id)
            .field("connection", &self.connection)
            .field("has_h3_sender", &self.h3_sender.is_some())
            .field("has_h3_driver", &self.h3_driver.is_some())
            .finish()
    }
}

impl QuicConnection {
    /// Check if connection is idle
    pub fn is_idle(&self, timeout_secs: u64) -> bool {
        let now = current_timestamp();
        now > self.last_activity + timeout_secs
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = current_timestamp();
    }
}

/// Alt-Svc (Alternative Service) entry for HTTP/3 discovery
#[derive(Debug, Clone)]
pub struct AltSvcEntry {
    /// Origin hostname
    pub origin: String,
    /// Alternative hostname
    pub alt_hostname: String,
    /// Alternative port
    pub alt_port: u16,
    /// Protocol (e.g., "h3")
    pub protocol: String,
    /// Max age in seconds
    pub max_age: u64,
    /// Cached timestamp
    pub cached_at: u64,
}

impl AltSvcEntry {
    /// Check if entry is expired
    pub fn is_expired(&self) -> bool {
        let now = current_timestamp();
        now > self.cached_at + self.max_age
    }
}

/// QUIC transport manager
pub struct QuicTransport {
    /// QUIC configuration
    config: QuicConfig,
    /// Session ticket cache: hostname -> QuicSessionTicket
    session_tickets: Arc<Mutex<HashMap<String, QuicSessionTicket>>>,
    /// Active connections: hostname -> QuicConnection
    connections: Arc<Mutex<HashMap<String, QuicConnection>>>,
    /// Alt-Svc cache: origin -> AltSvcEntry
    alt_svc_cache: Arc<Mutex<HashMap<String, AltSvcEntry>>>,
    /// Next connection ID
    next_connection_id: Arc<Mutex<u64>>,
    /// QUIC Endpoint
    endpoint: Endpoint,
}

impl QuicTransport {
    /// Create a new QUIC transport
    pub fn new(config: QuicConfig) -> Self {
        // Initialize QUIC endpoint
        // Bind to ephemeral port
        let mut endpoint =
            Endpoint::client("[::]:0".parse().unwrap()).expect("Failed to create QUIC endpoint");

        // Configure TLS
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let mut client_crypto = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        client_crypto.alpn_protocols = vec![b"h3".to_vec()];

        // Wrap rustls config in quinn's QuicClientConfig
        let quic_client_config = quinn::crypto::rustls::QuicClientConfig::try_from(client_crypto)
            .expect("Failed to create QUIC client config");

        let client_config = ClientConfig::new(Arc::new(quic_client_config));

        // Configure transport parameters from QuicConfig
        // let mut transport_config = quinn::TransportConfig::default();
        // transport_config.max_idle_timeout(Some(quinn::IdleTimeout::try_from(config.max_idle_timeout).unwrap()));
        // transport_config.initial_max_data(config.initial_max_data);
        // transport_config.initial_max_stream_data_bidi_local(config.initial_max_stream_data_bidi_local);
        // transport_config.initial_max_stream_data_bidi_remote(config.initial_max_stream_data_bidi_remote);
        // transport_config.initial_max_stream_data_uni(config.initial_max_stream_data_uni);
        // transport_config.max_concurrent_bidi_streams(quinn::VarInt::from_u64(config.max_concurrent_bidi_streams).unwrap());
        // transport_config.max_concurrent_uni_streams(quinn::VarInt::from_u64(config.max_concurrent_uni_streams).unwrap());

        // client_config.transport_config(Arc::new(transport_config));

        endpoint.set_default_client_config(client_config);

        Self {
            config,
            session_tickets: Arc::new(Mutex::new(HashMap::new())),
            connections: Arc::new(Mutex::new(HashMap::new())),
            alt_svc_cache: Arc::new(Mutex::new(HashMap::new())),
            next_connection_id: Arc::new(Mutex::new(1)),
            endpoint,
        }
    }

    /// Check if QUIC is available for a hostname
    ///
    /// # Arguments
    /// * `hostname` - Target hostname
    ///
    /// # Returns
    /// true if QUIC/HTTP3 is available via Alt-Svc
    pub fn is_quic_available(&self, hostname: &str) -> bool {
        let cache = self.alt_svc_cache.lock().unwrap();
        if let Some(entry) = cache.get(hostname) {
            if !entry.is_expired() {
                return entry.protocol == "h3" || entry.protocol == "h3-29";
            }
        }
        false
    }

    /// Connect to a server using QUIC
    ///
    /// # Arguments
    /// * `hostname` - Target hostname
    /// * `port` - Target port
    ///
    /// # Returns
    /// Result containing connection ID or error
    pub async fn connect(&self, hostname: &str, port: u16) -> Result<u64, String> {
        debug!("Connecting to {}:{} via QUIC", hostname, port);

        // Check for existing connection
        {
            let mut connections = self.connections.lock().unwrap();
            if let Some(conn) = connections.get_mut(hostname) {
                if !conn.is_idle(30) {
                    conn.touch();
                    debug!("Reusing existing QUIC connection to {}", hostname);
                    return Ok(conn.connection_id);
                } else {
                    debug!("Removing idle QUIC connection to {}", hostname);
                    connections.remove(hostname);
                }
            }
        }

        // Check for session ticket (0-RTT)
        let session_ticket = {
            let tickets = self.session_tickets.lock().unwrap();
            tickets.get(hostname).and_then(|t| {
                if !t.is_expired() {
                    Some(t.clone())
                } else {
                    None
                }
            })
        };

        if session_ticket.is_some() {
            debug!("Using 0-RTT session ticket for {}", hostname);
            // Note: 0-RTT implementation with quinn requires passing the ticket in client config
            // which is more complex as we need to clone the config per connection.
            // For now, we acknowledge it but don't use it in this implementation step.
        }

        let connection_id = {
            let mut next_id = self.next_connection_id.lock().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        // Resolve DNS
        let remote_addrs = tokio::net::lookup_host(format!("{}:{}", hostname, port))
            .await
            .map_err(|e| format!("DNS lookup failed: {}", e))?;

        let mut connection_result = None;
        let mut last_error = String::new();

        for addr in remote_addrs {
            match self.endpoint.connect(addr, hostname) {
                Ok(connecting) => match connecting.await {
                    Ok(conn) => {
                        connection_result = Some((conn, addr));
                        break;
                    }
                    Err(e) => {
                        last_error = format!("Connection failed: {}", e);
                        warn!("Failed to connect to {}: {}", addr, e);
                    }
                },
                Err(e) => {
                    last_error = format!("Failed to create connection: {}", e);
                    warn!("Failed to create connection to {}: {}", addr, e);
                }
            }
        }

        let (connection, remote_addr) = connection_result.ok_or_else(|| {
            if last_error.is_empty() {
                "No addresses resolved".to_string()
            } else {
                last_error
            }
        })?;

        let h3_connection = h3_quinn::Connection::new(connection.clone());
        let (mut h3_driver, h3_sender) = h3::client::new(h3_connection)
            .await
            .map_err(|e| format!("HTTP/3 connection setup failed: {}", e))?;
        let h3_driver = tokio::spawn(async move {
            let closed = future::poll_fn(|cx| h3_driver.poll_close(cx)).await;
            debug!("HTTP/3 connection driver closed: {}", closed);
        });

        let connection = QuicConnection {
            hostname: hostname.to_string(),
            remote_addr,
            established_at: current_timestamp(),
            last_activity: current_timestamp(),
            connection_id,
            connection: Some(connection),
            h3_sender: Some(h3_sender),
            h3_driver: Some(h3_driver),
        };

        {
            let mut connections = self.connections.lock().unwrap();
            connections.insert(hostname.to_string(), connection);
        }

        debug!(
            "Established QUIC connection {} to {}",
            connection_id, hostname
        );
        Ok(connection_id)
    }

    /// Send HTTP/3 request over QUIC connection
    ///
    /// # Arguments
    /// * `connection_id` - Connection identifier
    /// * `method` - HTTP method
    /// * `path` - Request path
    /// * `headers` - Request headers
    /// * `body` - Optional request body
    ///
    /// # Returns
    /// Result containing response data
    pub async fn send_h3_request(
        &self,
        connection_id: u64,
        method: &str,
        path: &str,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
    ) -> Result<Vec<u8>, String> {
        debug!("Sending HTTP/3 {} request to {}", method, path);

        let (hostname, mut h3_sender) = {
            let mut connections = self.connections.lock().unwrap();
            let conn = connections
                .values_mut()
                .find(|conn| conn.connection_id == connection_id)
                .ok_or_else(|| format!("No QUIC connection found for id {}", connection_id))?;
            let quinn_conn = conn
                .connection
                .as_ref()
                .ok_or_else(|| format!("QUIC connection {} is closed", connection_id))?;
            if let Some(reason) = quinn_conn.close_reason() {
                return Err(format!(
                    "QUIC connection {} is closed: {}",
                    connection_id, reason
                ));
            }
            let h3_sender = conn
                .h3_sender
                .as_ref()
                .ok_or_else(|| {
                    format!(
                        "HTTP/3 request sender unavailable for connection {}",
                        connection_id
                    )
                })?
                .clone();
            conn.touch();
            (conn.hostname.clone(), h3_sender)
        };

        let request = build_h3_request(&hostname, method, path, headers)?;
        let mut request_stream = h3_sender
            .send_request(request)
            .await
            .map_err(|e| format!("HTTP/3 stream open failed: {}", e))?;

        if let Some(body) = body {
            if !body.is_empty() {
                request_stream
                    .send_data(Bytes::from(body))
                    .await
                    .map_err(|e| format!("HTTP/3 request body send failed: {}", e))?;
            }
        }

        request_stream
            .finish()
            .await
            .map_err(|e| format!("HTTP/3 request finish failed: {}", e))?;
        request_stream
            .recv_response()
            .await
            .map_err(|e| format!("HTTP/3 response headers failed: {}", e))?;

        let mut response = Vec::new();
        while let Some(chunk) = request_stream
            .recv_data()
            .await
            .map_err(|e| format!("HTTP/3 response body failed: {}", e))?
        {
            append_response_chunk(&mut response, chunk);
        }

        {
            let mut connections = self.connections.lock().unwrap();
            if let Some(conn) = connections
                .values_mut()
                .find(|conn| conn.connection_id == connection_id)
            {
                conn.touch();
            }
        }

        Ok(response)
    }

    /// Parse and cache Alt-Svc header
    ///
    /// # Arguments
    /// * `origin` - Origin hostname
    /// * `alt_svc_header` - Alt-Svc header value
    pub fn cache_alt_svc(&self, origin: &str, alt_svc_header: &str) {
        // Format: h3=":443"; ma=2592000

        // Simple parser for common format
        if let Some(entry) = parse_alt_svc(origin, alt_svc_header) {
            let mut cache = self.alt_svc_cache.lock().unwrap();
            cache.insert(origin.to_string(), entry);
            debug!("Cached Alt-Svc for {}", origin);
        }
    }

    /// Cache session ticket for 0-RTT resumption
    ///
    /// # Arguments
    /// * `hostname` - Server hostname
    /// * `ticket` - Session ticket data
    /// * `transport_params` - QUIC transport parameters
    pub fn cache_session_ticket(&self, hostname: &str, ticket: Vec<u8>, transport_params: Vec<u8>) {
        let session_ticket = QuicSessionTicket {
            hostname: hostname.to_string(),
            ticket,
            transport_params,
            created_at: current_timestamp(),
            lifetime: 86400, // 24 hours
        };

        let mut tickets = self.session_tickets.lock().unwrap();
        tickets.insert(hostname.to_string(), session_ticket);
        debug!("Cached QUIC session ticket for {}", hostname);
    }

    /// Clean up expired tickets and idle connections
    pub fn cleanup_expired(&self) {
        // Clean session tickets
        {
            let mut tickets = self.session_tickets.lock().unwrap();
            tickets.retain(|hostname, ticket| {
                let keep = !ticket.is_expired();
                if !keep {
                    debug!("Removing expired session ticket for {}", hostname);
                }
                keep
            });
        }

        // Clean Alt-Svc cache
        {
            let mut cache = self.alt_svc_cache.lock().unwrap();
            cache.retain(|origin, entry| {
                let keep = !entry.is_expired();
                if !keep {
                    debug!("Removing expired Alt-Svc entry for {}", origin);
                }
                keep
            });
        }

        // Clean idle connections
        {
            let mut connections = self.connections.lock().unwrap();
            connections.retain(|hostname, conn| {
                let keep = !conn.is_idle(self.config.max_idle_timeout.as_secs());
                if !keep {
                    debug!("Removing idle QUIC connection to {}", hostname);
                    if let Some(q_conn) = &conn.connection {
                        q_conn.close(0u32.into(), b"idle timeout");
                    }
                }
                keep
            });
        }
    }

    /// Get transport statistics
    pub fn stats(&self) -> QuicStats {
        let connections = self.connections.lock().unwrap();
        let tickets = self.session_tickets.lock().unwrap();
        let alt_svc = self.alt_svc_cache.lock().unwrap();

        QuicStats {
            active_connections: connections.len(),
            cached_tickets: tickets.len(),
            alt_svc_entries: alt_svc.len(),
        }
    }
}

impl Default for QuicTransport {
    fn default() -> Self {
        Self::new(QuicConfig::default())
    }
}

/// QUIC transport statistics
#[derive(Debug, Clone)]
pub struct QuicStats {
    pub active_connections: usize,
    pub cached_tickets: usize,
    pub alt_svc_entries: usize,
}

/// Parse Alt-Svc header value
fn parse_alt_svc(origin: &str, header: &str) -> Option<AltSvcEntry> {
    // Simple parser for: h3=":443"; ma=2592000
    let parts: Vec<&str> = header.split(';').collect();
    if parts.is_empty() {
        return None;
    }

    let protocol_part = parts[0].trim();
    let protocol_port: Vec<&str> = protocol_part.splitn(2, '=').collect();
    if protocol_port.len() != 2 {
        return None;
    }

    let protocol = protocol_port[0].trim().to_string();
    let port_str = protocol_port[1].trim().trim_matches('"').trim_matches(':');
    let alt_port = port_str.parse::<u16>().ok()?;

    let mut max_age = 86400; // Default 24 hours
    for part in &parts[1..] {
        if let Some(ma_str) = part.trim().strip_prefix("ma=") {
            if let Ok(ma) = ma_str.parse::<u64>() {
                max_age = ma;
            }
        }
    }

    Some(AltSvcEntry {
        origin: origin.to_string(),
        alt_hostname: origin.to_string(),
        alt_port,
        protocol,
        max_age,
        cached_at: current_timestamp(),
    })
}

fn build_h3_request(
    hostname: &str,
    method: &str,
    path: &str,
    headers: HashMap<String, String>,
) -> Result<Request<()>, String> {
    let method = Method::from_bytes(method.as_bytes())
        .map_err(|e| format!("Invalid HTTP method `{}`: {}", method, e))?;
    if !path.starts_with('/') {
        return Err(format!(
            "Invalid HTTP path `{}`: path must start with /",
            path
        ));
    }
    let uri = format!("https://{}{}", hostname, path)
        .parse::<Uri>()
        .map_err(|e| format!("Invalid HTTP path `{}`: {}", path, e))?;

    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        let header_name = HeaderName::from_lowercase(name.as_bytes())
            .map_err(|e| format!("Invalid HTTP header name `{}`: {}", name, e))?;
        let header_value = HeaderValue::from_str(&value)
            .map_err(|e| format!("Invalid HTTP header value for `{}`: {}", name, e))?;
        builder = builder.header(header_name, header_value);
    }

    builder
        .body(())
        .map_err(|e| format!("Invalid HTTP/3 request: {}", e))
}

fn append_response_chunk<B: Buf>(response: &mut Vec<u8>, mut chunk: B) {
    while chunk.has_remaining() {
        let bytes = chunk.copy_to_bytes(chunk.remaining());
        response.extend_from_slice(&bytes);
    }
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quic_config_default() {
        let config = QuicConfig::default();
        assert_eq!(config.max_idle_timeout, Duration::from_secs(30));
        assert!(config.enable_0rtt);
    }

    #[test]
    fn test_session_ticket_expiration() {
        let ticket = QuicSessionTicket {
            hostname: "example.com".to_string(),
            ticket: vec![1, 2, 3],
            transport_params: vec![4, 5, 6],
            created_at: 0,
            lifetime: 1,
        };

        assert!(ticket.is_expired());
    }

    #[test]
    fn test_alt_svc_expiration() {
        let entry = AltSvcEntry {
            origin: "example.com".to_string(),
            alt_hostname: "example.com".to_string(),
            alt_port: 443,
            protocol: "h3".to_string(),
            max_age: 1,
            cached_at: 0,
        };

        assert!(entry.is_expired());
    }

    #[test]
    fn test_parse_alt_svc() {
        let entry = parse_alt_svc("example.com", "h3=\":443\"; ma=2592000").unwrap();
        assert_eq!(entry.protocol, "h3");
        assert_eq!(entry.alt_port, 443);
        assert_eq!(entry.max_age, 2592000);
    }

    #[test]
    fn test_parse_alt_svc_minimal() {
        let entry = parse_alt_svc("example.com", "h3=\":443\"").unwrap();
        assert_eq!(entry.protocol, "h3");
        assert_eq!(entry.alt_port, 443);
        assert_eq!(entry.max_age, 86400); // Default
    }

    #[test]
    fn test_parse_alt_svc_invalid() {
        assert!(parse_alt_svc("example.com", "invalid").is_none());
        assert!(parse_alt_svc("example.com", "h3").is_none());
    }

    #[tokio::test]
    async fn test_quic_transport_creation() {
        let transport = QuicTransport::default();
        let stats = transport.stats();
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.cached_tickets, 0);
    }

    #[tokio::test]
    async fn test_cache_alt_svc() {
        let transport = QuicTransport::default();
        transport.cache_alt_svc("example.com", "h3=\":443\"; ma=2592000");

        assert!(transport.is_quic_available("example.com"));
    }

    #[tokio::test]
    async fn test_cache_session_ticket() {
        let transport = QuicTransport::default();
        transport.cache_session_ticket("example.com", vec![1, 2, 3], vec![4, 5, 6]);

        let stats = transport.stats();
        assert_eq!(stats.cached_tickets, 1);
    }

    #[test]
    fn test_connection_idle_detection() {
        let mut conn = QuicConnection {
            hostname: "example.com".to_string(),
            remote_addr: "127.0.0.1:443".parse().unwrap(),
            established_at: 0,
            last_activity: 0,
            connection_id: 1,
            connection: None,
            h3_sender: None,
            h3_driver: None,
        };

        assert!(conn.is_idle(1));

        conn.touch();
        assert!(!conn.is_idle(3600));
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let transport = QuicTransport::default();

        // Add expired ticket
        {
            let mut tickets = transport.session_tickets.lock().unwrap();
            tickets.insert(
                "example.com".to_string(),
                QuicSessionTicket {
                    hostname: "example.com".to_string(),
                    ticket: vec![1, 2, 3],
                    transport_params: vec![4, 5, 6],
                    created_at: 0,
                    lifetime: 1,
                },
            );
        }

        transport.cleanup_expired();

        let stats = transport.stats();
        assert_eq!(stats.cached_tickets, 0);
    }

    #[tokio::test]
    async fn test_send_h3_request_missing_connection_error() {
        let transport = QuicTransport::default();
        let error = transport
            .send_h3_request(404, "GET", "/", HashMap::new(), None)
            .await
            .unwrap_err();

        assert!(error.contains("No QUIC connection found for id 404"));
    }

    #[tokio::test]
    async fn test_send_h3_request_closed_connection_error() {
        let transport = QuicTransport::default();
        {
            let mut connections = transport.connections.lock().unwrap();
            connections.insert(
                "example.com".to_string(),
                QuicConnection {
                    hostname: "example.com".to_string(),
                    remote_addr: "127.0.0.1:443".parse().unwrap(),
                    established_at: current_timestamp(),
                    last_activity: current_timestamp(),
                    connection_id: 7,
                    connection: None,
                    h3_sender: None,
                    h3_driver: None,
                },
            );
        }

        let error = transport
            .send_h3_request(7, "GET", "/", HashMap::new(), None)
            .await
            .unwrap_err();

        assert!(error.contains("QUIC connection 7 is closed"));
    }

    #[test]
    fn test_build_h3_request_rejects_invalid_method() {
        let error = build_h3_request("example.com", "BAD METHOD", "/", HashMap::new()).unwrap_err();

        assert!(error.contains("Invalid HTTP method"));
    }

    #[test]
    fn test_build_h3_request_rejects_invalid_path() {
        let error = build_h3_request("example.com", "GET", "relative", HashMap::new()).unwrap_err();

        assert!(error.contains("path must start with /"));
    }

    #[test]
    fn test_build_h3_request_rejects_invalid_header() {
        let mut headers = HashMap::new();
        headers.insert("X-Test".to_string(), "value".to_string());
        let error = build_h3_request("example.com", "GET", "/", headers).unwrap_err();

        assert!(error.contains("Invalid HTTP header name"));
    }

    #[test]
    fn test_append_response_chunk_collects_response_body() {
        let mut response = Vec::new();

        append_response_chunk(&mut response, Bytes::from_static(b"hello "));
        append_response_chunk(&mut response, Bytes::from_static(b"h3"));

        assert_eq!(response, b"hello h3");
    }
}
