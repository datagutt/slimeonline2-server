# Protocol: Binary Message Format

## Overview

All messages between client and server use a binary format with little-endian byte ordering. This document specifies the exact binary layout and parsing rules.

## Core Data Types

### Primitive Types

| Type | Size | Range | Rust Type | Notes |
|------|------|-------|-----------|-------|
| `byte` | 1 byte | 0 to 255 | `u8` | Unsigned 8-bit integer |
| `ushort` | 2 bytes | 0 to 65,535 | `u16` | Unsigned 16-bit integer, little-endian |
| `uint` | 4 bytes | 0 to 4,294,967,295 | `u32` | Unsigned 32-bit integer, little-endian |
| `short` | 2 bytes | -32,768 to 32,767 | `i16` | Signed 16-bit integer, little-endian (rarely used) |
| `int` | 4 bytes | -2,147,483,648 to 2,147,483,647 | `i32` | Signed 32-bit integer, little-endian (rarely used) |
| `float` | 4 bytes | IEEE 754 | `f32` | 32-bit floating point (rarely used) |
| `double` | 8 bytes | IEEE 754 | `f64` | 64-bit floating point (unused) |
| `string` | variable | null-terminated | `String` | UTF-8 string ending with 0x00 |

**Byte Order:** Little-endian for all multi-byte types

### String Format

Strings are **null-terminated** (C-style strings):

```
Example: "Hello"
Bytes: 48 65 6C 6C 6F 00
       H  e  l  l  o  \0
```

**Important:**
- Empty string is just `00` (single null byte)
- Maximum practical length: 255 characters (GameMaker limitation)
- Encoding: UTF-8 (though client likely uses ASCII/Latin-1)
- Server must validate: no embedded nulls, reasonable length

## Message Structure

### Standard Message Format

```
┌─────────────────────────────────────────────┐
│ Message Type (ushort, 2 bytes)              │
├─────────────────────────────────────────────┤
│ Payload (variable length)                    │
│   - Field 1 (type-specific)                  │
│   - Field 2 (type-specific)                  │
│   - ...                                      │
└─────────────────────────────────────────────┘
```

**Every message starts with a 2-byte message type:**
```rust
let message_type = u16::from_le_bytes([buffer[0], buffer[1]]);
```

**Example: MSG_CHAT (17)**

Client sends:
```gml
clearbuffer()
writeushort(MSG_CHAT)     // 17 = 0x0011
writestring("Hello!")      // "Hello!" + null
```

Binary representation:
```
Offset | Hex Value | Meaning
-------|-----------|------------------
0x00   | 11 00     | Message type 17 (little-endian)
0x02   | 48        | 'H'
0x03   | 65        | 'e'
0x04   | 6C        | 'l'
0x05   | 6C        | 'l'
0x06   | 6F        | 'o'
0x07   | 21        | '!'
0x08   | 00        | Null terminator
```

## Parsing Messages

### Reader Implementation

Create a binary reader that tracks position:

```rust
pub struct MessageReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> MessageReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
    
    pub fn read_u8(&mut self) -> Result<u8> {
        if self.pos >= self.data.len() {
            return Err("Unexpected end of message".into());
        }
        let value = self.data[self.pos];
        self.pos += 1;
        Ok(value)
    }
    
    pub fn read_u16(&mut self) -> Result<u16> {
        if self.pos + 2 > self.data.len() {
            return Err("Unexpected end of message".into());
        }
        let bytes = [self.data[self.pos], self.data[self.pos + 1]];
        self.pos += 2;
        Ok(u16::from_le_bytes(bytes))
    }
    
    pub fn read_u32(&mut self) -> Result<u32> {
        if self.pos + 4 > self.data.len() {
            return Err("Unexpected end of message".into());
        }
        let bytes = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ];
        self.pos += 4;
        Ok(u32::from_le_bytes(bytes))
    }
    
    pub fn read_string(&mut self) -> Result<String> {
        let start = self.pos;
        
        // Find null terminator
        while self.pos < self.data.len() && self.data[self.pos] != 0 {
            self.pos += 1;
        }
        
        if self.pos >= self.data.len() {
            return Err("String not null-terminated".into());
        }
        
        let string_bytes = &self.data[start..self.pos];
        self.pos += 1; // Skip null terminator
        
        // Validate UTF-8
        String::from_utf8(string_bytes.to_vec())
            .map_err(|_| "Invalid UTF-8 in string".into())
    }
    
    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }
}
```

