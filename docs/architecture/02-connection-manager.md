# Connection Manager

**Document Status:** Complete  
**Last Updated:** 2024-01-08  
**Related:** [`01-overview.md`](01-overview.md), [`../protocol/01-connection.md`](../protocol/01-connection.md)

## Overview

The Connection Manager is the **entry point** for all client connections. It handles TCP socket management, RC4 encryption/decryption, connection lifecycle, rate limiting, and message buffering.

**Responsibilities:**
- ✅ Accept TCP connections on port 5555
- ✅ Initialize RC4 cipher for each connection
- ✅ Read/write encrypted binary messages
- ✅ Buffer incomplete messages
- ✅ Track connection state (unauthenticated → authenticated)
- ✅ Enforce rate limits and connection limits
- ✅ Detect and handle disconnections
- ✅ Forward decrypted messages to Message Router

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    Connection Manager                         │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  TCP Listener (port 5555)                                    │
│       │                                                       │
│       ├─► Connection Handler (per client)                    │
│       │    ├─► RC4 Cipher (encrypt/decrypt)                  │
│       │    ├─► Message Buffer (incomplete messages)          │
│       │    ├─► Connection State Machine                      │
│       │    ├─► Rate Limiter                                  │
│       │    └─► Message Router (forward)                      │
│       │                                                       │
│       └─► Connection Pool (DashMap<ConnectionId, Connection>)│
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

## Data Structures

### Connection

```rust
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use dashmap::DashMap;
use std::sync::Arc;

/// Represents a single client connection
pub struct Connection {
    pub id: ConnectionId,
    pub socket: TcpStream,
    pub cipher: Rc4Cipher,
    pub state: ConnectionState,
    pub buffer: MessageBuffer,
    pub rate_limiter: RateLimiter,
    pub player_id: Option<u16>,
    
    // Metadata
    pub connected_at: Instant,
    pub last_activity: Instant,
    pub remote_addr: SocketAddr,
    
    // Communication
    pub outbound_tx: mpsc::UnboundedSender<Vec<u8>>,
    pub outbound_rx: mpsc::UnboundedReceiver<Vec<u8>>,
}

/// Unique connection identifier
pub type ConnectionId = Uuid;

/// Connection state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Just connected, waiting for MSG_LOGIN or MSG_REGISTER
    Unauthenticated,
    
    /// Authenticated, normal operation
    Authenticated {
        player_id: u16,
        account_id: u32,
    },
    
    /// Gracefully closing
    Closing,
    
    /// Closed (should be removed from pool)
    Closed,
}

/// RC4 cipher for encryption/decryption
pub struct Rc4Cipher {
    encrypt: rc4::Rc4,  // Encrypt outgoing with CLIENT_DECRYPT_KEY
    decrypt: rc4::Rc4,  // Decrypt incoming with CLIENT_ENCRYPT_KEY
}

impl Rc4Cipher {
    pub fn new() -> Self {
        use rc4::{Rc4, KeyInit};
        
        const CLIENT_ENCRYPT_KEY: &[u8] = b"retrtz7jmijb5467n47";
        const CLIENT_DECRYPT_KEY: &[u8] = b"t54gz65u74njb6zg6";
        
        Self {
            decrypt: Rc4::new(CLIENT_ENCRYPT_KEY.into()),
            encrypt: Rc4::new(CLIENT_DECRYPT_KEY.into()),
        }
    }
    
    pub fn decrypt(&mut self, data: &mut [u8]) {
        use rc4::StreamCipher;
        self.decrypt.apply_keystream(data);
    }
    
    pub fn encrypt(&mut self, data: &mut [u8]) {
        use rc4::StreamCipher;
        self.encrypt.apply_keystream(data);
    }
}
```

### Message Buffer

Handles incomplete messages (message may arrive in multiple TCP packets):

