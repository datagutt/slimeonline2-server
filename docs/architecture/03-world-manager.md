# World Manager

**Document Status:** Complete  
**Last Updated:** 2024-01-08  
**Related:** [`01-overview.md`](01-overview.md), [`04-player-manager.md`](04-player-manager.md), [`05-room-system.md`](05-room-system.md)

## Overview

The World Manager maintains the **server-side game world state**, including rooms, world objects, collectibles, plants, and discarded items. It provides the authoritative source of truth for all game state and coordinates between different game systems.

**Responsibilities:**
- ✅ Manage room instances and their state
- ✅ Track world objects (collectibles, plants, build spots)
- ✅ Handle discarded item persistence
- ✅ Coordinate player presence in rooms
- ✅ Persist world state to database
- ✅ Load world state on server startup
- ✅ Handle world object lifecycle (spawn, evolve, despawn)

## Architecture

```
┌────────────────────────────────────────────────────────┐
│                    World Manager                        │
├────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────────────────────────────────────────┐  │
│  │            Room Registry                         │  │
│  │  Arc<DashMap<RoomId, Arc<RwLock<Room>>>>       │  │
│  └─────────────────────────────────────────────────┘  │
│                          │                              │
│                          v                              │
│  ┌─────────────────────────────────────────────────┐  │
│  │              Room State                          │  │
│  │  - Players in room (HashSet<PlayerId>)          │  │
│  │  - Collectibles (HashMap<u8, Collectible>)      │  │
│  │  - Plant spots (HashMap<u8, PlantSpot>)         │  │
│  │  - Build spots (HashMap<u8, BuildSpot>)         │  │
│  │  - Discarded items (HashMap<u16, DiscardedItem>)│  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
│  ┌─────────────────────────────────────────────────┐  │
│  │          World Object Registry                   │  │
│  │  - Collectible tracker                          │  │
│  │  - Plant lifecycle manager                      │  │
│  │  - Build spot manager                           │  │
│  │  - Discarded item cleanup                       │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
└────────────────────────────────────────────────────────┘
```

## Data Structures

### World Manager

```rust
use dashmap::DashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

pub struct WorldManager {
    /// All rooms in the game world
    rooms: Arc<DashMap<RoomId, Arc<RwLock<Room>>>>,
    
    /// Global world state
    world_state: Arc<RwLock<WorldState>>,
    
    /// Database connection pool
    db_pool: sqlx::PgPool,
}

pub type RoomId = u16;

impl WorldManager {
    pub fn new(db_pool: sqlx::PgPool) -> Self {
        Self {
            rooms: Arc::new(DashMap::new()),
            world_state: Arc::new(RwLock::new(WorldState::new())),
            db_pool,
        }
    }
    
    /// Initialize all rooms on server startup
    pub async fn initialize(&self) -> Result<(), WorldError> {
        // Load rooms from database or create default rooms
        self.load_rooms().await?;
        
        // Load world objects (collectibles, plants, etc.)
        self.load_world_objects().await?;
        
        log::info!("World Manager initialized with {} rooms", self.rooms.len());
        
        Ok(())
    }
    
    /// Get or create room
    pub fn get_room(&self, room_id: RoomId) -> Arc<RwLock<Room>> {
        self.rooms.entry(room_id)
            .or_insert_with(|| Arc::new(RwLock::new(Room::new(room_id))))
            .clone()
    }
}
```

### Room

