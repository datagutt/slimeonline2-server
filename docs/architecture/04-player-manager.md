# Player Manager

**Document Status:** Complete  
**Last Updated:** 2024-01-08  
**Related:** [`01-overview.md`](01-overview.md), [`02-connection-manager.md`](02-connection-manager.md), [`03-world-manager.md`](03-world-manager.md)

## Overview

The Player Manager maintains **active player sessions** and coordinates between the connection layer, game state, and database. It's responsible for loading/saving player data, managing online player state, and tracking player presence across rooms.

**Responsibilities:**
- ✅ Load player data from database on login
- ✅ Save player data to database (periodically and on logout)
- ✅ Track online players (PlayerId → Player mapping)
- ✅ Manage player inventory and equipment
- ✅ Handle player movement between rooms
- ✅ Track player stats (points, bank balance, trees planted, etc.)
- ✅ Coordinate with World Manager for room presence
- ✅ Provide player lookup for message handlers

## Architecture

```
┌────────────────────────────────────────────────────────┐
│                   Player Manager                        │
├────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────────────────────────────────────────┐  │
│  │         Online Players Registry                  │  │
│  │  Arc<DashMap<PlayerId, Arc<RwLock<Player>>>>   │  │
│  └─────────────────────────────────────────────────┘  │
│                          │                              │
│                          v                              │
│  ┌─────────────────────────────────────────────────┐  │
│  │              Player Session                      │  │
│  │  - Account ID                                    │  │
│  │  - Character data (name, appearance, position)  │  │
│  │  - Inventory (items, outfits, accessories)      │  │
│  │  - Stats (points, bank, quest state)            │  │
│  │  - Movement state (ileft, iright, etc.)         │  │
│  │  - Connection ID                                 │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
│  ┌─────────────────────────────────────────────────┐  │
│  │         Account → Player Mapping                 │  │
│  │  Arc<DashMap<AccountId, PlayerId>>              │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
│  ┌─────────────────────────────────────────────────┐  │
│  │         Auto-Save System                         │  │
│  │  - Periodic saves every 5 minutes                │  │
│  │  - Save on logout                                │  │
│  │  - Save on room change                           │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
└────────────────────────────────────────────────────────┘
```

## Data Structures

### Player Manager

```rust
use dashmap::DashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

pub struct PlayerManager {
    /// Online players (player_id → Player)
    players: Arc<DashMap<PlayerId, Arc<RwLock<Player>>>>,
    
    /// Account to player mapping (for duplicate login detection)
    account_to_player: Arc<DashMap<AccountId, PlayerId>>,
    
    /// Database connection pool
    db_pool: sqlx::PgPool,
    
    /// Next available player ID
    next_player_id: Arc<AtomicU16>,
}

pub type PlayerId = u16;
pub type AccountId = u32;

impl PlayerManager {
    pub fn new(db_pool: sqlx::PgPool) -> Self {
        Self {
            players: Arc::new(DashMap::new()),
            account_to_player: Arc::new(DashMap::new()),
            db_pool,
            next_player_id: Arc::new(AtomicU16::new(1)),
        }
    }
    
    /// Get online player by ID
    pub fn get_player(&self, player_id: PlayerId) -> Option<Arc<RwLock<Player>>> {
        self.players.get(&player_id).map(|entry| entry.clone())
    }
    
    /// Get all online player IDs
    pub fn get_online_players(&self) -> Vec<PlayerId> {
        self.players.iter().map(|entry| *entry.key()).collect()
    }
    
    /// Get online player count
    pub fn online_count(&self) -> usize {
        self.players.len()
    }
    
    /// Check if account is already logged in
    pub fn is_account_online(&self, account_id: AccountId) -> bool {
        self.account_to_player.contains_key(&account_id)
    }
}
```

### Player Session