```rust
pub struct MessageBuffer {
    buffer: Vec<u8>,
    max_buffer_size: usize,
}

impl MessageBuffer {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(8192),
            max_buffer_size: 65536, // 64KB max
        }
    }
    
    /// Append incoming data to buffer
    pub fn append(&mut self, data: &[u8]) -> Result<(), BufferError> {
        if self.buffer.len() + data.len() > self.max_buffer_size {
            return Err(BufferError::BufferOverflow);
        }
        
        self.buffer.extend_from_slice(data);
        Ok(())
    }
    
    /// Extract complete messages from buffer
    /// Returns Vec of complete messages, leaving incomplete data in buffer
    pub fn extract_messages(&mut self) -> Vec<Vec<u8>> {
        let mut messages = Vec::new();
        
        while self.buffer.len() >= 2 {
            // Read message length (first 2 bytes, little-endian)
            let msg_len = u16::from_le_bytes([
                self.buffer[0],
                self.buffer[1],
            ]) as usize;
            
            // Check if we have the complete message
            if self.buffer.len() < 2 + msg_len {
                break; // Incomplete message, wait for more data
            }
            
            // Extract message (skip 2-byte length prefix)
            let message = self.buffer.drain(0..2 + msg_len).skip(2).collect();
            messages.push(message);
        }
        
        messages
    }
    
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
    
    pub fn len(&self) -> usize {
        self.buffer.len()
    }
}

#[derive(Debug)]
pub enum BufferError {
    BufferOverflow,
}
```

### Rate Limiter

Prevents message spam and DDoS:

```rust
use std::collections::VecDeque;

pub struct RateLimiter {
    /// Sliding window of message timestamps
    message_times: VecDeque<Instant>,
    
    /// Max messages per window
    max_messages: usize,
    
    /// Window duration
    window: Duration,
    
    /// Violation count
    violations: u32,
}

impl RateLimiter {
    pub fn new(max_messages: usize, window: Duration) -> Self {
        Self {
            message_times: VecDeque::with_capacity(max_messages),
            max_messages,
            window,
            violations: 0,
        }
    }
    
    /// Check if message is allowed
    pub fn check_rate(&mut self) -> RateLimitResult {
        let now = Instant::now();
        
        // Remove old timestamps outside window
        while let Some(&time) = self.message_times.front() {
            if now.duration_since(time) > self.window {
                self.message_times.pop_front();
            } else {
                break;
            }
        }
        
        // Check if under limit
        if self.message_times.len() < self.max_messages {
            self.message_times.push_back(now);
            RateLimitResult::Allowed
        } else {
            self.violations += 1;
            
            if self.violations > 10 {
                RateLimitResult::Ban
            } else {
                RateLimitResult::Throttle
            }
        }
    }
    
    pub fn reset_violations(&mut self) {
        self.violations = 0;
    }
}

pub enum RateLimitResult {
    Allowed,
    Throttle,  // Slow down, drop message
    Ban,       // Too many violations, disconnect
}
```

## Connection Lifecycle

### 1. Accept Connection

```rust
pub struct ConnectionManager {
    connections: Arc<DashMap<ConnectionId, Arc<Mutex<Connection>>>>,
    max_connections: usize,
}

impl ConnectionManager {
    pub fn new(max_connections: usize) -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
            max_connections,
        }
    }
    
    /// Accept new TCP connection
    pub async fn accept_connection(
        &self,
        socket: TcpStream,
        remote_addr: SocketAddr,
    ) -> Result<ConnectionId, ConnectionError> {
        // Check connection limit
        if self.connections.len() >= self.max_connections {
            log::warn!("Connection limit reached, rejecting {}", remote_addr);
            return Err(ConnectionError::ConnectionLimitReached);
        }
        
        // Create connection
        let id = Uuid::new_v4();
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();
        
        let connection = Connection {
            id,
            socket,
            cipher: Rc4Cipher::new(),
            state: ConnectionState::Unauthenticated,
            buffer: MessageBuffer::new(),
            rate_limiter: RateLimiter::new(100, Duration::from_secs(1)),
            player_id: None,
            connected_at: Instant::now(),
            last_activity: Instant::now(),
            remote_addr,
            outbound_tx,
            outbound_rx,
        };
        
        self.connections.insert(id, Arc::new(Mutex::new(connection)));
        
        log::info!("New connection {} from {}", id, remote_addr);
        
        Ok(id)
    }
}
```

### 2. Connection Handler Task

Each connection runs in its own tokio task:

