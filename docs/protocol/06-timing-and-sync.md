# Timing and Synchronization

**Document Status:** Complete  
**Last Updated:** 2024-01-08  
**Related:** [`01-connection.md`](01-connection.md), [`02-message-format.md`](02-message-format.md)

## Overview

This document specifies the timing and synchronization mechanisms for Slime Online 2. The protocol uses **keepalive pings** to detect disconnections and **server time synchronization** to coordinate in-game events.

**Key Mechanisms:**
- **Keepalive:** Server sends MSG_PING every 30 seconds
- **Timeout:** Client disconnected if no MSG_PING received for 60 seconds
- **Time Sync:** Server broadcasts in-game time every minute
- **Ping Measurement:** Client can request latency measurement

## Keepalive Protocol

### Server → Client: Keepalive Ping

The server sends MSG_PING every **30 seconds** (1800 frames at 60 FPS):

```
MSG_PING (9)
// No payload
```

**Client Response:**
```
MSG_PING (9)
// No payload (echo back)
```

### Timing

```rust
pub const PING_INTERVAL: Duration = Duration::from_secs(30);  // 1800 frames
pub const PING_TIMEOUT: Duration = Duration::from_secs(60);   // 2x ping interval
```

**Client Behavior (from case_msg_ping.gml):**
```javascript
// When MSG_PING received:
alarm[0] = 1800;        // Reset 30-second timer
global.canping = true;  // Allow manual ping

// Send response
clearbuffer();
writeushort(MSG_PING);
send_message();
```

**Client Timeout Detection:**
```javascript
// In alarm[0] event (fires when timer reaches 0):
if (alarm[0] == 0) {
    // 30 seconds elapsed without MSG_PING
    // Assume connection lost
    disconnect();
}
```

### Server Implementation

```rust
pub struct PingManager {
    ping_interval: Duration,
    ping_timeout: Duration,
}

impl PingManager {
    pub fn new() -> Self {
        Self {
            ping_interval: Duration::from_secs(30),
            ping_timeout: Duration::from_secs(60),
        }
    }
    
    /// Send keepalive ping to player
    pub async fn send_ping(&self, player: &mut Player) -> Result<(), ServerError> {
        let mut msg = MessageWriter::new();
        msg.write_u16(MSG_PING);
        
        player.send_message(&msg.build()).await?;
        player.last_ping_sent = Instant::now();
        
        Ok(())
    }
    
    /// Check if player should receive ping
    pub fn needs_ping(&self, player: &Player) -> bool {
        player.last_ping_sent.elapsed() >= self.ping_interval
    }
    
    /// Check if player timed out
    pub fn is_timed_out(&self, player: &Player) -> bool {
        player.last_ping_received.elapsed() >= self.ping_timeout
    }
}

/// Background task to send pings
pub async fn ping_loop(server: Arc<Server>) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    
    loop {
        interval.tick().await;
        
        let players = server.get_all_players();
        
        for player_id in players {
            let player = server.get_player_mut(player_id);
            
            // Send ping if interval elapsed
            if server.ping_manager.needs_ping(&player) {
                if let Err(e) = server.ping_manager.send_ping(&mut player).await {
                    log::error!("Failed to send ping to player {}: {}", player_id, e);
                }
            }
            
            // Check for timeout
            if server.ping_manager.is_timed_out(&player) {
                log::warn!("Player {} timed out (no pong for 60s)", player_id);
                server.disconnect_player(player_id, "Ping timeout").await;
            }
        }
    }
}
```

### Handling Ping Response

```rust
pub async fn handle_ping_response(
    server: &Server,
    player_id: u16,
) -> Result<(), ServerError> {
    let player = server.get_player_mut(player_id)?;
    player.last_ping_received = Instant::now();
    
    // Calculate RTT (round-trip time)
    let rtt = player.last_ping_received - player.last_ping_sent;
    player.latency = rtt;
    
    log::trace!("Player {} ping: {}ms", player_id, rtt.as_millis());
    
    Ok(())
}
```

## Latency Measurement

### Client → Server: Request Ping Measurement

Client can manually request latency measurement:

```
MSG_PING_REQ (117)
// No payload
```

**Purpose:** Player wants to see their ping/latency in UI.

### Server → Client: Ping Result

```
MSG_PING_REQ (117)
// No payload (echo back immediately)
```