```rust
use crate::protocol::MovementState;

pub struct Player {
    // Identity
    pub player_id: PlayerId,
    pub account_id: AccountId,
    pub character_id: u32,
    pub username: String,
    
    // Connection
    pub connection_id: ConnectionId,
    
    // Position & Room
    pub x: u16,
    pub y: u16,
    pub current_room: RoomId,
    
    // Appearance
    pub body_id: u16,
    pub acs1_id: u16,  // Accessory 1 (equipped)
    pub acs2_id: u16,  // Accessory 2 (equipped)
    
    // Movement state
    pub movement_state: MovementState,
    
    // Inventory
    pub inventory: Inventory,
    
    // Currency & Stats
    pub points: i32,
    pub bank_balance: i32,
    pub trees_planted: u16,
    pub objects_built: u16,
    
    // Quest State
    pub quest_id: u16,
    pub quest_step: u16,
    pub quest_var: u16,
    
    // Permissions
    pub has_signature: bool,
    pub is_moderator: bool,
    
    // Clan
    pub clan_id: Option<u32>,
    
    // Session metadata
    pub logged_in_at: Instant,
    pub last_save: Instant,
    pub last_ping_sent: Instant,
    pub last_ping_received: Instant,
    pub latency: Duration,
    
    // Flags
    pub can_move: bool,
    pub is_typing: bool,
}

impl Player {
    /// Create new player session from database data
    pub fn from_db_record(
        player_id: PlayerId,
        connection_id: ConnectionId,
        record: CharacterRecord,
    ) -> Self {
        Self {
            player_id,
            account_id: record.account_id as u32,
            character_id: record.id as u32,
            username: record.username,
            connection_id,
            x: record.x as u16,
            y: record.y as u16,
            current_room: record.room_id as u16,
            body_id: record.body_id as u16,
            acs1_id: record.acs1_id as u16,
            acs2_id: record.acs2_id as u16,
            movement_state: MovementState::default(),
            inventory: Inventory::default(),
            points: record.points,
            bank_balance: record.bank_balance,
            trees_planted: record.trees_planted as u16,
            objects_built: record.objects_built as u16,
            quest_id: record.quest_id as u16,
            quest_step: record.quest_step as u16,
            quest_var: record.quest_var as u16,
            has_signature: record.has_signature,
            is_moderator: record.is_moderator,
            clan_id: record.clan_id.map(|id| id as u32),
            logged_in_at: Instant::now(),
            last_save: Instant::now(),
            last_ping_sent: Instant::now(),
            last_ping_received: Instant::now(),
            latency: Duration::ZERO,
            can_move: true,
            is_typing: false,
        }
    }
    
    /// Check if player data needs saving
    pub fn needs_save(&self) -> bool {
        self.last_save.elapsed() > Duration::from_secs(300) // 5 minutes
    }
    
    /// Update position
    pub fn set_position(&mut self, x: u16, y: u16) {
        self.x = x;
        self.y = y;
    }
    
    /// Change room
    pub fn change_room(&mut self, new_room: RoomId) {
        self.current_room = new_room;
    }
}
```

### Inventory

```rust
pub struct Inventory {
    // Emotes (5 slots)
    pub emotes: [u16; 5],
    
    // Outfits (9 slots)
    pub outfits: [u16; 9],
    
    // Accessories (9 slots)
    pub accessories: [u16; 9],
    
    // Items (9 slots)
    pub items: [u16; 9],
    
    // Item quantities (9 slots)
    pub item_quantities: [u16; 9],
    
    // Tools (7 slots)
    pub tools: [u16; 7],
    
    // Equipped tool
    pub equipped_tool: Option<u8>, // Slot index (0-6)
}

impl Inventory {
    pub fn default() -> Self {
        Self {
            emotes: [0; 5],
            outfits: [0; 9],
            accessories: [0; 9],
            items: [0; 9],
            item_quantities: [0; 9],
            tools: [0; 7],
            equipped_tool: None,
        }
    }
    
    /// Add item to inventory
    pub fn add_item(&mut self, item_id: u16, quantity: u16) -> Result<(), InventoryError> {
        // Find existing stack
        for i in 0..9 {
            if self.items[i] == item_id {
                self.item_quantities[i] = self.item_quantities[i]
                    .saturating_add(quantity);
                return Ok(());
            }
        }
        
        // Find empty slot
        for i in 0..9 {
            if self.items[i] == 0 {
                self.items[i] = item_id;
                self.item_quantities[i] = quantity;
                return Ok(());
            }
        }
        
        Err(InventoryError::InventoryFull)
    }
    
    /// Remove item from inventory
    pub fn remove_item(&mut self, slot: u8, quantity: u16) -> Result<(), InventoryError> {
        let slot = slot as usize;
        
        if slot >= 9 {
            return Err(InventoryError::InvalidSlot);
        }
        
        if self.items[slot] == 0 {
            return Err(InventoryError::SlotEmpty);
        }
        
        if self.item_quantities[slot] < quantity {
            return Err(InventoryError::InsufficientQuantity);
        }
        
        self.item_quantities[slot] -= quantity;
        
        // Clear slot if empty
        if self.item_quantities[slot] == 0 {
            self.items[slot] = 0;
        }
        
        Ok(())
    }
    
    /// Check if player has item
    pub fn has_item(&self, item_id: u16, quantity: u16) -> bool {
        for i in 0..9 {
            if self.items[i] == item_id && self.item_quantities[i] >= quantity {
                return true;
            }
        }
        false
    }
}

#[derive(Debug)]
pub enum InventoryError {
    InventoryFull,
    InvalidSlot,
    SlotEmpty,
    InsufficientQuantity,
}
```