```rust
pub async fn handle_connection(
    conn_id: ConnectionId,
    connections: Arc<DashMap<ConnectionId, Arc<Mutex<Connection>>>>,
    router: Arc<MessageRouter>,
) {
    let connection = connections.get(&conn_id)
        .expect("Connection not found")
        .clone();
    
    // Split socket into read/write halves
    let (mut read_half, mut write_half) = {
        let mut conn = connection.lock().await;
        conn.socket.split()
    };
    
    // Spawn writer task
    let conn_clone = connection.clone();
    tokio::spawn(async move {
        handle_outbound_messages(conn_clone, &mut write_half).await;
    });
    
    // Handle inbound messages
    let mut buffer = vec![0u8; 8192];
    
    loop {
        match read_half.read(&mut buffer).await {
            Ok(0) => {
                // Connection closed
                log::info!("Connection {} closed", conn_id);
                break;
            }
            
            Ok(n) => {
                let mut conn = connection.lock().await;
                
                // Decrypt incoming data
                conn.cipher.decrypt(&mut buffer[..n]);
                
                // Append to message buffer
                if let Err(e) = conn.buffer.append(&buffer[..n]) {
                    log::error!("Buffer error for {}: {:?}", conn_id, e);
                    break;
                }
                
                // Extract complete messages
                let messages = conn.buffer.extract_messages();
                
                // Update activity
                conn.last_activity = Instant::now();
                
                // Release lock before processing
                drop(conn);
                
                // Process each message
                for message in messages {
                    if let Err(e) = process_message(
                        conn_id,
                        &message,
                        &connection,
                        &router,
                    ).await {
                        log::error!("Message processing error: {:?}", e);
                    }
                }
            }
            
            Err(e) => {
                log::error!("Read error for {}: {}", conn_id, e);
                break;
            }
        }
    }
    
    // Cleanup
    cleanup_connection(conn_id, &connections, &router).await;
}
```

### 3. Process Incoming Message

```rust
async fn process_message(
    conn_id: ConnectionId,
    message: &[u8],
    connection: &Arc<Mutex<Connection>>,
    router: &Arc<MessageRouter>,
) -> Result<(), ProcessError> {
    // Check rate limit
    {
        let mut conn = connection.lock().await;
        match conn.rate_limiter.check_rate() {
            RateLimitResult::Allowed => {},
            RateLimitResult::Throttle => {
                log::warn!("Rate limit throttle for {}", conn_id);
                return Ok(()); // Drop message
            }
            RateLimitResult::Ban => {
                log::warn!("Rate limit ban for {}", conn_id);
                return Err(ProcessError::RateLimitExceeded);
            }
        }
    }
    
    // Parse message type (first 2 bytes)
    if message.len() < 2 {
        log::warn!("Message too short from {}", conn_id);
        return Ok(());
    }
    
    let msg_type = u16::from_le_bytes([message[0], message[1]]);
    let payload = &message[2..];
    
    // Validate authentication
    {
        let conn = connection.lock().await;
        
        if conn.state == ConnectionState::Unauthenticated {
            // Only MSG_LOGIN (10) and MSG_REGISTER (7) allowed
            if msg_type != 10 && msg_type != 7 {
                log::warn!("Unauthenticated message {} from {}", msg_type, conn_id);
                return Err(ProcessError::NotAuthenticated);
            }
        }
    }
    
    // Route message
    router.route_message(conn_id, msg_type, payload).await?;
    
    Ok(())
}
```

### 4. Send Outbound Message

```rust
impl Connection {
    /// Queue message for sending
    pub fn send_message(&self, message: &[u8]) -> Result<(), SendError> {
        // Encrypt message
        let mut encrypted = message.to_vec();
        self.cipher.encrypt(&mut encrypted);
        
        // Prepend length (little-endian u16)
        let len = encrypted.len() as u16;
        let mut packet = vec![
            (len & 0xFF) as u8,
            (len >> 8) as u8,
        ];
        packet.extend_from_slice(&encrypted);
        
        // Queue for sending
        self.outbound_tx.send(packet)
            .map_err(|_| SendError::ChannelClosed)?;
        
        Ok(())
    }
}

async fn handle_outbound_messages(
    connection: Arc<Mutex<Connection>>,
    write_half: &mut tokio::net::tcp::WriteHalf<'_>,
) {
    let mut rx = {
        let conn = connection.lock().await;
        conn.outbound_rx.clone()
    };
    
    while let Some(packet) = rx.recv().await {
        if let Err(e) = write_half.write_all(&packet).await {
            log::error!("Write error: {}", e);
            break;
        }
    }
}
```

### 5. Cleanup Connection