**Client Behavior (from case_msg_ping_req.gml):**
```javascript
// When MSG_PING_REQ received (response):
global.pingcountnow = false;
global.ping = global.pingcount;  // Save measured latency
```

**Client Measurement Logic:**
```javascript
// Before sending request:
global.pingcountnow = true;
global.pingcount = 0;

// Every frame while waiting:
if (global.pingcountnow) {
    global.pingcount += 1;  // Increment frame counter
}

// When response received:
// global.pingcount now contains frames elapsed
// At 60 FPS: ping_ms = (pingcount / 60) * 1000
```

### Server Implementation

```rust
pub async fn handle_ping_request(
    server: &Server,
    player_id: u16,
) -> Result<(), ServerError> {
    // Immediately echo back MSG_PING_REQ
    let mut msg = MessageWriter::new();
    msg.write_u16(MSG_PING_REQ);
    
    let player = server.get_player(player_id)?;
    player.send_message(&msg.build()).await?;
    
    log::trace!("Player {} requested ping measurement", player_id);
    
    Ok(())
}
```

### Ping Display Calculation

```rust
/// Calculate ping in milliseconds from frame count
pub fn frames_to_ping_ms(frame_count: u32, fps: u32) -> u32 {
    (frame_count * 1000) / fps
}

// Example: 6 frames at 60 FPS = (6 * 1000) / 60 = 100ms
```

## Server Time Synchronization

The server maintains an **in-game clock** separate from real time. This clock is synchronized to all clients every minute.

### In-Game Time Format

```rust
pub struct GameTime {
    pub hour: u8,      // 0-23
    pub minute: u8,    // 0-59
    pub day: u8,       // 1-7 (1=Sun, 2=Mon, ..., 7=Sat)
}

impl GameTime {
    pub fn new() -> Self {
        Self {
            hour: 0,
            minute: 0,
            day: 1, // Sunday
        }
    }
    
    /// Advance time by 1 minute
    pub fn tick_minute(&mut self) {
        self.minute += 1;
        
        if self.minute >= 60 {
            self.minute = 0;
            self.hour += 1;
            
            if self.hour >= 24 {
                self.hour = 0;
                self.day += 1;
                
                if self.day > 7 {
                    self.day = 1; // Wrap to Sunday
                }
            }
        }
    }
}
```

### Server → Client: Time Update

Sent **every minute** (3600 frames at 60 FPS):

```
MSG_TIME (21)
├─ update_type: u8    // 1 = hour, 2 = minute, 3 = day change
└─ value: u8          // New hour/minute/day value
```

**Update Types:**

```rust
pub enum TimeUpdateType {
    Hour = 1,      // Hour changed
    Minute = 2,    // Minute changed
    DayChange = 3, // Day changed (includes new day)
}
```

### Message Examples

**Minute Update (every minute):**
```
[00 15]          // MSG_TIME (21)
[02]             // update_type = 2 (Minute)
[1E]             // minute = 30
```

**Hour Update (every hour):**
```
[00 15]          // MSG_TIME (21)
[01]             // update_type = 1 (Hour)
[0E]             // hour = 14 (2 PM)
```

**Day Change (at midnight):**
```
[00 15]          // MSG_TIME (21)
[03]             // update_type = 3 (DayChange)
[02]             // day = 2 (Monday)
```

### Client Behavior (from case_msg_time.gml)

```javascript
switch (update_type) {
    case 1: // Hour update
        global.hour = readbyte();
        
        // Change cursor based on time
        if (global.hour >= 20 || global.hour <= 8) {
            cursor_sprite = spr_cursor_sleep;  // Night cursor
        } else {
            cursor_sprite = spr_cursor_awake;  // Day cursor
        }
        break;
    
    case 2: // Minute update
        global.minute = readbyte();
        break;
    
    case 3: // Day change
        global.hour = 0;
        global.day = readbyte();
        
        // Convert day number to string
        switch(global.day) {
            case 1: global.day = 'Sun.'; break;
            case 2: global.day = 'Mon.'; break;
            case 3: global.day = 'Tue.'; break;
            case 4: global.day = 'Wed.'; break;
            case 5: global.day = 'Thu.'; break;
            case 6: global.day = 'Fri.'; break;
            case 7: global.day = 'Sat.'; break;
        }
        break;
}

// Update clock objects in world
with(obj_clock_stand) {
    alarm[0] = 1; // Trigger clock update
}
```