```rust
use std::collections::{HashMap, HashSet};

pub struct Room {
    pub id: RoomId,
    pub name: String,
    
    /// Players currently in this room
    pub players: HashSet<PlayerId>,
    
    /// Collectibles in this room (slot → collectible)
    pub collectibles: HashMap<u8, Collectible>,
    
    /// Plant spots in this room (slot → plant)
    pub plant_spots: HashMap<u8, PlantSpot>,
    
    /// Build spots in this room (slot → build)
    pub build_spots: HashMap<u8, BuildSpot>,
    
    /// Discarded items on ground (instance_id → item)
    pub discarded_items: HashMap<u16, DiscardedItem>,
    
    /// Room-specific NPCs
    pub npcs: Vec<Npc>,
    
    /// Shops in this room
    pub shops: Vec<Shop>,
    
    /// Room metadata
    pub metadata: RoomMetadata,
}

pub type PlayerId = u16;

impl Room {
    pub fn new(id: RoomId) -> Self {
        Self {
            id,
            name: format!("Room {}", id),
            players: HashSet::new(),
            collectibles: HashMap::new(),
            plant_spots: HashMap::new(),
            build_spots: HashMap::new(),
            discarded_items: HashMap::new(),
            npcs: Vec::new(),
            shops: Vec::new(),
            metadata: RoomMetadata::default(),
        }
    }
    
    /// Add player to room
    pub fn add_player(&mut self, player_id: PlayerId) {
        self.players.insert(player_id);
    }
    
    /// Remove player from room
    pub fn remove_player(&mut self, player_id: PlayerId) {
        self.players.remove(&player_id);
    }
    
    /// Get all players in room
    pub fn get_players(&self) -> Vec<PlayerId> {
        self.players.iter().copied().collect()
    }
    
    /// Check if room is empty
    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
    }
}

#[derive(Default)]
pub struct RoomMetadata {
    pub is_safe_zone: bool,      // No PvP/damage
    pub has_shops: bool,
    pub has_warpcenter: bool,
    pub max_players: Option<u16>,
}
```

### Collectible

```rust
pub struct Collectible {
    pub slot: u8,              // Position slot (0-255)
    pub collectible_id: u16,   // Type of collectible
    pub x: u16,
    pub y: u16,
    pub evolution_stage: u8,   // For evolving collectibles
    pub spawned_at: Instant,
}

impl Collectible {
    /// Check if collectible can evolve
    pub fn can_evolve(&self, elapsed: Duration) -> bool {
        // Collectibles evolve after certain time periods
        match self.evolution_stage {
            0 => elapsed > Duration::from_secs(300),  // 5 minutes → stage 1
            1 => elapsed > Duration::from_secs(600),  // 10 minutes → stage 2
            2 => elapsed > Duration::from_secs(900),  // 15 minutes → stage 3
            _ => false,
        }
    }
    
    /// Evolve to next stage
    pub fn evolve(&mut self) {
        if self.evolution_stage < 3 {
            self.evolution_stage += 1;
            self.spawned_at = Instant::now(); // Reset timer
        }
    }
}
```

### Plant Spot

```rust
pub struct PlantSpot {
    pub slot: u8,
    pub plant_type: u16,       // Tree type ID
    pub planted_by: PlayerId,
    pub planted_at: Instant,
    pub growth_stage: u8,      // 0-4 (seedling → full grown)
    pub has_fruit: bool,
    pub fruit_count: u8,
    pub has_pinwheel: bool,    // Decoration
    pub has_fairy: bool,       // Decoration
}

impl PlantSpot {
    pub fn new(slot: u8, plant_type: u16, planted_by: PlayerId) -> Self {
        Self {
            slot,
            plant_type,
            planted_by,
            planted_at: Instant::now(),
            growth_stage: 0,
            has_fruit: false,
            fruit_count: 0,
            has_pinwheel: false,
            has_fairy: false,
        }
    }
    
    /// Update plant growth
    pub fn update_growth(&mut self, elapsed: Duration) {
        // Growth stages every 10 minutes
        let minutes_elapsed = elapsed.as_secs() / 60;
        let new_stage = (minutes_elapsed / 10).min(4) as u8;
        
        if new_stage > self.growth_stage {
            self.growth_stage = new_stage;
            
            // Fully grown trees can produce fruit
            if self.growth_stage == 4 && !self.has_fruit {
                self.has_fruit = true;
                self.fruit_count = 3; // Default fruit count
            }
        }
    }
    
    /// Check if plant should die (wilted)
    pub fn should_die(&self, elapsed: Duration) -> bool {
        // Plants die after 7 days without care
        elapsed > Duration::from_secs(7 * 24 * 60 * 60)
    }
}
```

### Build Spot

```rust
pub struct BuildSpot {
    pub slot: u8,
    pub object_type: u16,      // Type of built object
    pub built_by: PlayerId,
    pub built_at: Instant,
    pub durability: u8,        // 0-100
}

impl BuildSpot {
    pub fn new(slot: u8, object_type: u16, built_by: PlayerId) -> Self {
        Self {
            slot,
            object_type,
            built_by,
            built_at: Instant::now(),
            durability: 100,
        }
    }
    
    /// Decay durability over time
    pub fn decay(&mut self, elapsed: Duration) {
        let days = elapsed.as_secs() / (24 * 60 * 60);
        self.durability = self.durability.saturating_sub((days * 5) as u8);
    }
    
    /// Check if object should be removed
    pub fn is_broken(&self) -> bool {
        self.durability == 0
    }
}
```

