//! Connection Manager
//!
//! Manages DNS caching, TLS session resumption, and HTTP connection pooling
//! for improved performance and reduced latency.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use log::{debug, warn};
use tokio::{net, time};

/// Default DNS cache TTL (5 minutes)
const DEFAULT_DNS_TTL: u64 = 300;

/// Default TLS session lifetime (24 hours)
const DEFAULT_TLS_LIFETIME: u64 = 86400;

/// Maximum connections per host
const MAX_CONNECTIONS_PER_HOST: usize = 6;

/// DNS cache entry
#[derive(Debug, Clone)]
pub struct DnsEntry {
    /// Resolved IP addresses
    pub addresses: Vec<IpAddr>,
    /// Timestamp when cached (Unix epoch)
    pub cached_at: u64,
    /// Time-to-live in seconds
    pub ttl: u64,
}

impl DnsEntry {
    /// Check if entry is expired
    pub fn is_expired(&self) -> bool {
        let now = current_timestamp();
        now > self.cached_at + self.ttl
    }
}

/// TLS session ticket for resumption
#[derive(Debug, Clone)]
pub struct TlsSession {
    /// Hostname
    pub hostname: String,
    /// Session ticket data
    pub ticket: Vec<u8>,
    /// Creation timestamp
    pub created_at: u64,
    /// Lifetime in seconds
    pub lifetime: u64,
}

impl TlsSession {
    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        let now = current_timestamp();
        now > self.created_at + self.lifetime
    }
}

/// Pooled HTTP connection
#[derive(Debug, Clone)]
pub struct PooledConnection {
    /// Hostname
    pub hostname: String,
    /// Connection in use
    pub in_use: bool,
    /// Last used timestamp
    pub last_used: u64,
    /// Unique connection identifier
    pub connection_id: u64,
}

impl PooledConnection {
    /// Check if connection is idle for too long (60 seconds)
    pub fn is_idle_timeout(&self) -> bool {
        let now = current_timestamp();
        !self.in_use && now > self.last_used + 60
    }
}