### Server Implementation

```rust
pub struct TimeManager {
    game_time: GameTime,
    last_update: Instant,
    update_interval: Duration,
}

impl TimeManager {
    pub fn new() -> Self {
        Self {
            game_time: GameTime::new(),
            last_update: Instant::now(),
            update_interval: Duration::from_secs(60), // 1 minute real time
        }
    }
    
    /// Check if time should update
    pub fn should_update(&self) -> bool {
        self.last_update.elapsed() >= self.update_interval
    }
    
    /// Update game time and return update message
    pub fn update(&mut self) -> Option<(u8, u8)> {
        if !self.should_update() {
            return None;
        }
        
        self.last_update = Instant::now();
        
        let old_hour = self.game_time.hour;
        let old_day = self.game_time.day;
        
        self.game_time.tick_minute();
        
        // Determine update type
        if self.game_time.day != old_day {
            // Day changed
            Some((3, self.game_time.day))
        } else if self.game_time.hour != old_hour {
            // Hour changed
            Some((1, self.game_time.hour))
        } else {
            // Minute changed
            Some((2, self.game_time.minute))
        }
    }
    
    /// Broadcast time update to all players
    pub async fn broadcast_update(&mut self, server: &Server) -> Result<(), ServerError> {
        if let Some((update_type, value)) = self.update() {
            let mut msg = MessageWriter::new();
            msg.write_u16(MSG_TIME);
            msg.write_u8(update_type);
            msg.write_u8(value);
            
            server.broadcast_to_all(&msg.build()).await?;
            
            log::debug!("Broadcast time update: type={}, value={}", update_type, value);
        }
        
        Ok(())
    }
}

/// Background task for time synchronization
pub async fn time_sync_loop(server: Arc<Server>) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    
    loop {
        interval.tick().await;
        
        if let Err(e) = server.time_manager.broadcast_update(&server).await {
            log::error!("Failed to broadcast time update: {}", e);
        }
    }
}
```

### Initial Time Sync

When a player logs in, send **current time** using MSG_TIME_UPDATE (see next section for details):

```rust
pub async fn send_initial_time(
    server: &Server,
    player: &Player,
) -> Result<(), ServerError> {
    // Use MSG_TIME_UPDATE for initial sync (more efficient)
    server.time_manager.send_full_time_sync(player).await?;
    
    Ok(())
}
```

## MSG_TIME_UPDATE - Full Time Sync

In addition to incremental updates via MSG_TIME, the server can send a **full time snapshot** using MSG_TIME_UPDATE (74).

### Server → Client: Complete Time Update

```
MSG_TIME_UPDATE (74)
├─ day: u8       // 1-7 (1=Sun, 2=Mon, ..., 7=Sat)
├─ hour: u8      // 0-23
└─ minute: u8    // 0-59
```

**Purpose:** Send complete time state to catch up player after disconnect or initial login.

**Example:**
```
[00 4A]          // MSG_TIME_UPDATE (74)
[03]             // day = 3 (Tuesday)
[0E]             // hour = 14 (2 PM)
[1E]             // minute = 30
```

### Client Behavior (from case_msg_time_update.gml)

```javascript
// Server sends updated time to make up for lost minutes/hours
_day = readbyte();
global.hour = readbyte();
global.minute = readbyte();

// Convert day number to string
switch(_day) {
    case 1: global.day = 'Sun.'; break;
    case 2: global.day = 'Mon.'; break;
    case 3: global.day = 'Tue.'; break;
    case 4: global.day = 'Wed.'; break;
    case 5: global.day = 'Thu.'; break;
    case 6: global.day = 'Fri.'; break;
    case 7: global.day = 'Sat.'; break;
}
```

### When to Use Each Message

**MSG_TIME (21) - Incremental Updates:**
- Sent every minute during normal operation
- Only sends what changed (hour OR minute OR day)
- Lower bandwidth (3 bytes)
- Used for ongoing synchronization

**MSG_TIME_UPDATE (74) - Full Sync:**
- Sent when player first logs in
- Sent after server time jumps (admin command, server restart)
- Sent to catch up disconnected players
- Higher bandwidth (5 bytes) but complete state
- Used for initial sync or recovery

### Server Implementation

