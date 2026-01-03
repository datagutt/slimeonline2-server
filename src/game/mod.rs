//! Game state management for Slime Online 2
//!
//! Note: Dropped items (player-discarded) are stored in the database (ground_items table),
//! not in memory. See `src/db/runtime_state.rs` for ground item persistence.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use tokio::sync::{Notify, RwLock};
use uuid::Uuid;

use crate::config::DefaultsConfig;
use crate::constants::*;

/// Handle to a player session with notification support
/// 
/// This wraps a PlayerSession with a Notify that can wake up the connection
/// handler when messages are queued for sending.
pub struct SessionHandle {
    pub session: Arc<RwLock<PlayerSession>>,
    pub notify: Arc<Notify>,
}

impl SessionHandle {
    pub fn new(session: PlayerSession) -> Self {
        Self {
            session: Arc::new(RwLock::new(session)),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Queue a message and notify the connection handler to send it
    pub async fn queue_message(&self, message: Vec<u8>) {
        self.session.write().await.queue_message(message);
        self.notify.notify_one();
    }
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
    /// Collectibles in this room (spawn points with their current state)
    pub collectibles: RwLock<HashMap<u8, ActiveCollectible>>,
}

impl Room {
    pub fn new(id: u16) -> Self {
        Self {
            id,
            players: RwLock::new(HashSet::new()),
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

    /// Initialize collectibles with database state for persistence across restarts
    pub async fn init_collectibles_with_state(
        &self,
        spawns: Vec<CollectibleSpawn>,
        db_states: HashMap<u8, crate::db::CollectibleState>,
    ) {
        use chrono::{DateTime, Utc};

        let mut collectibles = self.collectibles.write().await;
        let now = Instant::now();

        for spawn in spawns {
            let col_id = spawn.col_id;

            // Check if we have DB state for this spawn
            let taken_at = if let Some(db_state) = db_states.get(&col_id) {
                if db_state.available == 0 {
                    // Collectible is taken - check if it should have respawned by now
                    if let Some(ref respawn_str) = db_state.respawn_at {
                        if let Ok(respawn_time) = DateTime::parse_from_rfc3339(respawn_str) {
                            let respawn_utc: DateTime<Utc> = respawn_time.into();
                            if respawn_utc > Utc::now() {
                                // Still waiting for respawn - mark as taken
                                // We approximate the taken_at time based on remaining wait
                                let remaining_secs =
                                    (respawn_utc - Utc::now()).num_seconds().max(0) as u64;
                                let total_respawn = spawn.respawn_secs.unwrap_or(3600) as u64;
                                let elapsed = total_respawn.saturating_sub(remaining_secs);
                                Some(now - std::time::Duration::from_secs(elapsed))
                            } else {
                                // Already respawned
                                None
                            }
                        } else {
                            None // Invalid date, treat as available
                        }
                    } else {
                        None // No respawn time, treat as available
                    }
                } else {
                    None // Available in DB
                }
            } else {
                None // No DB state, treat as available
            };

            collectibles.insert(col_id, ActiveCollectible { spawn, taken_at });
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

/// Pending clan invite
#[derive(Debug, Clone)]
pub struct PendingClanInvite {
    pub clan_id: i64,
    pub clan_name: String,
    pub inviter_id: u16,
    pub invited_at: Instant,
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
    pub is_moderator: bool,
    pub connected_at: Instant,
    pub last_activity: Instant,
    pub ip_address: String,
    /// Queue of messages to send to this player
    pub outgoing_messages: VecDeque<Vec<u8>>,
    /// Flag to disconnect this player (set by anti-cheat, etc.)
    pub should_disconnect: bool,
    /// Reason for disconnection (for logging)
    pub disconnect_reason: Option<String>,
    /// Player's current clan ID (cached, loaded from DB on login)
    pub clan_id: Option<i64>,
    /// Whether this player is the leader of their clan
    pub is_clan_leader: bool,
    /// Whether clan has a base
    pub has_clan_base: bool,
    /// Pending clan invite (only one at a time)
    pub pending_clan_invite: Option<PendingClanInvite>,
    /// Last time we sent an invite to each player (for 15s cooldown)
    pub clan_invite_cooldowns: HashMap<u16, Instant>,
    /// Current race ID (if in a race)
    pub race_id: Option<u8>,
    /// When the race started
    pub race_start_time: Option<Instant>,
    /// Checkpoints reached in current race
    pub race_checkpoints: Vec<u16>,
}

impl PlayerSession {
    /// Create a new player session with config-based defaults
    pub fn new(ip_address: String, defaults: &DefaultsConfig) -> Self {
        let now = Instant::now();
        Self {
            session_id: Uuid::new_v4(),
            player_id: None,
            account_id: None,
            character_id: None,
            username: None,
            room_id: defaults.spawn_room,
            x: defaults.spawn_x,
            y: defaults.spawn_y,
            body_id: defaults.outfit,
            acs1_id: defaults.accessory1,
            acs2_id: defaults.accessory2,
            points: DEFAULT_POINTS,
            is_authenticated: false,
            is_moderator: false,
            connected_at: now,
            last_activity: now,
            ip_address,
            outgoing_messages: VecDeque::new(),
            should_disconnect: false,
            disconnect_reason: None,
            clan_id: None,
            is_clan_leader: false,
            has_clan_base: false,
            pending_clan_invite: None,
            clan_invite_cooldowns: HashMap::new(),
            race_id: None,
            race_start_time: None,
            race_checkpoints: Vec::new(),
        }
    }

    /// Mark this session for disconnection
    pub fn kick(&mut self, reason: impl Into<String>) {
        self.should_disconnect = true;
        self.disconnect_reason = Some(reason.into());
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
}

impl GameState {
    pub fn new() -> Self {
        Self {
            rooms: DashMap::new(),
            players_by_id: DashMap::new(),
        }
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

    /// Initialize collectibles for a room from spawn definitions
    pub async fn init_room_collectibles(&self, room_id: u16, spawns: Vec<CollectibleSpawn>) {
        let room = self.get_or_create_room(room_id);
        room.init_collectibles(spawns).await;
    }

    /// Initialize collectibles for a room with database state for persistence
    pub async fn init_room_collectibles_with_state(
        &self,
        room_id: u16,
        spawns: Vec<CollectibleSpawn>,
        db_states: std::collections::HashMap<u8, crate::db::CollectibleState>,
    ) {
        let room = self.get_or_create_room(room_id);
        room.init_collectibles_with_state(spawns, db_states).await;
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