## Player Lifecycle

### 1. Login - Load Player Data

```rust
impl PlayerManager {
    /// Load player from database and create session
    pub async fn login_player(
        &self,
        account_id: AccountId,
        connection_id: ConnectionId,
    ) -> Result<PlayerId, PlayerError> {
        // Check if already logged in
        if let Some(existing_player_id) = self.account_to_player.get(&account_id) {
            log::warn!("Account {} already logged in as player {}", 
                      account_id, existing_player_id);
            return Err(PlayerError::AlreadyLoggedIn(*existing_player_id));
        }
        
        // Load character from database
        let character = self.load_character(account_id).await?;
        
        // Load inventory
        let inventory = self.load_inventory(character.id).await?;
        
        // Allocate player ID
        let player_id = self.next_player_id.fetch_add(1, Ordering::Relaxed);
        
        // Create player session
        let mut player = Player::from_db_record(player_id, connection_id, character);
        player.inventory = inventory;
        
        // Insert into registries
        self.players.insert(player_id, Arc::new(RwLock::new(player)));
        self.account_to_player.insert(account_id, player_id);
        
        log::info!("Player {} logged in (account {})", player_id, account_id);
        
        Ok(player_id)
    }
    
    /// Load character from database
    async fn load_character(&self, account_id: AccountId) -> Result<CharacterRecord, PlayerError> {
        let record = sqlx::query_as!(
            CharacterRecord,
            r#"
            SELECT id, account_id, username, x, y, room_id, 
                   body_id, acs1_id, acs2_id, 
                   points, bank_balance, trees_planted, objects_built,
                   quest_id, quest_step, quest_var,
                   has_signature, is_moderator, clan_id
            FROM characters
            WHERE account_id = $1
            "#,
            account_id as i32
        )
        .fetch_optional(&self.db_pool)
        .await?;
        
        record.ok_or(PlayerError::CharacterNotFound)
    }
    
    /// Load inventory from database
    async fn load_inventory(&self, character_id: u32) -> Result<Inventory, PlayerError> {
        let record = sqlx::query!(
            r#"
            SELECT emote_1, emote_2, emote_3, emote_4, emote_5,
                   outfit_1, outfit_2, outfit_3, outfit_4, outfit_5,
                   outfit_6, outfit_7, outfit_8, outfit_9,
                   accessory_1, accessory_2, accessory_3, accessory_4, accessory_5,
                   accessory_6, accessory_7, accessory_8, accessory_9,
                   item_1, item_2, item_3, item_4, item_5,
                   item_6, item_7, item_8, item_9,
                   item_1_qty, item_2_qty, item_3_qty, item_4_qty, item_5_qty,
                   item_6_qty, item_7_qty, item_8_qty, item_9_qty,
                   tool_1, tool_2, tool_3, tool_4, tool_5, tool_6, tool_7,
                   equipped_tool
            FROM inventories
            WHERE character_id = $1
            "#,
            character_id as i32
        )
        .fetch_optional(&self.db_pool)
        .await?;
        
        if let Some(inv) = record {
            Ok(Inventory {
                emotes: [
                    inv.emote_1 as u16, inv.emote_2 as u16, inv.emote_3 as u16,
                    inv.emote_4 as u16, inv.emote_5 as u16,
                ],
                outfits: [
                    inv.outfit_1 as u16, inv.outfit_2 as u16, inv.outfit_3 as u16,
                    inv.outfit_4 as u16, inv.outfit_5 as u16, inv.outfit_6 as u16,
                    inv.outfit_7 as u16, inv.outfit_8 as u16, inv.outfit_9 as u16,
                ],
                accessories: [
                    inv.accessory_1 as u16, inv.accessory_2 as u16, inv.accessory_3 as u16,
                    inv.accessory_4 as u16, inv.accessory_5 as u16, inv.accessory_6 as u16,
                    inv.accessory_7 as u16, inv.accessory_8 as u16, inv.accessory_9 as u16,
                ],
                items: [
                    inv.item_1 as u16, inv.item_2 as u16, inv.item_3 as u16,
                    inv.item_4 as u16, inv.item_5 as u16, inv.item_6 as u16,
                    inv.item_7 as u16, inv.item_8 as u16, inv.item_9 as u16,
                ],
                item_quantities: [
                    inv.item_1_qty as u16, inv.item_2_qty as u16, inv.item_3_qty as u16,
                    inv.item_4_qty as u16, inv.item_5_qty as u16, inv.item_6_qty as u16,
                    inv.item_7_qty as u16, inv.item_8_qty as u16, inv.item_9_qty as u16,
                ],
                tools: [
                    inv.tool_1 as u16, inv.tool_2 as u16, inv.tool_3 as u16,
                    inv.tool_4 as u16, inv.tool_5 as u16, inv.tool_6 as u16,
                    inv.tool_7 as u16,
                ],
                equipped_tool: inv.equipped_tool.map(|t| t as u8),
            })
        } else {
            // Create default inventory
            Ok(Inventory::default())
        }
    }
}
```

