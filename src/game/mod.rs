//! Game state management for Slime Online 2

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::constants::*;

/// A dropped item on the ground (player-discarded)
#[derive(Debug, Clone)]
pub struct DroppedItem {
    pub instance_id: u16,
    pub item_id: u16,
    pub x: u16,
    pub y: u16,
    pub dropped_at: Instant,
}

/// A collectible spawn point (static world collectibles)
#[derive(Debug, Clone)]
pub struct CollectibleSpawn {
    /// Unique ID for this spawn point within a room (0-255)
    pub col_id: u8,
    /// The item ID this spawn point gives
    pub item_id: u16,
    /// X position in room
    pub x: u16,
    /// Y position in room
    pub y: u16,
    /// Respawn delay in seconds (None = never respawns)
    pub respawn_secs: Option<u32>,
}

/// Active collectible state in a room
#[derive(Debug, Clone)]
pub struct ActiveCollectible {
    /// Reference to spawn definition
    pub spawn: CollectibleSpawn,
    /// When this collectible was taken (None = available)
    pub taken_at: Option<Instant>,
}

/// Room state
#[derive(Debug)]
pub struct Room {
    pub id: u16,
    pub players: RwLock<HashSet<u16>>,
    /// Items dropped on the ground in this room (player-discarded)
    pub dropped_items: RwLock<HashMap<u16, DroppedItem>>,
    /// Collectibles in this room (spawn points with their current state)
    pub collectibles: RwLock<HashMap<u8, ActiveCollectible>>,
}

impl Room {
    pub fn new(id: u16) -> Self {
        Self {
            id,
            players: RwLock::new(HashSet::new()),
            dropped_items: RwLock::new(HashMap::new()),
            collectibles: RwLock::new(HashMap::new()),
        }
    }

    pub async fn add_player(&self, player_id: u16) {
        self.players.write().await.insert(player_id);
    }

    pub async fn remove_player(&self, player_id: u16) {
        self.players.write().await.remove(&player_id);
    }

    pub async fn player_count(&self) -> usize {
        self.players.read().await.len()
    }

    pub async fn get_players(&self) -> Vec<u16> {
        self.players.read().await.iter().copied().collect()
    }

    /// Add a dropped item to the room
    pub async fn add_dropped_item(&self, item: DroppedItem) {
        self.dropped_items
            .write()
            .await
            .insert(item.instance_id, item);
    }

    /// Remove and return a dropped item by instance ID
    pub async fn take_dropped_item(&self, instance_id: u16) -> Option<DroppedItem> {
        self.dropped_items.write().await.remove(&instance_id)
    }

    /// Get all dropped items in the room
    pub async fn get_dropped_items(&self) -> Vec<DroppedItem> {
        self.dropped_items.read().await.values().cloned().collect()
    }

    /// Initialize collectibles for this room from spawn definitions
    pub async fn init_collectibles(&self, spawns: Vec<CollectibleSpawn>) {
        let mut collectibles = self.collectibles.write().await;
        for spawn in spawns {
            collectibles.insert(
                spawn.col_id,
                ActiveCollectible {
                    spawn,
                    taken_at: None,
                },
            );
        }
    }

    /// Get all available (not taken) collectibles in the room
    pub async fn get_available_collectibles(&self) -> Vec<ActiveCollectible> {
        let mut collectibles = self.collectibles.write().await;
        let mut available = Vec::new();

        for (_, col) in collectibles.iter_mut() {
            // Check if it's available or has respawned
            if let Some(taken_at) = col.taken_at {
                if let Some(respawn_secs) = col.spawn.respawn_secs {
                    if taken_at.elapsed().as_secs() >= respawn_secs as u64 {
                        // Respawned!
                        col.taken_at = None;
                    }
                }
            }

            if col.taken_at.is_none() {
                available.push(col.clone());
            }
        }

        available
    }

    /// Take a collectible by ID, returns the collectible spawn info if successful
    pub async fn take_collectible(&self, col_id: u8) -> Option<CollectibleSpawn> {
        let mut collectibles = self.collectibles.write().await;
        let now = Instant::now();

        if let Some(col) = collectibles.get_mut(&col_id) {
            // Check if it's available (or respawned)
            if let Some(taken_at) = col.taken_at {
                if let Some(respawn_secs) = col.spawn.respawn_secs {
                    if taken_at.elapsed().as_secs() >= respawn_secs as u64 {
                        // Respawned, allow taking
                        col.taken_at = Some(now);
                        return Some(col.spawn.clone());
                    }
                }
                // Already taken and not respawned
                return None;
            }

            // Available - take it
            col.taken_at = Some(now);
            Some(col.spawn.clone())
        } else {
            None
        }
    }
}

/// Player session state (in-memory only, not persisted)
#[derive(Debug)]
pub struct PlayerSession {
    pub session_id: Uuid,
    pub player_id: Option<u16>,
    pub account_id: Option<i64>,
    pub character_id: Option<i64>,
    pub username: Option<String>,
    pub room_id: u16,
    pub x: u16,
    pub y: u16,
    pub body_id: u16,
    pub acs1_id: u16,
    pub acs2_id: u16,
    pub points: u32,
    pub is_authenticated: bool,
    pub connected_at: Instant,
    pub last_activity: Instant,
    pub ip_address: String,
    /// Queue of messages to send to this player
    pub outgoing_messages: VecDeque<Vec<u8>>,
}

