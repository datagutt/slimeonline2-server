# Room System & Broadcasting

**Document Status:** Complete  
**Last Updated:** 2024-01-08  
**Related:** [`03-world-manager.md`](03-world-manager.md), [`04-player-manager.md`](04-player-manager.md)

## Overview

The Room System manages **player visibility** and **message broadcasting** based on room-based zones. Players can only see and interact with other players in the same room. This system coordinates broadcasts to ensure messages (movement, chat, etc.) are only sent to relevant players.

**Key Concepts:**
- **Room-based visibility:** Players only see others in same room
- **Broadcast optimization:** Messages sent only to players who need them
- **Room transitions:** Handle player entering/leaving rooms
- **Player introduction:** Sync new players with existing room state

## Architecture

```
┌────────────────────────────────────────────────────────┐
│                    Room System                          │
├────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────────────────────────────────────────┐  │
│  │          Broadcast Manager                       │  │
│  │  - Room-based broadcasting                      │  │
│  │  - Global announcements                          │  │
│  │  - Targeted messages                             │  │
│  └─────────────────────────────────────────────────┘  │
│                          │                              │
│                          v                              │
│  ┌─────────────────────────────────────────────────┐  │
│  │         Room Player Registry                     │  │
│  │  Room ID → Set<PlayerId>                        │  │
│  │  (derived from Player.current_room)             │  │
│  └─────────────────────────────────────────────────┘  │
│                          │                              │
│                          v                              │
│  ┌─────────────────────────────────────────────────┐  │
│  │        Player Introduction Protocol              │  │
│  │  - Send MSG_NEW_PLAYER when joining             │  │
│  │  - Receive existing players in room              │  │
│  │  - Sync room state (collectibles, plants, etc.) │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
└────────────────────────────────────────────────────────┘
```

## Broadcast Manager

```rust
use dashmap::DashMap;
use std::sync::Arc;

pub struct BroadcastManager {
    /// Player manager reference
    player_manager: Arc<PlayerManager>,
    
    /// Connection manager reference (for sending messages)
    connection_manager: Arc<ConnectionManager>,
}

impl BroadcastManager {
    pub fn new(
        player_manager: Arc<PlayerManager>,
        connection_manager: Arc<ConnectionManager>,
    ) -> Self {
        Self {
            player_manager,
            connection_manager,
        }
    }
}
```

## Broadcasting Strategies

### 1. Broadcast to Room

Send message to all players in a specific room:

```rust
impl BroadcastManager {
    /// Broadcast message to all players in room
    pub async fn broadcast_to_room(
        &self,
        room_id: RoomId,
        message: &[u8],
    ) -> Result<usize, BroadcastError> {
        let player_ids = self.player_manager.get_players_in_room(room_id).await;
        
        let mut sent_count = 0;
        
        for player_id in player_ids {
            if let Some(player_arc) = self.player_manager.get_player(player_id) {
                let player = player_arc.read().await;
                
                // Get connection
                if let Some(conn) = self.connection_manager.get_connection(player.connection_id) {
                    let conn = conn.lock().await;
                    
                    if let Err(e) = conn.send_message(message) {
                        log::warn!("Failed to send to player {}: {}", player_id, e);
                    } else {
                        sent_count += 1;
                    }
                }
            }
        }
        
        log::trace!("Broadcast to room {}: {} players", room_id, sent_count);
        
        Ok(sent_count)
    }
    
    /// Broadcast to room except specific player (used for echoing)
    pub async fn broadcast_to_room_except(
        &self,
        room_id: RoomId,
        except_player_id: PlayerId,
        message: &[u8],
    ) -> Result<usize, BroadcastError> {
        let player_ids = self.player_manager.get_players_in_room(room_id).await;
        
        let mut sent_count = 0;
        
        for player_id in player_ids {
            // Skip the excluded player
            if player_id == except_player_id {
                continue;
            }
            
            if let Some(player_arc) = self.player_manager.get_player(player_id) {
                let player = player_arc.read().await;
                
                if let Some(conn) = self.connection_manager.get_connection(player.connection_id) {
                    let conn = conn.lock().await;
                    
                    if let Err(e) = conn.send_message(message) {
                        log::warn!("Failed to send to player {}: {}", player_id, e);
                    } else {
                        sent_count += 1;
                    }
                }
            }
        }
        
        Ok(sent_count)
    }
}
```

### 2. Global Broadcast

Send message to all online players:

```rust
impl BroadcastManager {
    /// Broadcast to all online players
    pub async fn broadcast_to_all(
        &self,
        message: &[u8],
    ) -> Result<usize, BroadcastError> {
        let player_ids = self.player_manager.get_online_players();
        
        let mut sent_count = 0;
        
        for player_id in player_ids {
            if let Some(player_arc) = self.player_manager.get_player(player_id) {
                let player = player_arc.read().await;
                
                if let Some(conn) = self.connection_manager.get_connection(player.connection_id) {
                    let conn = conn.lock().await;
                    
                    if let Err(e) = conn.send_message(message) {
                        log::warn!("Failed to send to player {}: {}", player_id, e);
                    } else {
                        sent_count += 1;
                    }
                }
            }
        }
        
        log::debug!("Global broadcast: {} players", sent_count);
        
        Ok(sent_count)
    }
    
    /// Broadcast to all except one player
    pub async fn broadcast_to_all_except(
        &self,
        except_player_id: PlayerId,
        message: &[u8],
    ) -> Result<usize, BroadcastError> {
        let player_ids = self.player_manager.get_online_players();
        
        let mut sent_count = 0;
        
        for player_id in player_ids {
            if player_id == except_player_id {
                continue;
            }
            
            if let Some(player_arc) = self.player_manager.get_player(player_id) {
                let player = player_arc.read().await;
                
                if let Some(conn) = self.connection_manager.get_connection(player.connection_id) {
                    let conn = conn.lock().await;
                    
                    if let Err(e) = conn.send_message(message) {
                        log::warn!("Failed to send to player {}: {}", player_id, e);
                    } else {
                        sent_count += 1;
                    }
                }
            }
        }
        
        Ok(sent_count)
    }
}
```

### 3. Targeted Broadcast

Send to specific list of players:

```rust
impl BroadcastManager {
    /// Send message to specific players
    pub async fn send_to_players(
        &self,
        player_ids: &[PlayerId],
        message: &[u8],
    ) -> Result<usize, BroadcastError> {
        let mut sent_count = 0;
        
        for &player_id in player_ids {
            if let Some(player_arc) = self.player_manager.get_player(player_id) {
                let player = player_arc.read().await;
                
                if let Some(conn) = self.connection_manager.get_connection(player.connection_id) {
                    let conn = conn.lock().await;
                    
                    if let Err(e) = conn.send_message(message) {
                        log::warn!("Failed to send to player {}: {}", player_id, e);
                    } else {
                        sent_count += 1;
                    }
                }
            }
        }
        
        Ok(sent_count)
    }
    
    /// Send to single player
    pub async fn send_to_player(
        &self,
        player_id: PlayerId,
        message: &[u8],
    ) -> Result<(), BroadcastError> {
        if let Some(player_arc) = self.player_manager.get_player(player_id) {
            let player = player_arc.read().await;
            
            if let Some(conn) = self.connection_manager.get_connection(player.connection_id) {
                let conn = conn.lock().await;
                conn.send_message(message)
                    .map_err(|_| BroadcastError::SendFailed)?;
                
                return Ok(());
            }
        }
        
        Err(BroadcastError::PlayerNotFound)
    }
}
```

## Room Transitions

### Player Entering Room

When a player changes rooms, they must be introduced to all existing players in the new room:

```rust
pub async fn handle_room_change(
    player_id: PlayerId,
    new_room: RoomId,
    player_manager: &PlayerManager,
    world_manager: &WorldManager,
    broadcast_manager: &BroadcastManager,
) -> Result<(), RoomError> {
    // 1. Get player info
    let (username, x, y, body_id, acs1, acs2) = {
        let player_arc = player_manager.get_player(player_id)
            .ok_or(RoomError::PlayerNotFound)?;
        let player = player_arc.read().await;
        
        (
            player.username.clone(),
            player.x,
            player.y,
            player.body_id,
            player.acs1_id,
            player.acs2_id,
        )
    };
    
    // 2. Get existing players in new room (before we join)
    let existing_players = player_manager.get_players_in_room(new_room).await;
    
    // 3. Move player to new room
    player_manager.change_player_room(player_id, new_room, world_manager).await?;
    
    // 4. Introduce new player to existing players (MSG_NEW_PLAYER case 1)
    for existing_player_id in &existing_players {
        send_player_introduction(
            *existing_player_id,
            player_id,
            x, y,
            new_room,
            &username,
            body_id,
            acs1,
            acs2,
            broadcast_manager,
        ).await?;
    }
    
    // 5. Send existing players to new player (MSG_NEW_PLAYER case 2)
    for existing_player_id in &existing_players {
        send_existing_player_to_new(
            player_id,
            *existing_player_id,
            player_manager,
            broadcast_manager,
        ).await?;
    }
    
    // 6. Send room state (collectibles, plants, etc.)
    send_room_state(player_id, new_room, world_manager, broadcast_manager).await?;
    
    Ok(())
}
```