/// Connection manager for DNS, TLS, and connection pooling
pub struct ConnectionManager {
    /// DNS cache: hostname -> DnsEntry
    dns_cache: Arc<Mutex<HashMap<String, DnsEntry>>>,
    /// TLS session cache: hostname -> TlsSession
    tls_sessions: Arc<Mutex<HashMap<String, TlsSession>>>,
    /// Connection pool: connection_id -> PooledConnection
    connection_pool: Arc<Mutex<HashMap<u64, PooledConnection>>>,
    /// Next connection ID
    next_connection_id: Arc<Mutex<u64>>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new() -> Self {
        Self {
            dns_cache: Arc::new(Mutex::new(HashMap::new())),
            tls_sessions: Arc::new(Mutex::new(HashMap::new())),
            connection_pool: Arc::new(Mutex::new(HashMap::new())),
            next_connection_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Resolve DNS with caching
    ///
    /// # Arguments
    /// * `hostname` - Hostname to resolve
    ///
    /// # Returns
    /// Vector of resolved IP addresses
    pub async fn resolve_dns(&self, hostname: &str) -> Result<Vec<IpAddr>, String> {
        // Check cache first
        {
            let cache = self.dns_cache.lock().unwrap();
            if let Some(entry) = cache.get(hostname) {
                if !entry.is_expired() {
                    debug!("DNS cache hit for {}", hostname);
                    return Ok(entry.addresses.clone());
                }
            }
        }

        debug!("DNS cache miss for {}, performing lookup", hostname);

        // Perform DNS lookup using tokio
        let host_port = format!("{}:0", hostname);
        let addresses: Vec<IpAddr> = net::lookup_host(&host_port)
            .await
            .map_err(|e| format!("DNS lookup failed: {}", e))?
            .map(|socket_addr| socket_addr.ip())
            .collect();

        // Cache result
        let entry = DnsEntry {
            addresses: addresses.clone(),
            cached_at: current_timestamp(),
            ttl: DEFAULT_DNS_TTL,
        };

        {
            let mut cache = self.dns_cache.lock().unwrap();
            cache.insert(hostname.to_string(), entry);
        }

        Ok(addresses)
    }

    /// Get cached TLS session for resumption
    ///
    /// # Arguments
    /// * `hostname` - Server hostname
    ///
    /// # Returns
    /// Optional TLS session ticket
    pub fn get_tls_session(&self, hostname: &str) -> Option<Vec<u8>> {
        let sessions = self.tls_sessions.lock().unwrap();
        if let Some(session) = sessions.get(hostname) {
            if !session.is_expired() {
                debug!("TLS session cache hit for {}", hostname);
                return Some(session.ticket.clone());
            }
        }
        None
    }

    /// Cache TLS session for future resumption
    ///
    /// # Arguments
    /// * `hostname` - Server hostname
    /// * `ticket` - Session ticket data
    pub fn cache_tls_session(&self, hostname: &str, ticket: Vec<u8>) {
        let session = TlsSession {
            hostname: hostname.to_string(),
            ticket,
            created_at: current_timestamp(),
            lifetime: DEFAULT_TLS_LIFETIME,
        };

        let mut sessions = self.tls_sessions.lock().unwrap();
        sessions.insert(hostname.to_string(), session);
        debug!("Cached TLS session for {}", hostname);
    }

    /// Get available connection from pool
    ///
    /// # Arguments
    /// * `hostname` - Target hostname
    ///
    /// # Returns
    /// Optional connection ID if available
    pub fn get_connection(&self, hostname: &str) -> Option<u64> {
        let mut pool = self.connection_pool.lock().unwrap();
        
        for (id, conn) in pool.iter_mut() {
            if conn.hostname == hostname && !conn.in_use && !conn.is_idle_timeout() {
                conn.in_use = true;
                conn.last_used = current_timestamp();
                debug!("Reusing pooled connection {} for {}", id, hostname);
                return Some(*id);
            }
        }

        // No available connection, create new if under limit
        let host_connections = pool.values().filter(|c| c.hostname == hostname).count();
        if host_connections < MAX_CONNECTIONS_PER_HOST {
            let connection_id = {
                let mut next_id = self.next_connection_id.lock().unwrap();
                let id = *next_id;
                *next_id += 1;
                id
            };

            let conn = PooledConnection {
                hostname: hostname.to_string(),
                in_use: true,
                last_used: current_timestamp(),
                connection_id,
            };

            pool.insert(connection_id, conn);
            debug!("Created new pooled connection {} for {}", connection_id, hostname);
            return Some(connection_id);
        }

        None
    }

    /// Release connection back to pool
    ///
    /// # Arguments
    /// * `connection_id` - Connection identifier
    pub fn release_connection(&self, connection_id: u64) {
        let mut pool = self.connection_pool.lock().unwrap();
        if let Some(conn) = pool.get_mut(&connection_id) {
            conn.in_use = false;
            conn.last_used = current_timestamp();
            debug!("Released connection {} back to pool", connection_id);
        }
    }

    /// Prewarm connection to a host
    ///
    /// # Arguments
    /// * `hostname` - Target hostname
    pub async fn prewarm_connection(&self, hostname: &str) -> Result<(), String> {
        debug!("Prewarming connection to {}", hostname);
        
        // Resolve DNS
        let _ = self.resolve_dns(hostname).await?;
        
        // Reserve connection slot
        let _ = self.get_connection(hostname);
        
        Ok(())
    }

    /// Clean up expired entries and idle connections
    pub fn cleanup_expired(&self) {
        // Clean DNS cache
        {
            let mut cache = self.dns_cache.lock().unwrap();
            cache.retain(|hostname, entry| {
                let keep = !entry.is_expired();
                if !keep {
                    debug!("Removing expired DNS entry for {}", hostname);
                }
                keep
            });
        }

        // Clean TLS sessions
        {
            let mut sessions = self.tls_sessions.lock().unwrap();
            sessions.retain(|hostname, session| {
                let keep = !session.is_expired();
                if !keep {
                    debug!("Removing expired TLS session for {}", hostname);
                }
                keep
            });
        }

        // Clean idle connections
        {
            let mut pool = self.connection_pool.lock().unwrap();
            pool.retain(|id, conn| {
                let keep = !conn.is_idle_timeout();
                if !keep {
                    debug!("Removing idle connection {}", id);
                }
                keep
            });
        }
    }

    /// Start background cleanup task
    pub fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                self.cleanup_expired();
            }
        });
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
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

    #[tokio::test]
    async fn test_dns_caching() {
        let manager = ConnectionManager::new();
        
        let result1 = manager.resolve_dns("example.com").await.unwrap();
        let result2 = manager.resolve_dns("example.com").await.unwrap();
        
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_tls_session_caching() {
        let manager = ConnectionManager::new();
        let ticket = vec![1, 2, 3, 4, 5];
        
        manager.cache_tls_session("example.com", ticket.clone());
        let cached = manager.get_tls_session("example.com");
        
        assert_eq!(cached, Some(ticket));
    }

    #[test]
    fn test_connection_pooling() {
        let manager = ConnectionManager::new();
        
        let conn1 = manager.get_connection("example.com");
        assert!(conn1.is_some());
        
        let conn2 = manager.get_connection("example.com");
        assert!(conn2.is_some());
        assert_ne!(conn1, conn2);
    }

    #[test]
    fn test_connection_reuse() {
        let manager = ConnectionManager::new();
        
        let conn1 = manager.get_connection("example.com").unwrap();
        manager.release_connection(conn1);
        
        let conn2 = manager.get_connection("example.com").unwrap();
        assert_eq!(conn1, conn2);
    }

    #[test]
    fn test_dns_entry_expiration() {
        let entry = DnsEntry {
            addresses: vec![IpAddr::from([127, 0, 0, 1])],
            cached_at: 0,
            ttl: 1,
        };
        
        assert!(entry.is_expired());
    }

    #[test]
    fn test_tls_session_expiration() {
        let session = TlsSession {
            hostname: "example.com".to_string(),
            ticket: vec![1, 2, 3],
            created_at: 0,
            lifetime: 1,
        };
        
        assert!(session.is_expired());
    }

    #[test]
    fn test_connection_idle_timeout() {
        let conn = PooledConnection {
            hostname: "example.com".to_string(),
            in_use: false,
            last_used: 0,
            connection_id: 1,
        };
        
        assert!(conn.is_idle_timeout());
    }
}