```rust
impl TimeManager {
    /// Send complete time state to player (used at login)
    pub async fn send_full_time_sync(
        &self,
        player: &Player,
    ) -> Result<(), ServerError> {
        let mut msg = MessageWriter::new();
        msg.write_u16(MSG_TIME_UPDATE);
        msg.write_u8(self.game_time.day);
        msg.write_u8(self.game_time.hour);
        msg.write_u8(self.game_time.minute);
        
        player.send_message(&msg.build()).await?;
        
        log::debug!(
            "Sent full time sync to player {}: {:?}",
            player.id,
            self.game_time
        );
        
        Ok(())
    }
}
```

### Comparison: MSG_TIME vs MSG_TIME_UPDATE

| Feature | MSG_TIME (21) | MSG_TIME_UPDATE (74) |
|---------|---------------|----------------------|
| **Size** | 3 bytes | 5 bytes |
| **Content** | Single value (hour OR minute OR day) | Complete state (day + hour + minute) |
| **Frequency** | Every minute (incremental) | On login / time jump |
| **Use Case** | Ongoing sync | Initial sync / recovery |
| **Bandwidth** | Lower | Higher |
| **Complexity** | Requires state tracking | Self-contained |

## Clock Speed Configuration

### Real-Time vs. Accelerated Time

You can configure how fast in-game time passes:

```rust
pub enum ClockSpeed {
    RealTime,           // 1 minute real = 1 minute game
    Accelerated(u32),   // 1 minute real = N minutes game
}

impl TimeManager {
    pub fn new_with_speed(speed: ClockSpeed) -> Self {
        let update_interval = match speed {
            ClockSpeed::RealTime => Duration::from_secs(60),
            ClockSpeed::Accelerated(n) => Duration::from_secs(60 / n),
        };
        
        Self {
            game_time: GameTime::new(),
            last_update: Instant::now(),
            update_interval,
        }
    }
}

// Examples:
// RealTime: 1 real minute = 1 game minute (60s interval)
// Accelerated(2): 1 real minute = 2 game minutes (30s interval)
// Accelerated(60): 1 real minute = 1 game hour (1s interval)
```

## Player State Tracking

### Connection State

```rust
pub struct Player {
    // ... other fields ...
    
    // Ping/keepalive
    pub last_ping_sent: Instant,
    pub last_ping_received: Instant,
    pub latency: Duration,
    
    // Connection health
    pub connected_at: Instant,
    pub last_message_time: Instant,
}

impl Player {
    pub fn update_activity(&mut self) {
        self.last_message_time = Instant::now();
    }
    
    pub fn is_idle(&self, threshold: Duration) -> bool {
        self.last_message_time.elapsed() > threshold
    }
    
    pub fn connection_duration(&self) -> Duration {
        self.connected_at.elapsed()
    }
}
```

## Idle Detection

Detect idle players (not sending messages):

```rust
pub struct IdleDetector {
    idle_threshold: Duration,
    kick_threshold: Duration,
}

impl IdleDetector {
    pub fn new() -> Self {
        Self {
            idle_threshold: Duration::from_secs(300),  // 5 minutes
            kick_threshold: Duration::from_secs(1800), // 30 minutes
        }
    }
    
    pub fn check_idle(&self, player: &Player) -> IdleStatus {
        let idle_time = player.last_message_time.elapsed();
        
        if idle_time >= self.kick_threshold {
            IdleStatus::KickRequired
        } else if idle_time >= self.idle_threshold {
            IdleStatus::Idle
        } else {
            IdleStatus::Active
        }
    }
}

pub enum IdleStatus {
    Active,
    Idle,
    KickRequired,
}
```

## Performance Monitoring

### Server Performance Metrics

```rust
pub struct ServerMetrics {
    pub tick_rate: u32,           // Target: 60 TPS
    pub avg_tick_time: Duration,
    pub message_rate: u32,        // Messages per second
    pub player_count: u32,
}

impl ServerMetrics {
    pub fn log_stats(&self) {
        log::info!(
            "Server: {} players, {:.1} TPS, {:.2}ms avg tick, {} msg/s",
            self.player_count,
            self.tick_rate,
            self.avg_tick_time.as_secs_f32() * 1000.0,
            self.message_rate
        );
    }
}
```

## Graceful Disconnect

### Client Disconnect Detection