### MSG_NEW_PLAYER Protocol

From `case_msg_new_player.gml`, there are two cases:

```rust
/// Case 1: Tell existing player about new player (requires response)
async fn send_player_introduction(
    to_player_id: PlayerId,
    new_player_id: PlayerId,
    x: u16,
    y: u16,
    room_id: RoomId,
    username: &str,
    body_id: u16,
    acs1: u16,
    acs2: u16,
    broadcast_manager: &BroadcastManager,
) -> Result<(), RoomError> {
    let mut msg = MessageWriter::new();
    msg.write_u16(MSG_NEW_PLAYER);
    msg.write_u8(1); // Case 1: needs response
    msg.write_u16(x);
    msg.write_u16(y);
    msg.write_u16(new_player_id);
    msg.write_u16(room_id);
    msg.write_string(username);
    msg.write_u16(body_id);
    msg.write_u16(acs1);
    msg.write_u16(acs2);
    
    broadcast_manager.send_to_player(to_player_id, &msg.build()).await?;
    
    Ok(())
}

/// Case 2: Send existing player info to new player (no response needed)
async fn send_existing_player_to_new(
    to_player_id: PlayerId,
    existing_player_id: PlayerId,
    player_manager: &PlayerManager,
    broadcast_manager: &BroadcastManager,
) -> Result<(), RoomError> {
    let player_arc = player_manager.get_player(existing_player_id)
        .ok_or(RoomError::PlayerNotFound)?;
    
    let player = player_arc.read().await;
    
    let mut msg = MessageWriter::new();
    msg.write_u16(MSG_NEW_PLAYER);
    msg.write_u8(2); // Case 2: no response needed
    msg.write_u16(player.x);
    msg.write_u16(player.y);
    msg.write_u16(existing_player_id);
    msg.write_u8(player.movement_state.ileft as u8);
    msg.write_u8(player.movement_state.iright as u8);
    msg.write_u8(player.movement_state.iup as u8);
    msg.write_u8(player.movement_state.idown as u8);
    msg.write_u8(player.movement_state.iup_press as u8);
    msg.write_u16(player.current_room);
    msg.write_string(&player.username);
    msg.write_u16(player.body_id);
    msg.write_u16(player.acs1_id);
    msg.write_u16(player.acs2_id);
    
    broadcast_manager.send_to_player(to_player_id, &msg.build()).await?;
    
    Ok(())
}
```

### Send Room State

```rust
async fn send_room_state(
    player_id: PlayerId,
    room_id: RoomId,
    world_manager: &WorldManager,
    broadcast_manager: &BroadcastManager,
) -> Result<(), RoomError> {
    let room_arc = world_manager.get_room(room_id);
    let room = room_arc.read().await;
    
    // Send collectibles
    for (slot, collectible) in &room.collectibles {
        let mut msg = MessageWriter::new();
        msg.write_u16(MSG_COLLECTIBLE_INFO);
        msg.write_u8(*slot);
        msg.write_u16(collectible.collectible_id);
        msg.write_u16(collectible.x);
        msg.write_u16(collectible.y);
        msg.write_u8(collectible.evolution_stage);
        
        broadcast_manager.send_to_player(player_id, &msg.build()).await?;
    }
    
    // Send plant spots
    for (slot, plant) in &room.plant_spots {
        let mut msg = MessageWriter::new();
        msg.write_u16(MSG_PLANT_SPOT_USED);
        msg.write_u8(*slot);
        msg.write_u16(plant.plant_type);
        msg.write_u8(plant.growth_stage);
        msg.write_u8(plant.has_fruit as u8);
        msg.write_u8(plant.fruit_count);
        msg.write_u8(plant.has_pinwheel as u8);
        msg.write_u8(plant.has_fairy as u8);
        
        broadcast_manager.send_to_player(player_id, &msg.build()).await?;
    }
    
    // Send build spots
    for (slot, build) in &room.build_spots {
        let mut msg = MessageWriter::new();
        msg.write_u16(MSG_BUILD_SPOT_USED);
        msg.write_u8(*slot);
        msg.write_u16(build.object_type);
        msg.write_u8(build.durability);
        
        broadcast_manager.send_to_player(player_id, &msg.build()).await?;
    }
    
    // Send discarded items
    for (instance_id, item) in &room.discarded_items {
        let mut msg = MessageWriter::new();
        msg.write_u16(MSG_DISCARD_ITEM);
        msg.write_u16(*instance_id);
        msg.write_u16(item.item_id);
        msg.write_u16(item.quantity);
        msg.write_u16(item.x);
        msg.write_u16(item.y);
        
        broadcast_manager.send_to_player(player_id, &msg.build()).await?;
    }
    
    Ok(())
}
```