### Writer Implementation

Create a binary writer that builds messages:

```rust
pub struct MessageWriter {
    buffer: Vec<u8>,
}

impl MessageWriter {
    pub fn new() -> Self {
        Self { buffer: Vec::with_capacity(256) }
    }
    
    pub fn write_u8(&mut self, value: u8) {
        self.buffer.push(value);
    }
    
    pub fn write_u16(&mut self, value: u16) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }
    
    pub fn write_u32(&mut self, value: u32) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }
    
    pub fn write_string(&mut self, value: &str) {
        self.buffer.extend_from_slice(value.as_bytes());
        self.buffer.push(0); // Null terminator
    }
    
    pub fn into_bytes(self) -> Vec<u8> {
        self.buffer
    }
    
    pub fn len(&self) -> usize {
        self.buffer.len()
    }
}
```

## Message Parsing Example

### Parsing MSG_LOGIN (10)

**Client sends:**
```gml
writeushort(MSG_LOGIN)        // 10
writestring(GameVersion)      // "0.106"
writestring(username)         // e.g., "Player1"
writestring(password)         // e.g., "password123"
writestring(mac_address)      // e.g., "00-11-22-33-44-55"
```

**Server parses:**
```rust
fn parse_login(data: &[u8]) -> Result<LoginMessage> {
    let mut reader = MessageReader::new(data);
    
    // First 2 bytes already consumed to determine message type
    let version = reader.read_string()?;
    let username = reader.read_string()?;
    let password = reader.read_string()?;
    let mac_address = reader.read_string()?;
    
    // Validate
    if version != "0.106" {
        return Err("Invalid client version".into());
    }
    
    if username.len() < 3 || username.len() > 20 {
        return Err("Invalid username length".into());
    }
    
    Ok(LoginMessage {
        version,
        username,
        password,
        mac_address,
    })
}
```

### Building MSG_LOGIN Response (Success)

**Server sends:**
```gml
// case 1: Successful login
writeushort(MSG_LOGIN)       // 10
writebyte(1)                 // Success case
writeushort(player_id)       // e.g., 42
writeuint(server_time)       // Unix timestamp
writestring(motd)            // "Welcome!"
writebyte(day_of_week)       // 1-7
writebyte(hour)              // 0-23
writebyte(minute)            // 0-59
writestring(username)        // Echo back username
writeushort(spawn_x)         // e.g., 160
writeushort(spawn_y)         // e.g., 120
writeushort(spawn_room)      // e.g., 5
writeushort(body_id)         // Outfit
writeushort(acs1_id)         // Accessory 1
writeushort(acs2_id)         // Accessory 2
writeuint(points)            // Currency
writebyte(signature)         // Permission flag
writeushort(quest_id)        // Active quest
writebyte(quest_step)        // Quest progress
writeushort(trees_planted)   // Stat
writeushort(objects_built)   // Stat
// ... 5 emotes (bytes)
// ... 9 outfits (ushorts)
// ... 9 accessories (ushorts)
// ... 9 items (ushorts)
// ... 9 tools (bytes)
```

**Server builds:**
```rust
fn build_login_success(player: &Player) -> Vec<u8> {
    let mut writer = MessageWriter::new();
    
    writer.write_u16(MSG_LOGIN);
    writer.write_u8(1); // Success case
    writer.write_u16(player.id);
    writer.write_u32(get_server_timestamp());
    writer.write_string(&get_motd());
    
    let (day, hour, minute) = get_game_time();
    writer.write_u8(day);
    writer.write_u8(hour);
    writer.write_u8(minute);
    
    writer.write_string(&player.username);
    writer.write_u16(player.spawn_x);
    writer.write_u16(player.spawn_y);
    writer.write_u16(player.spawn_room);
    writer.write_u16(player.body_id);
    writer.write_u16(player.acs1_id);
    writer.write_u16(player.acs2_id);
    writer.write_u32(player.points);
    writer.write_u8(player.has_signature as u8);
    writer.write_u16(player.quest_id);
    writer.write_u8(player.quest_step);
    writer.write_u16(player.trees_planted);
    writer.write_u16(player.objects_built);
    
    // 5 emote slots
    for i in 0..5 {
        writer.write_u8(player.emotes[i]);
    }
    
    // 9 outfit slots
    for i in 0..9 {
        writer.write_u16(player.outfits[i]);
    }
    
    // 9 accessory slots
    for i in 0..9 {
        writer.write_u16(player.accessories[i]);
    }
    
    // 9 item slots
    for i in 0..9 {
        writer.write_u16(player.items[i]);
    }
    
    // 9 tool slots
    for i in 0..9 {
        writer.write_u8(player.tools[i]);
    }
    
    writer.into_bytes()
}
```

