# Protocol: TCP Connection & Encryption

## Overview

The Slime Online 2 client uses a custom TCP protocol with RC4 stream cipher encryption. This document specifies the exact connection handshake and encryption mechanism that must be replicated server-side.

## Connection Flow

### 1. Client Initiates Connection

```
Client --> TCP SYN --> Server (port 5555 default)
Client <-- TCP SYN-ACK <-- Server
Client --> TCP ACK --> Server
[Connection established]
```

**Client Code Reference:**
```gml
// From login_controller Create Event
global.server = tcpconnect(global.server_ip, global.tcp_port, true);
setnagle(global.server, true);  // Enable Nagle's algorithm
setsync(global.server, 1);      // Synchronous mode
```

**Implementation Requirements:**
- **Port:** Configurable, default 5555
- **Protocol:** TCP only (no UDP)
- **Mode:** Non-blocking server with async I/O
- **Nagle:** Client enables Nagle's algorithm (coalesce small packets)
- **Keepalive:** TCP keepalive recommended (2 hour timeout)

### 2. Encryption Setup

**CRITICAL:** The encryption keys are hardcoded in the client and cannot be changed.

```gml
// From rm_load.room.gmx initialization
global.decstring = 't54gz65u74njb6zg6'     // Server decrypts incoming with this
global.encstring = 'retrtz7jmijb5467n47'   // Server encrypts outgoing with this
```

**Key Direction:**
- **Client → Server:** Encrypted with `retrtz7jmijb5467n47`, server decrypts with same
- **Server → Client:** Encrypted with `t54gz65u74njb6zg6`, client decrypts with same

**Note:** This is backwards from typical crypto (different keys for each direction). The client uses `encstring` to encrypt outgoing messages and `decstring` to decrypt incoming messages. The server must reverse this.

### 3. First Message (Login)

Immediately after connection, client sends MSG_LOGIN:

```
Client:
  1. clearbuffer()
  2. writeushort(MSG_LOGIN)  // 10
  3. writestring(GameVersion)  // "0.106"
  4. writestring(username)
  5. writestring(password)
  6. writestring(mac_address)
  7. bufferencrypt(global.encstring)  // RC4 encrypt entire buffer
  8. sendmessage(global.server)

Server:
  1. receivemessage() -> read TCP bytes
  2. bufferdecrypt(global.decstring) -> RC4 decrypt
  3. readushort() -> get message type (10)
  4. Read login data
  5. Validate and respond
```

## RC4 Encryption Specification

### Algorithm Details

**RC4 (Rivest Cipher 4):**
- Stream cipher
- Variable key length (1-256 bytes)
- Stateful (must maintain cipher state)
- Same algorithm for encrypt/decrypt

**Key Scheduling Algorithm (KSA):**
```
S = [0..255]  // State array
j = 0

for i from 0 to 255:
    j = (j + S[i] + key[i mod key_length]) mod 256
    swap(S[i], S[j])
```

**Pseudo-Random Generation Algorithm (PRGA):**
```
i = 0
j = 0

for each byte in plaintext:
    i = (i + 1) mod 256
    j = (j + S[i]) mod 256
    swap(S[i], S[j])
    k = S[(S[i] + S[j]) mod 256]
    ciphertext_byte = plaintext_byte XOR k
```

### Rust Implementation

**Required Crate:**
```toml
[dependencies]
rc4 = "0.1"  # Minimal RC4 implementation
```

**Encryption Helper:**
```rust
use rc4::{Rc4, KeyInit, StreamCipher};

const CLIENT_ENCRYPT_KEY: &[u8] = b"retrtz7jmijb5467n47";
const CLIENT_DECRYPT_KEY: &[u8] = b"t54gz65u74njb6zg6";

pub fn decrypt_client_message(data: &mut [u8]) {
    let mut cipher = Rc4::new(CLIENT_ENCRYPT_KEY.into());
    cipher.apply_keystream(data);
}

pub fn encrypt_server_message(data: &mut [u8]) {
    let mut cipher = Rc4::new(CLIENT_DECRYPT_KEY.into());
    cipher.apply_keystream(data);
}
```

**Important:** RC4 must be re-initialized for each message. The client calls `bufferencrypt()` for each send and `bufferdecrypt()` for each receive, creating a new cipher state each time.

### Message Encryption Flow

**Receiving from Client:**
```
1. Read TCP bytes into buffer
2. RC4 decrypt entire buffer using CLIENT_ENCRYPT_KEY
3. Parse decrypted binary message
```

**Sending to Client:**
```
1. Build binary message in buffer
2. RC4 encrypt entire buffer using CLIENT_DECRYPT_KEY
3. Send encrypted bytes over TCP
```

## TCP Socket Management

### Server-Side Connection State

Each client connection requires:

```rust
pub struct ClientConnection {
    socket: TcpStream,
    addr: SocketAddr,
    player_id: Option<u16>,  // Set after successful login
    session_token: Option<Uuid>,
    last_activity: Instant,
    send_buffer: BytesMut,
    recv_buffer: BytesMut,
}
```

### Read/Write Patterns

**Reading Messages:**
```rust
async fn read_message(socket: &mut TcpStream) -> Result<Vec<u8>> {
    // Read message length or until natural boundary
    // Client uses variable-length messages with null-terminated strings
    // No explicit length header in protocol
    
    // Read available bytes
    let mut buffer = vec![0u8; 4096];
    let n = socket.read(&mut buffer).await?;
    buffer.truncate(n);
    
    // Decrypt
    decrypt_client_message(&mut buffer);
    
    Ok(buffer)
}
```

**Writing Messages:**
```rust
async fn send_message(socket: &mut TcpStream, data: Vec<u8>) -> Result<()> {
    let mut encrypted = data;
    encrypt_server_message(&mut encrypted);
    socket.write_all(&encrypted).await?;
    socket.flush().await?;
    Ok(())
}
```

### Connection Lifecycle

**1. Accept Phase:**
```
- Server accepts TCP connection
- Create ClientConnection struct
- Start message read loop
- Wait for MSG_LOGIN (must be first message)
```

**2. Authenticated Phase:**
```
- MSG_LOGIN validated
- player_id assigned
- Session token generated
- Normal message processing begins
```

**3. Active Phase:**
```
- Process messages from client
- Update last_activity timestamp
- Send responses and broadcasts
- Validate all input
```

**4. Disconnect Phase:**
```
- Client sends MSG_LOGOUT (graceful), OR
- TCP connection drops (ungraceful), OR
- Server timeout (no activity for 5 minutes)

Actions:
- Broadcast MSG_LOGOUT to room
- Save player state to database
- Remove from active sessions
- Close TCP socket
```

## Timeout & Keepalive

### Client-Side Ping

**Client sends MSG_PING every 30 seconds:**
```gml
// From obj_controller Alarm[0]
alarm[0] = 1800  // 30 seconds at 60 FPS

clearbuffer()
    writeushort(MSG_PING)
send_message()
```

**Server must respond:**
```
Server receives MSG_PING
Server sends MSG_PING back immediately
```

### Connection Timeout Detection

**Server should disconnect if:**
- No message received for 5 minutes (300 seconds)
- TCP socket error (broken pipe, connection reset)
- Client sends invalid/malformed message 3 times in a row

**Graceful Shutdown:**
```
1. Send MSG_SERVER_CLOSE to client
2. Wait 1 second for acknowledgment
3. Close TCP socket
4. Clean up session state
```

## Error Handling

### Connection Errors

**During Accept:**
- Too many connections: Reject new connection
- Invalid source IP: Check ban list, reject if banned
- Socket error: Log and continue accepting others

**During Read:**
- Connection reset: Clean disconnect, save state
- Timeout: Send warning, then disconnect if repeated
- Malformed data: Log, send error response, disconnect after 3 strikes

**During Write:**
- Connection closed: Clean up session immediately
- Write timeout: Force disconnect
- Buffer full: Log warning, may indicate slow client

### Encryption Errors