### Player Leaving Room

When a player logs out or changes rooms, notify players in old room:

```rust
pub async fn handle_player_leave_room(
    player_id: PlayerId,
    room_id: RoomId,
    broadcast_manager: &BroadcastManager,
) -> Result<(), RoomError> {
    // Notify other players in room
    let mut msg = MessageWriter::new();
    msg.write_u16(MSG_REMOVE_PLAYER);
    msg.write_u16(player_id);
    
    broadcast_manager.broadcast_to_room_except(
        room_id,
        player_id,
        &msg.build(),
    ).await?;
    
    Ok(())
}
```

## Common Broadcasting Scenarios

### Movement Broadcasting

```rust
pub async fn broadcast_movement(
    player_id: PlayerId,
    direction: u8,
    x: Option<u16>,
    y: Option<u16>,
    player_manager: &PlayerManager,
    broadcast_manager: &BroadcastManager,
) -> Result<(), BroadcastError> {
    // Get player's room
    let room_id = {
        let player_arc = player_manager.get_player(player_id)
            .ok_or(BroadcastError::PlayerNotFound)?;
        let player = player_arc.read().await;
        player.current_room
    };
    
    // Build message
    let mut msg = MessageWriter::new();
    msg.write_u16(MSG_MOVE_PLAYER);
    msg.write_u16(player_id);
    msg.write_u8(direction);
    
    // Add position data based on direction
    match direction {
        1 | 2 | 5 | 6 | 9 => {
            msg.write_u16(x.unwrap());
            msg.write_u16(y.unwrap());
        }
        3 => {
            msg.write_u16(x.unwrap());
        }
        _ => {} // No position
    }
    
    // Broadcast to room (except sender)
    broadcast_manager.broadcast_to_room_except(
        room_id,
        player_id,
        &msg.build(),
    ).await?;
    
    Ok(())
}
```

### Chat Broadcasting

```rust
pub async fn broadcast_chat(
    player_id: PlayerId,
    message: &str,
    player_manager: &PlayerManager,
    broadcast_manager: &BroadcastManager,
) -> Result<(), BroadcastError> {
    // Get player's room and username
    let (room_id, username) = {
        let player_arc = player_manager.get_player(player_id)
            .ok_or(BroadcastError::PlayerNotFound)?;
        let player = player_arc.read().await;
        (player.current_room, player.username.clone())
    };
    
    // Build message
    let mut msg = MessageWriter::new();
    msg.write_u16(MSG_CHAT);
    msg.write_u16(player_id);
    msg.write_string(&username);
    msg.write_string(message);
    
    // Broadcast to room
    broadcast_manager.broadcast_to_room(
        room_id,
        &msg.build(),
    ).await?;
    
    Ok(())
}
```

### Global Announcement

```rust
pub async fn send_global_announcement(
    message: &str,
    broadcast_manager: &BroadcastManager,
) -> Result<(), BroadcastError> {
    let mut msg = MessageWriter::new();
    msg.write_u16(MSG_ANNOUNCEMENT);
    msg.write_string(message);
    
    broadcast_manager.broadcast_to_all(&msg.build()).await?;
    
    Ok(())
}
```

## Performance Optimization

### Batch Broadcasting

For high-frequency events, batch multiple messages:

```rust
pub struct BatchBroadcaster {
    pending: DashMap<RoomId, Vec<Vec<u8>>>,
    max_batch_size: usize,
}

impl BatchBroadcaster {
    pub fn new() -> Self {
        Self {
            pending: DashMap::new(),
            max_batch_size: 10,
        }
    }
    
    /// Add message to batch
    pub fn queue_message(&self, room_id: RoomId, message: Vec<u8>) {
        self.pending.entry(room_id)
            .or_insert_with(Vec::new)
            .push(message);
    }
    
    /// Flush batched messages
    pub async fn flush(
        &self,
        broadcast_manager: &BroadcastManager,
    ) -> Result<(), BroadcastError> {
        for entry in self.pending.iter() {
            let room_id = *entry.key();
            let messages = entry.value();
            
            if messages.is_empty() {
                continue;
            }
            
            // Send each message in batch
            for message in messages.iter() {
                broadcast_manager.broadcast_to_room(room_id, message).await?;
            }
        }
        
        // Clear pending
        self.pending.clear();
        
        Ok(())
    }
}
```