## Variable-Length Message Handling

### Challenge: No Explicit Length Header

The protocol does **not** include a message length field. Messages are variable length due to strings.

### Solution: Parse Until Complete

```rust
// Keep accumulating bytes until a complete message can be parsed
pub struct MessageBuffer {
    buffer: BytesMut,
}

impl MessageBuffer {
    pub fn feed(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }
    
    pub fn try_parse(&mut self) -> Result<Option<Vec<u8>>> {
        if self.buffer.len() < 2 {
            return Ok(None); // Need at least message type
        }
        
        let msg_type = u16::from_le_bytes([self.buffer[0], self.buffer[1]]);
        
        // Try to parse based on known structure
        // This requires knowing the expected format for each message type
        match msg_type {
            MSG_LOGIN => self.try_parse_login(),
            MSG_CHAT => self.try_parse_chat(),
            MSG_PING => self.try_parse_ping(),
            // ... handle all message types
            _ => Err("Unknown message type".into()),
        }
    }
    
    fn try_parse_login(&mut self) -> Result<Option<Vec<u8>>> {
        let mut pos = 2; // Skip message type
        
        // Try to parse all 4 strings
        for _ in 0..4 {
            match find_null_terminator(&self.buffer[pos..]) {
                Some(offset) => pos += offset + 1,
                None => return Ok(None), // Incomplete, need more data
            }
        }
        
        // Complete message found
        let message = self.buffer.split_to(pos).to_vec();
        Ok(Some(message))
    }
}

fn find_null_terminator(data: &[u8]) -> Option<usize> {
    data.iter().position(|&b| b == 0)
}
```

### Fixed-Length Messages

Some messages have no strings and are fixed-length:

```rust
const MSG_LENGTHS: &[(u16, usize)] = &[
    (MSG_PING, 2),           // Just message type
    (MSG_LOGOUT, 2),         // Just message type
    (MSG_CANMOVE_TRUE, 2),   // Just message type
    // ... etc
];

fn get_message_length(msg_type: u16) -> Option<usize> {
    MSG_LENGTHS.iter()
        .find(|(t, _)| *t == msg_type)
        .map(|(_, len)| *len)
}
```

## Validation Rules

### Message Type Validation

```rust
pub fn validate_message_type(msg_type: u16) -> Result<()> {
    if msg_type == 0 || msg_type > 141 {
        return Err("Invalid message type");
    }
    
    // Check for unused/reserved message types
    match msg_type {
        3 | 4 | 8 | 20 => Err("Reserved message type"),
        _ => Ok(()),
    }
}
```

### String Validation

```rust
pub fn validate_string(s: &str, max_len: usize, field_name: &str) -> Result<()> {
    if s.len() > max_len {
        return Err(format!("{} exceeds max length {}", field_name, max_len));
    }
    
    // Check for control characters (except null, which ends the string)
    if s.bytes().any(|b| b < 32 && b != 0) {
        return Err(format!("{} contains control characters", field_name));
    }
    
    Ok(())
}
```

### Numeric Range Validation

```rust
pub fn validate_room_id(room_id: u16) -> Result<()> {
    if room_id > MAX_ROOM_ID {
        return Err("Invalid room ID");
    }
    Ok(())
}

pub fn validate_position(x: u16, y: u16, room_id: u16) -> Result<()> {
    let room_bounds = get_room_bounds(room_id)?;
    
    if x > room_bounds.width || y > room_bounds.height {
        return Err("Position out of room bounds");
    }
    
    Ok(())
}
```

## Common Message Patterns

### Pattern 1: Simple Request-Response

```
Client: MSG_PING (just message type, 2 bytes)
Server: MSG_PING (echo back, 2 bytes)
```

