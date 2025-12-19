# Movement Protocol

**Document Status:** Complete  
**Last Updated:** 2024-01-08  
**Related:** [`02-message-format.md`](02-message-format.md), [`04-message-catalog.md`](04-message-catalog.md)

## Overview

This document specifies the movement synchronization protocol for Slime Online 2. Movement uses a **keypress-based system** where clients send input events (key press/release) and the server broadcasts these to other players. Each client performs **client-side prediction** while the server validates positions.

**Key Characteristics:**
- **Event-driven:** Movement messages sent on key press/release, not continuous updates
- **State-based:** Tracks pressed keys (left, right, up, down) as boolean flags
- **Physics-aware:** Different messages for ground vs. air movement
- **Position sync:** Ground movements include position for server validation
- **Client-authoritative prediction:** Client predicts movement, server validates

## Movement Direction Codes

MSG_MOVE_PLAYER (8) uses a single byte direction code:

```rust
#[repr(u8)]
pub enum MovementDirection {
    // Ground Movement - Press (includes position)
    LeftPress = 1,      // Press left while on ground (x, y included)
    RightPress = 2,     // Press right while on ground (x, y included)
    UpPress = 3,        // Press up/jump while on ground (x included, no y)
    DownPress = 4,      // Press down/crouch (no position)
    
    // Ground Movement - Release (includes position)
    LeftRelease = 5,    // Release left while on ground (x, y included)
    RightRelease = 6,   // Release right while on ground (x, y included)
    UpRelease = 7,      // Release up (no position)
    DownRelease = 8,    // Release down/crouch (no position)
    
    // Position Update
    Landing = 9,        // Player landed on ground (x, y included)
    
    // Air Movement - Press (no position)
    LeftPressAir = 10,  // Press left while in air
    RightPressAir = 11, // Press right while in air
    
    // Air Movement - Release (no position)
    LeftReleaseAir = 12,  // Release left while in air
    RightReleaseAir = 13, // Release right while in air
}
```

## Message Formats

### Client → Server: Movement Input

```
MSG_MOVE_PLAYER (8)
├─ direction: u8          // MovementDirection code (1-13)
└─ [position data]        // Conditional on direction code
```

**Position Data by Direction:**

| Direction | Position Data | Total Size |
|-----------|---------------|------------|
| 1 (LeftPress) | x: u16, y: u16 | 5 bytes |
| 2 (RightPress) | x: u16, y: u16 | 5 bytes |
| 3 (UpPress) | x: u16 | 3 bytes |
| 4 (DownPress) | *none* | 1 byte |
| 5 (LeftRelease) | x: u16, y: u16 | 5 bytes |
| 6 (RightRelease) | x: u16, y: u16 | 5 bytes |
| 7 (UpRelease) | *none* | 1 byte |
| 8 (DownRelease) | *none* | 1 byte |
| 9 (Landing) | x: u16, y: u16 | 5 bytes |
| 10 (LeftPressAir) | *none* | 1 byte |
| 11 (RightPressAir) | *none* | 1 byte |
| 12 (LeftReleaseAir) | *none* | 1 byte |
| 13 (RightReleaseAir) | *none* | 1 byte |

**Example - Press Right on Ground:**
```
[00 08]              // MSG_MOVE_PLAYER
[02]                 // direction = 2 (RightPress)
[C0 01]              // x = 448
[A0 00]              // y = 160
```

**Example - Press Left in Air:**
```
[00 08]              // MSG_MOVE_PLAYER
[0A]                 // direction = 10 (LeftPressAir)
// No position data
```

### Server → Client: Broadcast Movement

Server broadcasts to all players in the same room:

```
MSG_MOVE_PLAYER (8)
├─ player_id: u16         // Which player moved
├─ direction: u8          // MovementDirection code (1-13)
└─ [position data]        // Same as client→server format
```

**Example - Broadcast Player 5 Moving Right:**
```
[00 08]              // MSG_MOVE_PLAYER
[05 00]              // player_id = 5
[02]                 // direction = 2 (RightPress)
[C0 01]              // x = 448
[A0 00]              // y = 160
```

## Movement State Machine

Each player maintains movement state:

```rust
pub struct MovementState {
    // Input flags (synchronized across clients)
    pub ileft: bool,      // Left key pressed
    pub iright: bool,     // Right key pressed
    pub iup: bool,        // Up key held
    pub iup_press: bool,  // Up key was pressed (triggers jump)
    pub idown: bool,      // Down key pressed (crouching)
    
    // Physics state
    pub free: bool,       // Not touching ground (in air)
    pub hsp: f32,         // Horizontal speed
    pub vsp: f32,         // Vertical speed
    pub ice: bool,        // On ice (affects acceleration)
    
    // Position
    pub x: f32,
    pub y: f32,
    
    // Sprite state
    pub spr_dir: i8,      // Sprite direction: 1 = right, -1 = left
}
```

### State Transitions

**Ground → Air:**
- Player jumps (UpPress) → `free = true`
- Player walks off edge → Client sends Landing(9) when hits ground

**Air → Ground:**
- Player lands → `free = false`, client MAY send Landing(9)

**Key Press Handling:**

```rust
// Client sends when key state changes
if keyboard_pressed(LEFT) && on_ground() {
    send_message(MSG_MOVE_PLAYER, LeftPress, Some(x), Some(y));
    ileft = true;
}

if keyboard_released(LEFT) && on_ground() {
    send_message(MSG_MOVE_PLAYER, LeftRelease, Some(x), Some(y));
    ileft = false;
}

// Different messages for air
if keyboard_pressed(LEFT) && in_air() {
    send_message(MSG_MOVE_PLAYER, LeftPressAir, None, None);
    ileft = true;
}
```

## Physics Constants

### Movement Speeds

```rust
// From client code (scr_movement_o.gml)
pub const HSPMAX: f32 = 2.5;          // Maximum horizontal speed
pub const JUMP_SPEED: f32 = 4.5;      // Initial jump velocity
pub const GRAVITY: f32 = 0.2145;      // Gravity acceleration (0.33 * 0.65)
pub const TERMINAL_VELOCITY: f32 = 9.0; // Max fall speed

// Ground acceleration
pub const ACCEL_GROUND: f32 = 0.33;   // Horizontal acceleration on ground
pub const ACCEL_ICE: f32 = 0.033;     // Horizontal acceleration on ice

// Friction
pub const FRICTION_GROUND: f32 = 0.165; // Ground friction (0.5 * 0.33)
pub const FRICTION_ICE: f32 = 0.0066;   // Ice friction (0.02 * 0.33)
pub const FRICTION_CROUCH: f32 = 1.7;   // Crouch slowdown divisor

// Water physics
pub const WATER_RESISTANCE: f32 = 2.0;  // Water horizontal drag divisor
pub const WATER_BUOYANCY: f32 = 0.165;  // Upward force in water (0.5 * 0.33)
pub const WATER_SWIM_UP: f32 = 0.099;   // Extra up when holding up (0.3 * 0.33)
pub const WATER_SWIM_DOWN: f32 = 0.099; // Extra down when holding down
pub const WATER_MAX_FALL: f32 = 0.75;   // Max fall in water (1.5 * 0.5)
pub const WATER_MAX_RISE: f32 = -1.0;   // Max rise in water (-2 * 0.5)
```

### Client Physics Loop (60 FPS)

```rust
// Executed every frame on client
fn update_movement_physics(state: &mut MovementState) {
    // Horizontal acceleration
    if state.iright && !state.ileft && state.hsp < HSPMAX {
        if !state.ice {
            state.hsp += ACCEL_GROUND; // 0.33
        } else {
            state.hsp += ACCEL_ICE;    // 0.033
        }
    }
    
    if state.ileft && !state.iright && state.hsp > -HSPMAX {
        if !state.ice {
            state.hsp -= ACCEL_GROUND;
        } else {
            state.hsp -= ACCEL_ICE;
        }
    }
    
    // Jump
    if state.iup_press && state.vsp == 0.0 && !state.free {
        state.vsp = -JUMP_SPEED; // -4.5
        state.free = true;
    }
    
    // Gravity (only in air)
    if state.free && state.vsp < TERMINAL_VELOCITY {
        state.vsp += GRAVITY; // 0.2145 per frame
    }
    
    // Friction (only on ground)
    if !state.free {
        if state.ice {
            state.hsp = decel(state.hsp, FRICTION_ICE);
        } else {
            state.hsp = decel(state.hsp, FRICTION_GROUND);
        }
    }
    
    // Crouch slowdown
    if state.idown && !state.free && state.hsp != 0.0 {
        state.hsp /= FRICTION_CROUCH; // Divide by 1.7
    }
    
    // Water physics (if in water)
    if state.in_water && state.free {
        state.hsp /= WATER_RESISTANCE; // Divide by 2.0
        state.vsp -= WATER_BUOYANCY;   // Upward force
        
        if state.iup {
            state.vsp -= WATER_SWIM_UP;
        }
        if state.idown {
            state.vsp += WATER_SWIM_DOWN;
        }
        
        // Clamp water speeds
        state.vsp = state.vsp.clamp(WATER_MAX_RISE, WATER_MAX_FALL);
    }
    
    // Apply movement (with collision detection)
    apply_movement(state);
}

fn decel(speed: f32, friction: f32) -> f32 {
    let abs_speed = speed.abs();
    if abs_speed < friction { 0.0 } else { (abs_speed - friction) * speed.signum() }
}
```