### 2. Save Player Data

```rust
impl PlayerManager {
    /// Save player data to database
    pub async fn save_player(&self, player_id: PlayerId) -> Result<(), PlayerError> {
        let player = self.get_player(player_id)
            .ok_or(PlayerError::PlayerNotFound)?;
        
        let player = player.read().await;
        
        // Update character
        sqlx::query!(
            r#"
            UPDATE characters SET
                x = $1, y = $2, room_id = $3,
                body_id = $4, acs1_id = $5, acs2_id = $6,
                points = $7, bank_balance = $8,
                trees_planted = $9, objects_built = $10,
                quest_id = $11, quest_step = $12, quest_var = $13,
                has_signature = $14, is_moderator = $15,
                clan_id = $16,
                updated_at = NOW()
            WHERE id = $17
            "#,
            player.x as i16,
            player.y as i16,
            player.current_room as i16,
            player.body_id as i16,
            player.acs1_id as i16,
            player.acs2_id as i16,
            player.points,
            player.bank_balance,
            player.trees_planted as i16,
            player.objects_built as i16,
            player.quest_id as i16,
            player.quest_step as i16,
            player.quest_var as i16,
            player.has_signature,
            player.is_moderator,
            player.clan_id.map(|id| id as i32),
            player.character_id as i32,
        )
        .execute(&self.db_pool)
        .await?;
        
        // Update inventory
        self.save_inventory(&player).await?;
        
        log::trace!("Saved player {} data", player_id);
        
        Ok(())
    }
    
    /// Save inventory to database
    async fn save_inventory(&self, player: &Player) -> Result<(), PlayerError> {
        let inv = &player.inventory;
        
        sqlx::query!(
            r#"
            UPDATE inventories SET
                emote_1 = $1, emote_2 = $2, emote_3 = $3, emote_4 = $4, emote_5 = $5,
                outfit_1 = $6, outfit_2 = $7, outfit_3 = $8, outfit_4 = $9, outfit_5 = $10,
                outfit_6 = $11, outfit_7 = $12, outfit_8 = $13, outfit_9 = $14,
                accessory_1 = $15, accessory_2 = $16, accessory_3 = $17, accessory_4 = $18,
                accessory_5 = $19, accessory_6 = $20, accessory_7 = $21, accessory_8 = $22,
                accessory_9 = $23,
                item_1 = $24, item_2 = $25, item_3 = $26, item_4 = $27, item_5 = $28,
                item_6 = $29, item_7 = $30, item_8 = $31, item_9 = $32,
                item_1_qty = $33, item_2_qty = $34, item_3_qty = $35, item_4_qty = $36,
                item_5_qty = $37, item_6_qty = $38, item_7_qty = $39, item_8_qty = $40,
                item_9_qty = $41,
                tool_1 = $42, tool_2 = $43, tool_3 = $44, tool_4 = $45, tool_5 = $46,
                tool_6 = $47, tool_7 = $48,
                equipped_tool = $49
            WHERE character_id = $50
            "#,
            inv.emotes[0] as i16, inv.emotes[1] as i16, inv.emotes[2] as i16,
            inv.emotes[3] as i16, inv.emotes[4] as i16,
            inv.outfits[0] as i16, inv.outfits[1] as i16, inv.outfits[2] as i16,
            inv.outfits[3] as i16, inv.outfits[4] as i16, inv.outfits[5] as i16,
            inv.outfits[6] as i16, inv.outfits[7] as i16, inv.outfits[8] as i16,
            inv.accessories[0] as i16, inv.accessories[1] as i16, inv.accessories[2] as i16,
            inv.accessories[3] as i16, inv.accessories[4] as i16, inv.accessories[5] as i16,
            inv.accessories[6] as i16, inv.accessories[7] as i16, inv.accessories[8] as i16,
            inv.items[0] as i16, inv.items[1] as i16, inv.items[2] as i16,
            inv.items[3] as i16, inv.items[4] as i16, inv.items[5] as i16,
            inv.items[6] as i16, inv.items[7] as i16, inv.items[8] as i16,
            inv.item_quantities[0] as i16, inv.item_quantities[1] as i16, inv.item_quantities[2] as i16,
            inv.item_quantities[3] as i16, inv.item_quantities[4] as i16, inv.item_quantities[5] as i16,
            inv.item_quantities[6] as i16, inv.item_quantities[7] as i16, inv.item_quantities[8] as i16,
            inv.tools[0] as i16, inv.tools[1] as i16, inv.tools[2] as i16,
            inv.tools[3] as i16, inv.tools[4] as i16, inv.tools[5] as i16,
            inv.tools[6] as i16,
            inv.equipped_tool.map(|t| t as i16),
            player.character_id as i32,
        )
        .execute(&self.db_pool)
        .await?;
        
        Ok(())
    }
}
```