**Never happen in practice** because RC4 always succeeds (it's just XOR with keystream). However, validate decrypted data:

```rust
// After decryption
if buffer.len() < 2 {
    return Err("Message too short for type field");
}

let msg_type = u16::from_le_bytes([buffer[0], buffer[1]]);
if msg_type > 141 {
    return Err("Invalid message type");
}
```

## Connection Limits & DDoS Protection

### Per-IP Limits

```rust
const MAX_CONNECTIONS_PER_IP: usize = 3;
const CONNECTION_RATE_LIMIT: Duration = Duration::from_secs(1);
```

Track connections by source IP:
- Maximum 3 concurrent connections from same IP
- Maximum 1 new connection per second from same IP
- Temporary ban (10 minutes) if limits exceeded

### Global Limits

```rust
const MAX_TOTAL_CONNECTIONS: usize = 1000;
const MAX_UNAUTHENTICATED_CONNECTIONS: usize = 100;
const UNAUTHENTICATED_TIMEOUT: Duration = Duration::from_secs(30);
```

- Maximum 1000 total connections
- Maximum 100 unauthenticated connections
- Unauthenticated connections timeout after 30 seconds
- Oldest unauthenticated dropped first if limit reached

### SYN Flood Protection

**OS-Level:**
- Enable SYN cookies in Linux kernel
- Configure TCP backlog size
- Use connection tracking

**Application-Level:**
- Accept queue limited to 100
- Rapid accept loop to prevent queue saturation
- Monitor accept rate, alert if > 100/second

## Message Buffering

### Receive Buffer

Client may send partial messages or multiple messages in one TCP packet. Server must handle:

**Partial Message:**
```rust
// Accumulate bytes until complete message parsed
loop {
    socket.read(&mut recv_buffer).await?;
    
    if let Some(message) = try_parse_message(&recv_buffer) {
        process_message(message);
        recv_buffer.advance(message.len());
    } else {
        break; // Wait for more data
    }
}
```

**Multiple Messages:**
```rust
// Parse all complete messages from buffer
while let Some(message) = try_parse_message(&recv_buffer) {
    process_message(message);
    recv_buffer.advance(message.len());
}
```

### Send Buffer

**Batch small messages:**
```rust
// Instead of immediate flush for each message
send_buffer.extend_from_slice(&message);

// Flush when:
// 1. Buffer > 4KB
// 2. End of tick (every 16ms)
// 3. High-priority message (login response)
if send_buffer.len() > 4096 || is_tick_end || is_high_priority {
    socket.write_all(&send_buffer).await?;
    send_buffer.clear();
}
```

## Testing Connection Layer

### Unit Tests

```rust
#[test]
fn test_rc4_decrypt_login() {
    let encrypted = hex::decode("...").unwrap();
    let mut data = encrypted.clone();
    decrypt_client_message(&mut data);
    
    assert_eq!(&data[0..2], &10u16.to_le_bytes());  // MSG_LOGIN
}

#[test]
fn test_rc4_roundtrip() {
    let original = b"test message";
    let mut encrypted = original.to_vec();
    encrypt_server_message(&mut encrypted);
    
    let mut decrypted = encrypted.clone();
    decrypt_client_message(&mut decrypted);
    
    assert_eq!(&decrypted, original);
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_connection_lifecycle() {
    let server = start_test_server().await;
    let mut client = TcpStream::connect("127.0.0.1:5555").await.unwrap();
    
    // Send login
    send_encrypted_message(&mut client, create_login_message()).await;
    
    // Receive response
    let response = read_encrypted_message(&mut client).await;
    assert_eq!(parse_message_type(&response), MSG_LOGIN);
    
    // Send ping
    send_encrypted_message(&mut client, create_ping_message()).await;
    let response = read_encrypted_message(&mut client).await;
    assert_eq!(parse_message_type(&response), MSG_PING);
    
    // Disconnect
    drop(client);
    
    // Verify cleanup
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(server.active_connections(), 0);
}
```

### Load Tests

```bash
# Simulate 500 concurrent clients
for i in {1..500}; do
    (
        echo "Connecting client $i"
        nc localhost 5555 &
    )
done

# Monitor server metrics
watch -n 1 'ss -tn | grep :5555 | wc -l'
```

## Implementation Checklist

Connection layer implementation requirements:

- [ ] TCP server listening on configurable port
- [ ] RC4 decryption of incoming messages with `retrtz7jmijb5467n47`
- [ ] RC4 encryption of outgoing messages with `t54gz65u74njb6zg6`
- [ ] Per-connection receive buffer with partial message handling
- [ ] Per-connection send buffer with batching
- [ ] Connection state tracking (unauthenticated/authenticated)
- [ ] Last activity timestamp tracking
- [ ] 5-minute idle timeout
- [ ] Graceful disconnect on MSG_LOGOUT
- [ ] Force disconnect on timeout/error
- [ ] Per-IP connection limit (3 max)
- [ ] Per-IP rate limit (1/second)
- [ ] Global connection limit (1000 max)
- [ ] Unauthenticated connection limit (100 max)
- [ ] 30-second unauthenticated timeout
- [ ] Ping/pong response (MSG_PING)
- [ ] Connection metrics (active count, accept rate)
- [ ] Error logging (connection errors, malformed messages)
- [ ] Integration tests with real client

## Security Notes

### What Server Can Control

✅ **Server-side validation** - Validate all input, never trust client  
✅ **Rate limiting** - Limit message frequency per connection  
✅ **Session management** - Force disconnect suspicious clients  
✅ **IP banning** - Block malicious IPs at connection time  
✅ **Resource limits** - Prevent memory exhaustion  

### What Server Cannot Control

❌ **Encryption algorithm** - Must use RC4 (client hardcoded)  
❌ **Encryption keys** - Must use hardcoded keys  
❌ **Message format** - Must match binary protocol exactly  
❌ **Protocol version** - Must accept "0.106" version string  

### Threat Model

**Passive Eavesdropping:**
- Attacker can decrypt all traffic (keys are public)
- Mitigate: Warn users not to reuse passwords

**Man-in-the-Middle:**
- Attacker can intercept and modify traffic
- Mitigate: None possible without client changes
- Recommendation: Document in terms of service

**Replay Attacks:**
- Attacker can replay captured login messages
- Mitigate: Server-side session tokens, timestamp validation

**DDoS Attacks:**
- Attacker floods server with connections
- Mitigate: Connection limits, rate limiting, IP bans

---

**Next:** Read `02-message-format.md` to understand binary message structure.
