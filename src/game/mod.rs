//! Game state management for Slime Online 2

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::constants::*;

/// Room state
#[derive(Debug)]
pub struct Room {
    pub id: u16,
    pub players: RwLock<HashSet<u16>>,
}

impl Room {
    pub fn new(id: u16) -> Self {
        Self {
            id,
            players: RwLock::new(HashSet::new()),
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
    pub players_by_id: DashMap<u16, Uuid>,  // player_id -> session_id
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
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}