## Server-Side Validation

The server MUST validate all movement to prevent cheating:

### 1. Position Validation

```rust
pub fn validate_movement(
    player: &Player,
    direction: u8,
    new_x: Option<u16>,
    new_y: Option<u16>,
) -> Result<ValidatedMove, ValidationError> {
    let delta_time = (Instant::now() - player.last_move_time).as_secs_f32();
    
    // Validate position is included when required
    match direction {
        1 | 2 | 5 | 6 | 9 => { // Requires x, y
            if new_x.is_none() || new_y.is_none() {
                return Err(ValidationError::MissingPosition);
            }
        }
        3 => { // Requires x only
            if new_x.is_none() {
                return Err(ValidationError::MissingPosition);
            }
        }
        _ => {}
    }
    
    // Check for teleportation (if position included)
    if let (Some(x), Some(y)) = (new_x, new_y) {
        let distance = calculate_distance(
            player.x, player.y,
            x as f32, y as f32
        );
        
        // Max possible distance in delta_time
        let max_distance = calculate_max_distance(delta_time);
        
        if distance > max_distance * 1.5 { // 1.5x tolerance for lag
            log::warn!(
                "Player {} teleported: {:.1} > {:.1} in {:.3}s",
                player.id, distance, max_distance, delta_time
            );
            return Err(ValidationError::Teleport { distance, max_distance });
        }
    }
    
    Ok(ValidatedMove {
        direction,
        x: new_x,
        y: new_y,
        timestamp: Instant::now(),
    })
}

fn calculate_max_distance(delta_time: f32) -> f32 {
    // Max horizontal speed
    let max_hsp = HSPMAX; // 2.5 pixels/frame
    
    // Max vertical speed (falling)
    let max_vsp = TERMINAL_VELOCITY; // 9.0 pixels/frame
    
    // Convert to pixels per second (60 FPS)
    let max_speed_per_sec = ((max_hsp.powi(2) + max_vsp.powi(2)).sqrt()) * 60.0;
    
    max_speed_per_sec * delta_time
}
```

### 2. Rate Limiting

```rust
pub struct MovementRateLimiter {
    max_moves_per_second: u32,
    window_size: Duration,
}

impl MovementRateLimiter {
    pub fn new() -> Self {
        Self {
            max_moves_per_second: 60, // 60 FPS max
            window_size: Duration::from_secs(1),
        }
    }
    
    pub fn check_rate(&self, player: &Player) -> Result<(), ValidationError> {
        let recent_moves = player.movement_history.iter()
            .filter(|m| m.timestamp.elapsed() < self.window_size)
            .count();
        
        if recent_moves > self.max_moves_per_second as usize {
            log::warn!("Player {} exceeded movement rate: {}/s", 
                      player.id, recent_moves);
            return Err(ValidationError::RateLimitExceeded);
        }
        
        Ok(())
    }
}
```

### 3. State Consistency Validation

```rust
pub fn validate_state_transition(
    current_state: &MovementState,
    direction: u8,
) -> Result<(), ValidationError> {
    match direction {
        // Can't press a key that's already pressed
        1 | 10 => { // LeftPress or LeftPressAir
            if current_state.ileft {
                return Err(ValidationError::InvalidStateTransition(
                    "Left already pressed"
                ));
            }
        }
        
        // Can't release a key that's not pressed
        5 | 12 => { // LeftRelease or LeftReleaseAir
            if !current_state.ileft {
                return Err(ValidationError::InvalidStateTransition(
                    "Left not pressed"
                ));
            }
        }
        
        // Ground moves require being on ground
        1 | 2 | 3 | 5 | 6 => {
            if current_state.free {
                return Err(ValidationError::InvalidStateTransition(
                    "Ground move while in air"
                ));
            }
        }
        
        // Air moves require being in air
        10 | 11 | 12 | 13 => {
            if !current_state.free {
                return Err(ValidationError::InvalidStateTransition(
                    "Air move while on ground"
                ));
            }
        }
        
        _ => {}
    }
    
    Ok(())
}
```