### 3. Logout - Cleanup Session

```rust
impl PlayerManager {
    /// Handle player logout
    pub async fn logout_player(&self, player_id: PlayerId) -> Result<(), PlayerError> {
        // Save player data
        self.save_player(player_id).await?;
        
        // Remove from registries
        if let Some((_, player_arc)) = self.players.remove(&player_id) {
            let player = player_arc.read().await;
            
            // Remove account mapping
            self.account_to_player.remove(&player.account_id);
            
            log::info!("Player {} logged out (account {})", 
                      player_id, player.account_id);
        }
        
        Ok(())
    }
}
```

## Auto-Save System

```rust
/// Background task for auto-saving player data
pub async fn auto_save_loop(player_manager: Arc<PlayerManager>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60)); // Every minute
    
    loop {
        interval.tick().await;
        
        let player_ids = player_manager.get_online_players();
        
        for player_id in player_ids {
            if let Some(player_arc) = player_manager.get_player(player_id) {
                let needs_save = {
                    let player = player_arc.read().await;
                    player.needs_save()
                };
                
                if needs_save {
                    if let Err(e) = player_manager.save_player(player_id).await {
                        log::error!("Failed to auto-save player {}: {}", player_id, e);
                    } else {
                        // Update last_save timestamp
                        let mut player = player_arc.write().await;
                        player.last_save = Instant::now();
                    }
                }
            }
        }
    }
}
```

## Room Transitions