### Discarded Item

```rust
pub struct DiscardedItem {
    pub instance_id: u16,      // Unique per room
    pub item_id: u16,
    pub quantity: u16,
    pub x: u16,
    pub y: u16,
    pub discarded_by: PlayerId,
    pub discarded_at: Instant,
}

impl DiscardedItem {
    pub fn new(
        instance_id: u16,
        item_id: u16,
        quantity: u16,
        x: u16,
        y: u16,
        discarded_by: PlayerId,
    ) -> Self {
        Self {
            instance_id,
            item_id,
            quantity,
            x,
            y,
            discarded_by,
            discarded_at: Instant::now(),
        }
    }
    
    /// Check if item should despawn (30 minutes)
    pub fn should_despawn(&self) -> bool {
        self.discarded_at.elapsed() > Duration::from_secs(30 * 60)
    }
}
```

## Room Management

### Loading Rooms

```rust
impl WorldManager {
    /// Load all rooms from database
    async fn load_rooms(&self) -> Result<(), WorldError> {
        // Load room definitions
        let room_records = sqlx::query!(
            "SELECT room_id, room_name FROM rooms ORDER BY room_id"
        )
        .fetch_all(&self.db_pool)
        .await?;
        
        for record in room_records {
            let mut room = Room::new(record.room_id as u16);
            room.name = record.room_name;
            
            self.rooms.insert(record.room_id as u16, Arc::new(RwLock::new(room)));
        }
        
        // If no rooms in DB, create default rooms
        if self.rooms.is_empty() {
            self.create_default_rooms().await?;
        }
        
        Ok(())
    }
    
    /// Create default room set
    async fn create_default_rooms(&self) -> Result<(), WorldError> {
        let default_rooms = vec![
            (32, "City Mountain Feet 1"),
            (33, "City Mountain Feet 2"),
            (34, "City Mountain Feet 3"),
            (35, "City Mountain Side 1"),
            (36, "City Mountain Side 2"),
            (37, "City Mountain Top 1"),
            (38, "City Mountain Cave 1"),
            (39, "City Mountain Side 3"),
            (40, "City Mountain Feet 4"),
            (41, "Green Valley 1"),
            (42, "New City"),
            (43, "City Mountain Top 2"),
            (44, "New City Outfits"),
            (45, "New City Accessories"),
            (46, "New City Items"),
            (47, "Underground Cave 1"),
            (48, "Underground Cave 2"),
            (51, "New City Warpcenter"),
        ];
        
        for (room_id, room_name) in default_rooms {
            let mut room = Room::new(room_id);
            room.name = room_name.to_string();
            
            // Set metadata based on room type
            if room_name.contains("Shop") || room_name.contains("Outfits") 
                || room_name.contains("Accessories") || room_name.contains("Items") {
                room.metadata.has_shops = true;
                room.metadata.is_safe_zone = true;
            }
            
            if room_name.contains("Warpcenter") {
                room.metadata.has_warpcenter = true;
                room.metadata.is_safe_zone = true;
            }
            
            self.rooms.insert(room_id, Arc::new(RwLock::new(room)));
        }
        
        Ok(())
    }
}
```

### World Object Management