### 4. Anti-Cheat Detection

```rust
pub struct AntiCheat {
    max_speed_violations: u32,
    violation_window: Duration,
}

impl AntiCheat {
    pub fn check_violations(&self, player: &Player) -> CheatDetection {
        let recent_violations = player.violations.iter()
            .filter(|v| v.timestamp.elapsed() < self.violation_window)
            .count();
        
        if recent_violations > self.max_speed_violations as usize {
            CheatDetection::Detected {
                reason: "Multiple speed violations",
                action: BanAction::Temporary { duration: Duration::from_hours(24) },
            }
        } else {
            CheatDetection::Clean
        }
    }
}
```

## Movement Broadcasting

### Room-Based Broadcasting

Server broadcasts movement to all players in same room EXCEPT the sender:

```rust
pub async fn handle_movement(
    server: &Server,
    player_id: u16,
    direction: u8,
    x: Option<u16>,
    y: Option<u16>,
) -> Result<(), ServerError> {
    // 1. Validate movement
    let player = server.get_player(player_id)?;
    validate_movement(&player, direction, x, y)?;
    validate_state_transition(&player.movement_state, direction)?;
    
    // 2. Update player state
    update_movement_state(&mut player.movement_state, direction, x, y);
    
    // 3. Broadcast to room
    let room_id = player.current_room;
    let mut message = MessageWriter::new();
    message.write_u16(MSG_MOVE_PLAYER);
    message.write_u16(player_id);
    message.write_u8(direction);
    
    // Include position data based on direction
    match direction {
        1 | 2 | 5 | 6 | 9 => {
            message.write_u16(x.unwrap());
            message.write_u16(y.unwrap());
        }
        3 => {
            message.write_u16(x.unwrap());
        }
        _ => {} // No position
    }
    
    // Broadcast to all in room except sender
    server.broadcast_to_room_except(
        room_id,
        player_id,
        &message.build()
    ).await?;
    
    Ok(())
}

fn update_movement_state(
    state: &mut MovementState,
    direction: u8,
    x: Option<u16>,
    y: Option<u16>,
) {
    // Update position if provided
    if let Some(new_x) = x {
        state.x = new_x as f32;
    }
    if let Some(new_y) = y {
        state.y = new_y as f32;
    }
    
    // Update input flags
    match direction {
        1 | 10 => state.ileft = true,        // LeftPress
        2 | 11 => state.iright = true,       // RightPress
        3 => {
            state.iup = true;
            state.iup_press = true;
        }
        4 => state.idown = true,             // DownPress
        5 | 12 => state.ileft = false,       // LeftRelease
        6 | 13 => state.iright = false,      // RightRelease
        7 => {
            state.iup = false;
            state.iup_press = false;
        }
        8 => state.idown = false,            // DownRelease
        9 => state.free = false,             // Landing
        _ => {}
    }
}
```

## Special Cases

### 1. Landing Detection

The client SHOULD send Landing(9) when transitioning from air to ground, but this is NOT guaranteed. Server should also detect landings based on lack of movement updates.

```rust
// Server-side landing detection
pub fn detect_landing(player: &mut Player) -> bool {
    if player.movement_state.free {
        let time_since_movement = player.last_move_time.elapsed();
        
        // If no movement for 100ms while in air, assume landed
        if time_since_movement > Duration::from_millis(100) {
            player.movement_state.free = false;
            player.movement_state.vsp = 0.0;
            return true;
        }
    }
    false
}
```

### 2. Water Movement

Water affects physics but does NOT have special movement messages. Client applies water physics locally.

### 3. Jump Timing

Jump is triggered by UpPress(3), but the actual jump velocity is applied client-side immediately. Server should simulate jump on receiving UpPress.

### 4. Crouch Movement

Player can still move horizontally while crouching (DownPress), but at reduced speed (divided by 1.7).

## Performance Considerations

### Message Frequency