impl PlayerSession {
    pub fn new(ip_address: String) -> Self {
        let now = Instant::now();
        Self {
            session_id: Uuid::new_v4(),
            player_id: None,
            account_id: None,
            character_id: None,
            username: None,
            room_id: DEFAULT_SPAWN_ROOM,
            x: DEFAULT_SPAWN_X,
            y: DEFAULT_SPAWN_Y,
            body_id: DEFAULT_BODY_ID,
            acs1_id: 0,
            acs2_id: 0,
            points: DEFAULT_POINTS,
            is_authenticated: false,
            connected_at: now,
            last_activity: now,
            ip_address,
            outgoing_messages: VecDeque::new(),
        }
    }

    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn seconds_since_activity(&self) -> u64 {
        self.last_activity.elapsed().as_secs()
    }

    pub fn is_timed_out(&self) -> bool {
        if self.is_authenticated {
            self.seconds_since_activity() > CONNECTION_TIMEOUT_SECS
        } else {
            self.seconds_since_activity() > UNAUTHENTICATED_TIMEOUT_SECS
        }
    }

    /// Queue a message to be sent to this player
    pub fn queue_message(&mut self, message: Vec<u8>) {
        self.outgoing_messages.push_back(message);
    }

    /// Get all pending messages and clear the queue
    pub fn drain_messages(&mut self) -> Vec<Vec<u8>> {
        self.outgoing_messages.drain(..).collect()
    }

    /// Check if there are pending messages
    pub fn has_pending_messages(&self) -> bool {
        !self.outgoing_messages.is_empty()
    }
}

/// Global game state
pub struct GameState {
    pub rooms: DashMap<u16, Arc<Room>>,
    pub players_by_id: DashMap<u16, Uuid>, // player_id -> session_id
    /// Counter for generating unique dropped item instance IDs
    next_dropped_item_id: AtomicU16,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            rooms: DashMap::new(),
            players_by_id: DashMap::new(),
            next_dropped_item_id: AtomicU16::new(1),
        }
    }

    /// Generate a unique instance ID for a dropped item
    fn next_instance_id(&self) -> u16 {
        self.next_dropped_item_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Get or create a room
    pub fn get_or_create_room(&self, room_id: u16) -> Arc<Room> {
        self.rooms
            .entry(room_id)
            .or_insert_with(|| Arc::new(Room::new(room_id)))
            .clone()
    }

    /// Get a room if it exists
    pub fn get_room(&self, room_id: u16) -> Option<Arc<Room>> {
        self.rooms.get(&room_id).map(|r| r.clone())
    }

    /// Add player to a room
    pub async fn add_player_to_room(&self, player_id: u16, room_id: u16, session_id: Uuid) {
        let room = self.get_or_create_room(room_id);
        room.add_player(player_id).await;
        self.players_by_id.insert(player_id, session_id);
    }

    /// Remove player from a room
    pub async fn remove_player_from_room(&self, player_id: u16, room_id: u16) {
        if let Some(room) = self.get_room(room_id) {
            room.remove_player(player_id).await;
        }
        self.players_by_id.remove(&player_id);
    }

    /// Get all players in a room
    pub async fn get_room_players(&self, room_id: u16) -> Vec<u16> {
        if let Some(room) = self.get_room(room_id) {
            room.get_players().await
        } else {
            Vec::new()
        }
    }

    /// Add a dropped item to a room and return the instance ID
    pub async fn add_dropped_item(&self, room_id: u16, x: u16, y: u16, item_id: u16) -> u16 {
        let instance_id = self.next_instance_id();
        let room = self.get_or_create_room(room_id);
        let item = DroppedItem {
            instance_id,
            item_id,
            x,
            y,
            dropped_at: Instant::now(),
        };
        room.add_dropped_item(item).await;
        instance_id
    }

    /// Add a dropped item with a specific instance ID (for putting items back)
    pub async fn add_dropped_item_with_id(
        &self,
        room_id: u16,
        x: u16,
        y: u16,
        item_id: u16,
        instance_id: u16,
    ) {
        let room = self.get_or_create_room(room_id);
        let item = DroppedItem {
            instance_id,
            item_id,
            x,
            y,
            dropped_at: Instant::now(),
        };
        room.add_dropped_item(item).await;
    }

    /// Take a dropped item from a room
    pub async fn take_dropped_item(&self, room_id: u16, instance_id: u16) -> Option<DroppedItem> {
        if let Some(room) = self.get_room(room_id) {
            room.take_dropped_item(instance_id).await
        } else {
            None
        }
    }

    /// Get all dropped items in a room
    pub async fn get_dropped_items(&self, room_id: u16) -> Vec<DroppedItem> {
        if let Some(room) = self.get_room(room_id) {
            room.get_dropped_items().await
        } else {
            Vec::new()
        }
    }

    /// Initialize collectibles for a room from spawn definitions
    pub async fn init_room_collectibles(&self, room_id: u16, spawns: Vec<CollectibleSpawn>) {
        let room = self.get_or_create_room(room_id);
        room.init_collectibles(spawns).await;
    }

    /// Get all available collectibles in a room
    pub async fn get_available_collectibles(&self, room_id: u16) -> Vec<ActiveCollectible> {
        if let Some(room) = self.get_room(room_id) {
            room.get_available_collectibles().await
        } else {
            Vec::new()
        }
    }

    /// Take a collectible from a room
    pub async fn take_collectible(&self, room_id: u16, col_id: u8) -> Option<CollectibleSpawn> {
        if let Some(room) = self.get_room(room_id) {
            room.take_collectible(col_id).await
        } else {
            None
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}