### Message Deduplication

Prevent sending duplicate messages to same player:

```rust
use std::collections::HashSet;

pub async fn broadcast_to_room_deduplicated(
    room_id: RoomId,
    message: &[u8],
    player_manager: &PlayerManager,
    connection_manager: &ConnectionManager,
) -> Result<usize, BroadcastError> {
    let player_ids = player_manager.get_players_in_room(room_id).await;
    let mut sent_connections = HashSet::new();
    let mut sent_count = 0;
    
    for player_id in player_ids {
        if let Some(player_arc) = player_manager.get_player(player_id) {
            let player = player_arc.read().await;
            
            // Skip if already sent to this connection
            if sent_connections.contains(&player.connection_id) {
                continue;
            }
            
            if let Some(conn) = connection_manager.get_connection(player.connection_id) {
                let conn = conn.lock().await;
                
                if conn.send_message(message).is_ok() {
                    sent_connections.insert(player.connection_id);
                    sent_count += 1;
                }
            }
        }
    }
    
    Ok(sent_count)
}
```

## Error Handling

```rust
#[derive(Debug)]
pub enum BroadcastError {
    PlayerNotFound,
    RoomNotFound,
    SendFailed,
}

#[derive(Debug)]
pub enum RoomError {
    PlayerNotFound,
    RoomNotFound,
    BroadcastError(BroadcastError),
}

impl From<BroadcastError> for RoomError {
    fn from(e: BroadcastError) -> Self {
        RoomError::BroadcastError(e)
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_room_broadcast() {
        let (player_mgr, conn_mgr, broadcast_mgr) = setup_test_env().await;
        
        // Create 3 players in room 1
        let p1 = create_test_player(&player_mgr, &conn_mgr, 1).await;
        let p2 = create_test_player(&player_mgr, &conn_mgr, 1).await;
        let p3 = create_test_player(&player_mgr, &conn_mgr, 1).await;
        
        // Create 1 player in room 2
        let p4 = create_test_player(&player_mgr, &conn_mgr, 2).await;
        
        // Broadcast to room 1
        let message = vec![0x01, 0x02, 0x03];
        let count = broadcast_mgr.broadcast_to_room(1, &message).await.unwrap();
        
        // Should send to 3 players
        assert_eq!(count, 3);
        
        // Verify p4 did not receive
        assert!(!player_received_message(p4, &conn_mgr));
    }
    
    #[tokio::test]
    async fn test_room_transition() {
        let (player_mgr, world_mgr, broadcast_mgr) = setup_test_env().await;
        
        let player_id = create_test_player(&player_mgr, 1).await;
        
        // Change to room 2
        handle_room_change(
            player_id,
            2,
            &player_mgr,
            &world_mgr,
            &broadcast_mgr,
        ).await.unwrap();
        
        // Verify player is in room 2
        let player_arc = player_mgr.get_player(player_id).unwrap();
        let player = player_arc.read().await;
        assert_eq!(player.current_room, 2);
    }
}
```

## Summary

The Room System provides:
- ✅ **Room-based broadcasting** for movement, chat, and events
- ✅ **Global broadcasting** for announcements
- ✅ **Targeted messaging** to specific players
- ✅ **Player introduction** protocol (MSG_NEW_PLAYER)
- ✅ **Room state synchronization** (collectibles, plants, etc.)
- ✅ **Broadcast optimization** with batching and deduplication
- ✅ **Visibility control** - players only see same-room players

**Key Design Decisions:**
- Room membership derived from Player.current_room
- Two-case MSG_NEW_PLAYER protocol (with/without response)
- Broadcast excluding sender for movement/chat
- Full room state sent on room entry
- No persistent room player list (computed on demand)

**Broadcasting Patterns:**
- **Movement:** Room broadcast except sender
- **Chat:** Room broadcast including sender
- **Global:** All online players
- **Targeted:** Specific player list

**Next:** See [`06-event-system.md`](06-event-system.md) for event-driven message handling architecture.