```rust
impl PlayerManager {
    /// Move player to different room
    pub async fn change_player_room(
        &self,
        player_id: PlayerId,
        new_room: RoomId,
        world_manager: &WorldManager,
    ) -> Result<(), PlayerError> {
        let player_arc = self.get_player(player_id)
            .ok_or(PlayerError::PlayerNotFound)?;
        
        let old_room = {
            let mut player = player_arc.write().await;
            let old_room = player.current_room;
            
            // Update player's room
            player.change_room(new_room);
            
            old_room
        };
        
        // Update world manager
        if old_room != new_room {
            // Remove from old room
            let old_room_arc = world_manager.get_room(old_room);
            let mut old_room_state = old_room_arc.write().await;
            old_room_state.remove_player(player_id);
            drop(old_room_state);
            
            // Add to new room
            let new_room_arc = world_manager.get_room(new_room);
            let mut new_room_state = new_room_arc.write().await;
            new_room_state.add_player(player_id);
        }
        
        // Save player data after room change
        self.save_player(player_id).await?;
        
        Ok(())
    }
}
```

## Player Queries

```rust
impl PlayerManager {
    /// Get players in specific room
    pub async fn get_players_in_room(&self, room_id: RoomId) -> Vec<PlayerId> {
        let mut result = Vec::new();
        
        for entry in self.players.iter() {
            let player = entry.value().read().await;
            if player.current_room == room_id {
                result.push(*entry.key());
            }
        }
        
        result
    }
    
    /// Get player by username
    pub async fn get_player_by_username(&self, username: &str) -> Option<PlayerId> {
        for entry in self.players.iter() {
            let player = entry.value().read().await;
            if player.username.eq_ignore_ascii_case(username) {
                return Some(*entry.key());
            }
        }
        None
    }
    
    /// Get players in clan
    pub async fn get_clan_members(&self, clan_id: u32) -> Vec<PlayerId> {
        let mut result = Vec::new();
        
        for entry in self.players.iter() {
            let player = entry.value().read().await;
            if player.clan_id == Some(clan_id) {
                result.push(*entry.key());
            }
        }
        
        result
    }
}
```

## Error Handling

```rust
#[derive(Debug)]
pub enum PlayerError {
    PlayerNotFound,
    CharacterNotFound,
    AlreadyLoggedIn(PlayerId),
    DatabaseError(sqlx::Error),
    InvalidData,
}

impl From<sqlx::Error> for PlayerError {
    fn from(e: sqlx::Error) -> Self {
        PlayerError::DatabaseError(e)
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_inventory_operations() {
        let mut inv = Inventory::default();
        
        // Add item
        assert!(inv.add_item(100, 5).is_ok());
        assert_eq!(inv.items[0], 100);
        assert_eq!(inv.item_quantities[0], 5);
        
        // Stack same item
        assert!(inv.add_item(100, 3).is_ok());
        assert_eq!(inv.item_quantities[0], 8);
        
        // Remove item
        assert!(inv.remove_item(0, 5).is_ok());
        assert_eq!(inv.item_quantities[0], 3);
        
        // Remove all
        assert!(inv.remove_item(0, 3).is_ok());
        assert_eq!(inv.items[0], 0);
    }
    
    #[tokio::test]
    async fn test_player_lifecycle() {
        let db_pool = create_test_db_pool().await;
        let manager = PlayerManager::new(db_pool);
        
        // Login
        let player_id = manager.login_player(1, Uuid::new_v4()).await.unwrap();
        assert_eq!(manager.online_count(), 1);
        
        // Duplicate login should fail
        assert!(manager.login_player(1, Uuid::new_v4()).await.is_err());
        
        // Logout
        manager.logout_player(player_id).await.unwrap();
        assert_eq!(manager.online_count(), 0);
    }
}
```

## Summary

The Player Manager provides:
- ✅ **Player session management** with online player tracking
- ✅ **Database integration** for loading/saving player data
- ✅ **Inventory management** with 32 total item slots
- ✅ **Auto-save system** every 5 minutes
- ✅ **Room transition coordination** with World Manager
- ✅ **Duplicate login detection** via account mapping
- ✅ **Player queries** by ID, username, room, clan
- ✅ **Concurrent access** via Arc<RwLock<Player>>

**Key Design Decisions:**
- DashMap for lock-free player lookup
- RwLock per player for fine-grained locking
- Separate account→player mapping for duplicate detection
- Auto-save every 5 minutes + on logout/room change
- Atomic player ID allocation
- Full inventory state in memory (not lazy loaded)

**Next:** See [`05-room-system.md`](05-room-system.md) for room-based broadcasting and player visibility.