```rust
impl WorldManager {
    /// Load world objects from database
    async fn load_world_objects(&self) -> Result<(), WorldError> {
        // Load collectibles
        self.load_collectibles().await?;
        
        // Load plants
        self.load_plants().await?;
        
        // Load build spots
        self.load_build_spots().await?;
        
        Ok(())
    }
    
    /// Load collectibles into rooms
    async fn load_collectibles(&self) -> Result<(), WorldError> {
        let records = sqlx::query!(
            r#"
            SELECT room_id, slot, collectible_id, x, y, 
                   evolution_stage, spawned_at
            FROM collectibles
            WHERE despawned_at IS NULL
            "#
        )
        .fetch_all(&self.db_pool)
        .await?;
        
        for record in records {
            if let Some(room) = self.rooms.get(&(record.room_id as u16)) {
                let mut room = room.write().await;
                
                let collectible = Collectible {
                    slot: record.slot as u8,
                    collectible_id: record.collectible_id as u16,
                    x: record.x as u16,
                    y: record.y as u16,
                    evolution_stage: record.evolution_stage as u8,
                    spawned_at: Instant::now() - Duration::from_secs(
                        record.spawned_at.timestamp() as u64
                    ),
                };
                
                room.collectibles.insert(record.slot as u8, collectible);
            }
        }
        
        Ok(())
    }
    
    /// Spawn new collectible in room
    pub async fn spawn_collectible(
        &self,
        room_id: RoomId,
        slot: u8,
        collectible_id: u16,
        x: u16,
        y: u16,
    ) -> Result<(), WorldError> {
        let room = self.get_room(room_id);
        let mut room = room.write().await;
        
        // Check if slot is already occupied
        if room.collectibles.contains_key(&slot) {
            return Err(WorldError::SlotOccupied);
        }
        
        let collectible = Collectible {
            slot,
            collectible_id,
            x,
            y,
            evolution_stage: 0,
            spawned_at: Instant::now(),
        };
        
        room.collectibles.insert(slot, collectible);
        
        // Persist to database
        sqlx::query!(
            r#"
            INSERT INTO collectibles 
            (room_id, slot, collectible_id, x, y, evolution_stage, spawned_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            "#,
            room_id as i16,
            slot as i16,
            collectible_id as i16,
            x as i16,
            y as i16,
            0i16,
        )
        .execute(&self.db_pool)
        .await?;
        
        Ok(())
    }
    
    /// Player takes collectible
    pub async fn take_collectible(
        &self,
        room_id: RoomId,
        slot: u8,
        player_id: PlayerId,
    ) -> Result<Collectible, WorldError> {
        let room = self.get_room(room_id);
        let mut room = room.write().await;
        
        let collectible = room.collectibles.remove(&slot)
            .ok_or(WorldError::CollectibleNotFound)?;
        
        // Mark as taken in database
        sqlx::query!(
            r#"
            UPDATE collectibles 
            SET despawned_at = NOW(), taken_by = $1
            WHERE room_id = $2 AND slot = $3
            "#,
            player_id as i16,
            room_id as i16,
            slot as i16,
        )
        .execute(&self.db_pool)
        .await?;
        
        Ok(collectible)
    }
}
```

### Plant Management

```rust
impl WorldManager {
    /// Plant a tree at spot
    pub async fn plant_tree(
        &self,
        room_id: RoomId,
        slot: u8,
        plant_type: u16,
        player_id: PlayerId,
    ) -> Result<(), WorldError> {
        let room = self.get_room(room_id);
        let mut room = room.write().await;
        
        // Check if slot is free
        if room.plant_spots.contains_key(&slot) {
            return Err(WorldError::SlotOccupied);
        }
        
        let plant = PlantSpot::new(slot, plant_type, player_id);
        room.plant_spots.insert(slot, plant);
        
        // Persist to database
        sqlx::query!(
            r#"
            INSERT INTO plants 
            (room_id, slot, plant_type, planted_by, planted_at, growth_stage)
            VALUES ($1, $2, $3, $4, NOW(), 0)
            "#,
            room_id as i16,
            slot as i16,
            plant_type as i16,
            player_id as i16,
        )
        .execute(&self.db_pool)
        .await?;
        
        Ok(())
    }
    
    /// Update plant growth
    pub async fn update_plant_growth(&self, room_id: RoomId, slot: u8) -> Result<(), WorldError> {
        let room = self.get_room(room_id);
        let mut room = room.write().await;
        
        let plant = room.plant_spots.get_mut(&slot)
            .ok_or(WorldError::PlantNotFound)?;
        
        let elapsed = plant.planted_at.elapsed();
        let old_stage = plant.growth_stage;
        
        plant.update_growth(elapsed);
        
        // If stage changed, update database
        if plant.growth_stage != old_stage {
            sqlx::query!(
                r#"
                UPDATE plants 
                SET growth_stage = $1, has_fruit = $2, fruit_count = $3
                WHERE room_id = $4 AND slot = $5
                "#,
                plant.growth_stage as i16,
                plant.has_fruit,
                plant.fruit_count as i16,
                room_id as i16,
                slot as i16,
            )
            .execute(&self.db_pool)
            .await?;
        }
        
        Ok(())
    }
}
```

## Background Tasks

### World Tick Loop