```rust
async fn cleanup_connection(
    conn_id: ConnectionId,
    connections: &Arc<DashMap<ConnectionId, Arc<Mutex<Connection>>>>,
    router: &Arc<MessageRouter>,
) {
    // Get player_id before removing
    let player_id = {
        if let Some(conn) = connections.get(&conn_id) {
            let conn = conn.lock().await;
            conn.player_id
        } else {
            None
        }
    };
    
    // Notify game state
    if let Some(player_id) = player_id {
        router.handle_disconnect(player_id).await;
    }
    
    // Remove from connection pool
    connections.remove(&conn_id);
    
    log::info!("Connection {} cleaned up", conn_id);
}
```

## State Transitions

```
                    ┌─────────────────┐
                    │ TCP Connection  │
                    │   Established   │
                    └────────┬────────┘
                             │
                             v
                  ┌──────────────────────┐
                  │  Unauthenticated     │
                  │  (waiting for login) │
                  └──────────┬───────────┘
                             │
                    MSG_LOGIN or MSG_REGISTER
                             │
                             v
                  ┌──────────────────────┐
                  │    Authenticated     │
                  │  (normal operation)  │
                  └──────────┬───────────┘
                             │
                   MSG_LOGOUT or timeout
                             │
                             v
                  ┌──────────────────────┐
                  │      Closing         │
                  │  (graceful shutdown) │
                  └──────────┬───────────┘
                             │
                             v
                  ┌──────────────────────┐
                  │       Closed         │
                  │   (cleanup done)     │
                  └──────────────────────┘
```

## Rate Limiting Strategy

### Per-Connection Limits

```rust
pub struct RateLimitConfig {
    // General message limit
    pub max_messages_per_second: usize,
    
    // Specific message type limits
    pub max_movement_per_second: usize,
    pub max_chat_per_second: usize,
    pub max_item_ops_per_second: usize,
    
    // Connection limits
    pub max_connections_per_ip: usize,
    pub max_total_connections: usize,
    
    // Violation thresholds
    pub throttle_threshold: u32,
    pub ban_threshold: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_messages_per_second: 100,
            max_movement_per_second: 60,
            max_chat_per_second: 5,
            max_item_ops_per_second: 10,
            max_connections_per_ip: 3,
            max_total_connections: 500,
            throttle_threshold: 5,
            ban_threshold: 10,
        }
    }
}
```

### IP-Based Rate Limiting

```rust
pub struct IpRateLimiter {
    connections_per_ip: Arc<DashMap<IpAddr, Vec<ConnectionId>>>,
    max_per_ip: usize,
}

impl IpRateLimiter {
    pub fn check_ip(&self, ip: IpAddr) -> bool {
        let count = self.connections_per_ip
            .get(&ip)
            .map(|c| c.len())
            .unwrap_or(0);
        
        count < self.max_per_ip
    }
    
    pub fn register_connection(&self, ip: IpAddr, conn_id: ConnectionId) {
        self.connections_per_ip
            .entry(ip)
            .or_insert_with(Vec::new)
            .push(conn_id);
    }
    
    pub fn unregister_connection(&self, ip: IpAddr, conn_id: ConnectionId) {
        if let Some(mut conns) = self.connections_per_ip.get_mut(&ip) {
            conns.retain(|&id| id != conn_id);
            
            if conns.is_empty() {
                drop(conns);
                self.connections_per_ip.remove(&ip);
            }
        }
    }
}
```

## Error Handling

```rust
#[derive(Debug)]
pub enum ConnectionError {
    ConnectionLimitReached,
    IpBanned,
    RateLimitExceeded,
    SocketError(std::io::Error),
    EncryptionError,
    BufferOverflow,
}

#[derive(Debug)]
pub enum ProcessError {
    NotAuthenticated,
    InvalidMessage,
    RateLimitExceeded,
    RouteError,
}

#[derive(Debug)]
pub enum SendError {
    ChannelClosed,
    SocketClosed,
}
```

## Monitoring & Metrics

