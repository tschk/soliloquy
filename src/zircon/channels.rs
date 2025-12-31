//! Zircon channel-based IPC primitives
//!
//! Channels are bidirectional message queues that support:
//! - Reliable, ordered message delivery
//! - Handle transfer (pass VMOs, sockets, etc between processes)
//! - Non-blocking and blocking read/write
//!
//! Used for tab process communication and FIDL protocol transport.

use log::{debug, error, warn};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Message sent through a channel
#[derive(Debug, Clone)]
pub struct ChannelMessage {
    /// Message data (byte payload)
    pub data: Vec<u8>,
    /// Transferred handles (VMOs, other channels, etc)
    pub handles: Vec<u64>,
}

impl ChannelMessage {
    /// Create a new channel message
    pub fn new(data: Vec<u8>) -> Self {
        ChannelMessage {
            data,
            handles: Vec::new(),
        }
    }

    /// Create a message with handles
    pub fn with_handles(data: Vec<u8>, handles: Vec<u64>) -> Self {
        ChannelMessage { data, handles }
    }

    /// Get message size in bytes
    pub fn size(&self) -> usize {
        self.data.len() + self.handles.len() * 8
    }
}

/// One end of a bidirectional channel
pub struct ChannelEndpoint {
    /// Handle ID for this endpoint
    handle: u64,
    /// Message queue (shared with peer)
    queue: Arc<Mutex<VecDeque<ChannelMessage>>>,
    /// Peer's message queue
    peer_queue: Arc<Mutex<VecDeque<ChannelMessage>>>,
    /// Whether peer is closed
    peer_closed: Arc<Mutex<bool>>,
}

impl ChannelEndpoint {
    /// Write a message to the channel
    pub fn write(&self, message: ChannelMessage) -> Result<(), String> {
        let peer_closed = self.peer_closed.lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        
        if *peer_closed {
            return Err("Peer endpoint closed".to_string());
        }
        drop(peer_closed);

        let mut peer_queue = self.peer_queue.lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        
        debug!("Channel {}: Writing message ({} bytes, {} handles)", 
               self.handle, message.data.len(), message.handles.len());
        
        peer_queue.push_back(message);
        Ok(())
    }

    /// Read a message from the channel (non-blocking)
    pub fn read(&self) -> Result<ChannelMessage, String> {
        let mut queue = self.queue.lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        
        queue.pop_front()
            .ok_or_else(|| "No messages available".to_string())
    }

    /// Try to read a message (returns None if queue is empty)
    pub fn try_read(&self) -> Option<ChannelMessage> {
        let mut queue = self.queue.lock().ok()?;
        queue.pop_front()
    }

    /// Check if there are pending messages
    pub fn has_messages(&self) -> bool {
        let queue = self.queue.lock().ok();
        queue.map(|q| !q.is_empty()).unwrap_or(false)
    }

    /// Get the handle ID
    pub fn handle(&self) -> u64 {
        self.handle
    }
}

impl Drop for ChannelEndpoint {
    fn drop(&mut self) {
        debug!("Closing channel endpoint {}", self.handle);
        if let Ok(mut closed) = self.peer_closed.lock() {
            *closed = true;
        }
    }
}

/// Create a bidirectional channel pair
pub fn create_channel() -> Result<(ChannelEndpoint, ChannelEndpoint), String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);
    
    let handle1 = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let handle2 = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);

    let queue1 = Arc::new(Mutex::new(VecDeque::new()));
    let queue2 = Arc::new(Mutex::new(VecDeque::new()));
    let closed = Arc::new(Mutex::new(false));

    let endpoint1 = ChannelEndpoint {
        handle: handle1,
        queue: queue1.clone(),
        peer_queue: queue2.clone(),
        peer_closed: closed.clone(),
    };

    let endpoint2 = ChannelEndpoint {
        handle: handle2,
        queue: queue2,
        peer_queue: queue1,
        peer_closed: closed,
    };

    debug!("Created channel pair: {} <-> {}", handle1, handle2);

    Ok((endpoint1, endpoint2))
}

/// Tab communication channel manager
pub struct TabChannelManager {
    /// Channels indexed by tab ID
    channels: Mutex<std::collections::HashMap<u64, ChannelEndpoint>>,
}