```rust
pub async fn world_tick_loop(world_manager: Arc<WorldManager>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60)); // Every minute
    
    loop {
        interval.tick().await;
        
        // Update all world objects
        if let Err(e) = world_manager.tick_world().await {
            log::error!("World tick error: {}", e);
        }
    }
}

impl WorldManager {
    /// Update world state (called every minute)
    pub async fn tick_world(&self) -> Result<(), WorldError> {
        let room_ids: Vec<RoomId> = self.rooms.iter()
            .map(|entry| *entry.key())
            .collect();
        
        for room_id in room_ids {
            self.tick_room(room_id).await?;
        }
        
        Ok(())
    }
    
    /// Update single room
    async fn tick_room(&self, room_id: RoomId) -> Result<(), WorldError> {
        let room = self.get_room(room_id);
        let mut room = room.write().await;
        
        // Update collectible evolution
        for (slot, collectible) in &mut room.collectibles {
            let elapsed = collectible.spawned_at.elapsed();
            
            if collectible.can_evolve(elapsed) {
                collectible.evolve();
                
                // Broadcast evolution to players in room
                // (handled by broadcast manager)
            }
        }
        
        // Update plant growth
        for (slot, plant) in &mut room.plant_spots {
            let elapsed = plant.planted_at.elapsed();
            let old_stage = plant.growth_stage;
            
            plant.update_growth(elapsed);
            
            if plant.growth_stage != old_stage {
                // Broadcast growth update
            }
            
            // Check for plant death
            if plant.should_die(elapsed) {
                // Remove plant
                // Broadcast removal
            }
        }
        
        // Clean up old discarded items
        let now = Instant::now();
        room.discarded_items.retain(|_, item| {
            !item.should_despawn()
        });
        
        Ok(())
    }
}
```

## Error Handling

```rust
#[derive(Debug)]
pub enum WorldError {
    RoomNotFound,
    SlotOccupied,
    CollectibleNotFound,
    PlantNotFound,
    BuildSpotNotFound,
    DatabaseError(sqlx::Error),
    InvalidOperation,
}

impl From<sqlx::Error> for WorldError {
    fn from(e: sqlx::Error) -> Self {
        WorldError::DatabaseError(e)
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
    fn test_room_player_management() {
        let mut room = Room::new(1);
        
        room.add_player(100);
        room.add_player(101);
        assert_eq!(room.players.len(), 2);
        
        room.remove_player(100);
        assert_eq!(room.players.len(), 1);
        assert!(!room.is_empty());
        
        room.remove_player(101);
        assert!(room.is_empty());
    }
    
    #[test]
    fn test_collectible_evolution() {
        let mut collectible = Collectible {
            slot: 0,
            collectible_id: 1,
            x: 100,
            y: 100,
            evolution_stage: 0,
            spawned_at: Instant::now() - Duration::from_secs(400),
        };
        
        assert!(collectible.can_evolve(Duration::from_secs(400)));
        collectible.evolve();
        assert_eq!(collectible.evolution_stage, 1);
    }
    
    #[test]
    fn test_plant_growth() {
        let mut plant = PlantSpot::new(0, 1, 100);
        
        // Simulate 25 minutes
        plant.update_growth(Duration::from_secs(25 * 60));
        assert_eq!(plant.growth_stage, 2); // Stage 2 at 20 minutes
        
        // Simulate 45 minutes total
        plant.update_growth(Duration::from_secs(45 * 60));
        assert_eq!(plant.growth_stage, 4); // Fully grown
        assert!(plant.has_fruit);
    }
}
```

## Summary

The World Manager provides:
- ✅ **Room management** with player tracking
- ✅ **Collectible system** with evolution over time
- ✅ **Plant system** with growth stages and fruit
- ✅ **Build spots** with durability decay
- ✅ **Discarded items** with auto-cleanup
- ✅ **Database persistence** for all world state
- ✅ **Background tasks** for world updates
- ✅ **Concurrent access** via Arc<DashMap> and RwLock

**Key Design Decisions:**
- DashMap for lock-free room lookup
- RwLock per room for fine-grained locking
- Instant-based timers for growth/evolution
- Database persistence for world objects
- Automatic cleanup of old items (30 minutes)
- Growth stages every 10 minutes for plants

**Next:** See [`04-player-manager.md`](04-player-manager.md) for player session management.