```rust
pub struct ConnectionMetrics {
    pub total_connections: AtomicU64,
    pub active_connections: AtomicU64,
    pub messages_received: AtomicU64,
    pub messages_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub rate_limit_violations: AtomicU64,
}

impl ConnectionMetrics {
    pub fn log_stats(&self) {
        log::info!(
            "Connections: {} active ({} total), Messages: {} in / {} out, Rate violations: {}",
            self.active_connections.load(Ordering::Relaxed),
            self.total_connections.load(Ordering::Relaxed),
            self.messages_received.load(Ordering::Relaxed),
            self.messages_sent.load(Ordering::Relaxed),
            self.rate_limit_violations.load(Ordering::Relaxed),
        );
    }
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_buffer() {
        let mut buffer = MessageBuffer::new();
        
        // Incomplete message
        let data = vec![0x05, 0x00, 0x0A, 0x00]; // len=5, but only 2 bytes
        buffer.append(&data).unwrap();
        assert_eq!(buffer.extract_messages().len(), 0);
        
        // Complete message
        buffer.append(&[0x01, 0x02, 0x03]).unwrap();
        let messages = buffer.extract_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], vec![0x0A, 0x00, 0x01, 0x02, 0x03]);
    }
    
    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(5, Duration::from_secs(1));
        
        // First 5 messages allowed
        for _ in 0..5 {
            assert!(matches!(limiter.check_rate(), RateLimitResult::Allowed));
        }
        
        // 6th message throttled
        assert!(matches!(limiter.check_rate(), RateLimitResult::Throttle));
    }
    
    #[test]
    fn test_rc4_cipher() {
        let mut cipher = Rc4Cipher::new();
        let mut data = b"Hello, World!".to_vec();
        let original = data.clone();
        
        // Encrypt
        cipher.encrypt(&mut data);
        assert_ne!(data, original);
        
        // Decrypt (need new cipher - RC4 is stateful)
        let mut cipher2 = Rc4Cipher::new();
        cipher2.encrypt(&mut data); // Encrypt again = decrypt
        assert_eq!(data, original);
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_connection_lifecycle() {
    let manager = ConnectionManager::new(100);
    
    // Create mock TCP stream
    let (client, server) = tokio::io::duplex(1024);
    let socket = TcpStream::from_std(server.into_std().unwrap()).unwrap();
    let addr = "127.0.0.1:12345".parse().unwrap();
    
    // Accept connection
    let conn_id = manager.accept_connection(socket, addr).await.unwrap();
    
    // Verify state
    let conn = manager.connections.get(&conn_id).unwrap();
    let conn = conn.lock().await;
    assert_eq!(conn.state, ConnectionState::Unauthenticated);
    
    // Simulate login
    // ...
}
```

## Performance Considerations

### Concurrency

- **One task per connection:** Each connection handled independently
- **Lock-free reads:** Use Arc<DashMap> for connection pool (no global lock)
- **Bounded channels:** Prevent memory exhaustion from slow clients

### Memory Management

```rust
// Limit buffer sizes
const MAX_MESSAGE_BUFFER: usize = 65536;  // 64KB per connection
const MAX_OUTBOUND_QUEUE: usize = 100;     // Max queued messages

// Drop slow clients
if outbound_queue.len() > MAX_OUTBOUND_QUEUE {
    log::warn!("Client {} is slow, disconnecting", conn_id);
    disconnect(conn_id).await;
}
```

### CPU Optimization

```rust
// Reuse buffers
let mut buffer = vec![0u8; 8192];
loop {
    let n = socket.read(&mut buffer).await?;
    // Process buffer[..n]
    // Buffer is reused, no reallocation
}

// Batch message sends
let mut batch = Vec::with_capacity(10);
while let Ok(msg) = outbound_rx.try_recv() {
    batch.push(msg);
    if batch.len() >= 10 {
        break;
    }
}
socket.write_vectored(&batch).await?;
```

## Summary

The Connection Manager provides:
- ✅ **TCP connection handling** with tokio async I/O
- ✅ **RC4 encryption/decryption** per connection
- ✅ **Message buffering** for incomplete TCP packets
- ✅ **Connection state machine** (unauthenticated → authenticated)
- ✅ **Rate limiting** to prevent spam and DDoS
- ✅ **IP-based connection limits** (max 3 per IP)
- ✅ **Graceful disconnect handling** with cleanup
- ✅ **Metrics and monitoring** for observability

**Key Design Decisions:**
- One tokio task per connection (simplifies state management)
- Arc<DashMap> for lock-free connection pool access
- Separate read/write tasks (full-duplex communication)
- Rate limiting at multiple levels (per-connection, per-IP, per-message-type)
- Fail-fast on protocol violations (disconnect immediately)

**Next:** See [`03-world-manager.md`](03-world-manager.md) for game world state management.