impl TabChannelManager {
    /// Create a new channel manager
    pub fn new() -> Self {
        TabChannelManager {
            channels: Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Create a channel for a tab
    pub fn create_tab_channel(&self, tab_id: u64) -> Result<ChannelEndpoint, String> {
        let (endpoint1, endpoint2) = create_channel()?;
        
        let mut channels = self.channels.lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        
        channels.insert(tab_id, endpoint1);
        
        debug!("Created channel for tab {}", tab_id);
        Ok(endpoint2) // Return the other endpoint to the caller
    }

    /// Send message to a tab
    pub fn send_to_tab(&self, tab_id: u64, message: ChannelMessage) -> Result<(), String> {
        let channels = self.channels.lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        
        let channel = channels.get(&tab_id)
            .ok_or_else(|| format!("Tab {} has no channel", tab_id))?;
        
        channel.write(message)
    }

    /// Receive message from a tab (non-blocking)
    pub fn receive_from_tab(&self, tab_id: u64) -> Result<ChannelMessage, String> {
        let channels = self.channels.lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        
        let channel = channels.get(&tab_id)
            .ok_or_else(|| format!("Tab {} has no channel", tab_id))?;
        
        channel.read()
    }

    /// Remove a tab's channel
    pub fn remove_tab_channel(&self, tab_id: u64) -> Result<(), String> {
        let mut channels = self.channels.lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        
        channels.remove(&tab_id)
            .ok_or_else(|| format!("Tab {} has no channel", tab_id))?;
        
        debug!("Removed channel for tab {}", tab_id);
        Ok(())
    }
}

impl Default for TabChannelManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_creation() {
        let result = create_channel();
        assert!(result.is_ok());
        
        let (ep1, ep2) = result.unwrap();
        assert_ne!(ep1.handle(), ep2.handle());
    }

    #[test]
    fn test_channel_write_read() {
        let (ep1, ep2) = create_channel().unwrap();
        
        let msg = ChannelMessage::new(vec![1, 2, 3, 4]);
        ep1.write(msg.clone()).unwrap();
        
        let received = ep2.read().unwrap();
        assert_eq!(received.data, msg.data);
    }

    #[test]
    fn test_channel_bidirectional() {
        let (ep1, ep2) = create_channel().unwrap();
        
        ep1.write(ChannelMessage::new(vec![1, 2, 3])).unwrap();
        ep2.write(ChannelMessage::new(vec![4, 5, 6])).unwrap();
        
        let msg1 = ep2.read().unwrap();
        let msg2 = ep1.read().unwrap();
        
        assert_eq!(msg1.data, vec![1, 2, 3]);
        assert_eq!(msg2.data, vec![4, 5, 6]);
    }

    #[test]
    fn test_channel_with_handles() {
        let (ep1, ep2) = create_channel().unwrap();
        
        let msg = ChannelMessage::with_handles(
            vec![1, 2, 3],
            vec![100, 200, 300],
        );
        
        ep1.write(msg).unwrap();
        let received = ep2.read().unwrap();
        
        assert_eq!(received.handles.len(), 3);
        assert_eq!(received.handles, vec![100, 200, 300]);
    }

    #[test]
    fn test_channel_empty_read() {
        let (_, ep2) = create_channel().unwrap();
        
        let result = ep2.read();
        assert!(result.is_err());
    }

    #[test]
    fn test_channel_try_read() {
        let (ep1, ep2) = create_channel().unwrap();
        
        assert!(ep2.try_read().is_none());
        
        ep1.write(ChannelMessage::new(vec![1, 2])).unwrap();
        assert!(ep2.try_read().is_some());
    }

    #[test]
    fn test_channel_has_messages() {
        let (ep1, ep2) = create_channel().unwrap();
        
        assert!(!ep2.has_messages());
        
        ep1.write(ChannelMessage::new(vec![1])).unwrap();
        assert!(ep2.has_messages());
    }

    #[test]
    fn test_tab_channel_manager() {
        let manager = TabChannelManager::new();
        
        let endpoint = manager.create_tab_channel(1).unwrap();
        
        let msg = ChannelMessage::new(vec![1, 2, 3]);
        endpoint.write(msg).unwrap();
        
        let received = manager.receive_from_tab(1).unwrap();
        assert_eq!(received.data, vec![1, 2, 3]);
    }

    #[test]
    fn test_tab_channel_removal() {
        let manager = TabChannelManager::new();
        
        manager.create_tab_channel(1).unwrap();
        let result = manager.remove_tab_channel(1);
        assert!(result.is_ok());
        
        let result = manager.receive_from_tab(1);
        assert!(result.is_err());
    }
}