```rust
pub async fn detect_disconnects(server: Arc<Server>) {
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    
    loop {
        interval.tick().await;
        
        let players = server.get_all_players();
        
        for player_id in players {
            let player = server.get_player(player_id);
            
            // Check ping timeout
            if server.ping_manager.is_timed_out(&player) {
                log::warn!("Player {} timed out", player_id);
                server.disconnect_player(player_id, "Ping timeout").await;
                continue;
            }
            
            // Check idle kick
            if server.idle_detector.check_idle(&player) == IdleStatus::KickRequired {
                log::info!("Kicking idle player {}", player_id);
                server.disconnect_player(player_id, "Idle too long").await;
            }
        }
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
    fn test_game_time_tick() {
        let mut time = GameTime::new();
        
        // Test minute rollover
        time.minute = 59;
        time.tick_minute();
        assert_eq!(time.minute, 0);
        assert_eq!(time.hour, 1);
        
        // Test hour rollover
        time.hour = 23;
        time.minute = 59;
        time.tick_minute();
        assert_eq!(time.hour, 0);
        assert_eq!(time.minute, 0);
        assert_eq!(time.day, 2); // Monday
        
        // Test day rollover
        time.day = 7; // Saturday
        time.hour = 23;
        time.minute = 59;
        time.tick_minute();
        assert_eq!(time.day, 1); // Wrap to Sunday
    }
    
    #[test]
    fn test_ping_timeout() {
        let manager = PingManager::new();
        let mut player = Player::new();
        
        // Initial state
        assert!(!manager.is_timed_out(&player));
        
        // Simulate 61 seconds passing
        player.last_ping_received = Instant::now() - Duration::from_secs(61);
        assert!(manager.is_timed_out(&player));
    }
    
    #[test]
    fn test_frames_to_ping() {
        assert_eq!(frames_to_ping_ms(6, 60), 100);   // 6 frames = 100ms
        assert_eq!(frames_to_ping_ms(30, 60), 500);  // 30 frames = 500ms
        assert_eq!(frames_to_ping_ms(60, 60), 1000); // 60 frames = 1000ms
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_ping_keepalive() {
    let server = create_test_server().await;
    let player_id = server.add_player("test").await;
    
    // Send ping
    server.ping_manager.send_ping(&mut server.get_player_mut(player_id)).await.unwrap();
    
    // Client should receive MSG_PING
    let msg = server.get_player_message(player_id).await;
    assert_eq!(msg.msg_type, MSG_PING);
    
    // Client responds with MSG_PING
    server.handle_message(player_id, MSG_PING, &[]).await.unwrap();
    
    // Player should not time out
    assert!(!server.ping_manager.is_timed_out(&server.get_player(player_id)));
}

#[tokio::test]
async fn test_time_sync() {
    let server = create_test_server().await;
    let player_id = server.add_player("test").await;
    
    // Manually trigger time update
    server.time_manager.game_time.minute = 59;
    server.time_manager.last_update = Instant::now() - Duration::from_secs(61);
    
    server.time_manager.broadcast_update(&server).await.unwrap();
    
    // Player should receive MSG_TIME with minute=0, hour=1
    let msg = server.get_player_message(player_id).await;
    assert_eq!(msg.msg_type, MSG_TIME);
    assert_eq!(msg.read_u8(), 1); // Hour update
    assert_eq!(msg.read_u8(), 1); // hour = 1
}
```

## Summary

**Keepalive Protocol:**
- ✅ Server sends MSG_PING every 30 seconds
- ✅ Client responds with MSG_PING echo
- ✅ Server disconnects if no response for 60 seconds
- ✅ Client assumes disconnected if no ping for 30+ seconds

**Latency Measurement:**
- ✅ Client sends MSG_PING_REQ when player requests ping
- ✅ Server immediately echoes MSG_PING_REQ
- ✅ Client counts frames (60 FPS) to calculate RTT

**Time Synchronization:**
- ✅ Server maintains in-game clock (hour, minute, day)
- ✅ Updates broadcast every minute via MSG_TIME
- ✅ Three update types: hour (1), minute (2), day change (3)
- ✅ Clock can run real-time or accelerated

**Key Constants:**
```rust
const PING_INTERVAL: Duration = Duration::from_secs(30);   // 1800 frames
const PING_TIMEOUT: Duration = Duration::from_secs(60);    // Disconnect threshold
const TIME_UPDATE_INTERVAL: Duration = Duration::from_secs(60); // 1 minute
```

**Next:** See [`../architecture/02-connection-manager.md`](../architecture/02-connection-manager.md) for TCP connection lifecycle management.