### Pattern 2: Request with Data

```
Client: MSG_CHAT
  writeushort(MSG_CHAT)
  writestring(message_text)

Server: (no direct response, broadcasts to room)
```

### Pattern 3: Multi-Case Response

```
Client: MSG_LOGIN (credentials)

Server: MSG_LOGIN
  writeushort(MSG_LOGIN)
  writebyte(response_case)
  
  if case == 1: (success)
    ... write full player data
  elif case == 2: (account doesn't exist)
    (no additional data)
  elif case == 3: (wrong password)
    (no additional data)
  ...
```

### Pattern 4: Broadcast to Room

```
Player A moves:

Client A -> Server: MSG_MOVE_PLAYER
  writeushort(MSG_MOVE_PLAYER)
  writebyte(direction)
  writeushort(x)
  writeushort(y)

Server -> All clients in room (except A):
  writeushort(MSG_MOVE_PLAYER)
  writeushort(player_a_id)
  writebyte(direction)
  writeushort(x)
  writeushort(y)
```

## Error Handling

### Malformed Messages

**Too Short:**
```rust
if buffer.len() < 2 {
    return Err(ProtocolError::MessageTooShort);
}
```

**Missing Null Terminator:**
```rust
if !buffer.contains(&0) {
    return Err(ProtocolError::StringNotTerminated);
}
```

**Invalid UTF-8:**
```rust
match String::from_utf8(bytes) {
    Ok(s) => s,
    Err(_) => return Err(ProtocolError::InvalidUtf8),
}
```

**Unexpected End:**
```rust
if reader.remaining() < expected_bytes {
    return Err(ProtocolError::UnexpectedEnd);
}
```

### Server Response to Errors

**During Handshake (pre-login):**
- Disconnect immediately
- Log error with IP address
- Increment failed_connection_attempts counter

**After Login:**
- Send error response if appropriate
- Increment strike counter (disconnect after 3 strikes)
- Log error with player ID and username

## Testing

### Unit Tests

```rust
#[test]
fn test_write_read_roundtrip() {
    let mut writer = MessageWriter::new();
    writer.write_u16(17);
    writer.write_string("Test");
    
    let bytes = writer.into_bytes();
    let mut reader = MessageReader::new(&bytes);
    
    assert_eq!(reader.read_u16().unwrap(), 17);
    assert_eq!(reader.read_string().unwrap(), "Test");
    assert_eq!(reader.remaining(), 0);
}

#[test]
fn test_null_terminated_string() {
    let data = b"Hello\x00World\x00";
    let mut reader = MessageReader::new(data);
    
    assert_eq!(reader.read_string().unwrap(), "Hello");
    assert_eq!(reader.read_string().unwrap(), "World");
}

#[test]
fn test_little_endian() {
    let mut writer = MessageWriter::new();
    writer.write_u16(0x1234);
    writer.write_u32(0x12345678);
    
    let bytes = writer.into_bytes();
    
    // Little-endian: least significant byte first
    assert_eq!(bytes[0], 0x34);
    assert_eq!(bytes[1], 0x12);
    assert_eq!(bytes[2], 0x78);
    assert_eq!(bytes[3], 0x56);
    assert_eq!(bytes[4], 0x34);
    assert_eq!(bytes[5], 0x12);
}
```

### Fuzzing

```rust
#[test]
fn fuzz_message_parser() {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    for _ in 0..10000 {
        let len = rng.gen_range(0..1024);
        let mut data: Vec<u8> = (0..len).map(|_| rng.gen()).collect();
        
        // Try to parse random data
        // Should never panic, only return errors
        let _ = parse_message(&data);
    }
}
```

## Implementation Checklist

- [ ] MessageReader with all primitive types
- [ ] MessageWriter with all primitive types
- [ ] Null-terminated string reading
- [ ] Null-terminated string writing
- [ ] Little-endian byte order verification
- [ ] Variable-length message buffering
- [ ] Message type validation
- [ ] String length validation
- [ ] UTF-8 validation
- [ ] Numeric range validation
- [ ] Error handling for malformed messages
- [ ] Unit tests for all data types
- [ ] Round-trip serialization tests
- [ ] Fuzzing tests

---

**Next:** Read `03-authentication.md` for login/registration protocol.