- **Typical:** 5-10 movement messages per second per player (key press/release events)
- **Worst case:** 60 messages per second (rapid key spam)
- **Broadcast:** Each message × (players_in_room - 1)

### Optimization Strategies

```rust
// 1. Debounce rapid key presses (client-side)
pub struct MovementDebouncer {
    min_interval: Duration,
    last_send: Instant,
}

impl MovementDebouncer {
    pub fn can_send(&mut self) -> bool {
        let now = Instant::now();
        if now - self.last_send > self.min_interval {
            self.last_send = now;
            true
        } else {
            false
        }
    }
}

// 2. Position delta encoding (for future optimization)
pub fn encode_position_delta(
    old_x: u16, old_y: u16,
    new_x: u16, new_y: u16
) -> (i8, i8) {
    let dx = (new_x as i32 - old_x as i32).clamp(-128, 127) as i8;
    let dy = (new_y as i32 - old_y as i32).clamp(-128, 127) as i8;
    (dx, dy)
}
```

## Testing Movement

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_movement_validation() {
        let mut player = create_test_player();
        
        // Valid: Press left on ground
        let result = validate_movement(&player, 1, Some(100), Some(100));
        assert!(result.is_ok());
        
        // Invalid: Missing position
        let result = validate_movement(&player, 1, None, None);
        assert!(matches!(result, Err(ValidationError::MissingPosition)));
        
        // Invalid: Teleport
        player.x = 0.0;
        player.y = 0.0;
        let result = validate_movement(&player, 1, Some(1000), Some(1000));
        assert!(matches!(result, Err(ValidationError::Teleport { .. })));
    }
    
    #[test]
    fn test_state_transitions() {
        let mut state = MovementState::default();
        
        // Valid: Press left
        assert!(validate_state_transition(&state, 1).is_ok());
        state.ileft = true;
        
        // Invalid: Press left again
        assert!(validate_state_transition(&state, 1).is_err());
        
        // Valid: Release left
        assert!(validate_state_transition(&state, 5).is_ok());
    }
    
    #[test]
    fn test_physics_constants() {
        let mut state = MovementState {
            iright: true,
            ileft: false,
            hsp: 0.0,
            ice: false,
            ..Default::default()
        };
        
        // After 1 frame
        update_movement_physics(&mut state);
        assert_eq!(state.hsp, ACCEL_GROUND); // 0.33
        
        // After reaching max speed
        for _ in 0..20 {
            update_movement_physics(&mut state);
        }
        assert!(state.hsp <= HSPMAX); // 2.5
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_movement_broadcast() {
    let server = create_test_server().await;
    let player1 = server.add_player("player1").await;
    let player2 = server.add_player("player2").await;
    
    // Both in same room
    server.join_room(player1, 1).await;
    server.join_room(player2, 1).await;
    
    // Player1 moves right
    server.handle_movement(player1, 2, Some(100), Some(100)).await.unwrap();
    
    // Player2 should receive broadcast
    let message = server.get_player_message(player2).await;
    assert_eq!(message.msg_type, MSG_MOVE_PLAYER);
    assert_eq!(message.read_u16(), player1); // player_id
    assert_eq!(message.read_u8(), 2); // direction
    assert_eq!(message.read_u16(), 100); // x
    assert_eq!(message.read_u16(), 100); // y
}
```

## Client Behavior Reference

From decompiled client (`scr_controles.gml`, `case_msg_move_player.gml`):

1. **Client sends movement on key press/release ONLY**
2. **Position sent with ground movements** (left/right press/release)
3. **Air movements don't include position** (to reduce bandwidth)
4. **Jump sends x-position only** (y not needed, server knows ground level)
5. **Landing message optional** (client may not send it)
6. **Physics simulated client-side** at 60 FPS
7. **Position updates override client position** when received from server

## Summary

The movement protocol is **event-driven** rather than state-based:
- ✅ Clients send key press/release events with position snapshots
- ✅ Server validates positions against physics limits
- ✅ Server broadcasts to all players in same room
- ✅ Each client runs physics simulation locally
- ✅ 13 distinct movement direction codes
- ✅ Separate handling for ground vs. air movement
- ⚠️ Server MUST validate all positions to prevent teleport hacks
- ⚠️ Rate limiting required to prevent message spam

**Next:** See [`06-timing-and-sync.md`](06-timing-and-sync.md) for keepalive and timing protocols.
