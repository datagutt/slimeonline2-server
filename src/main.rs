//! Slime Online 2 Private Server
//!
//! A Rust implementation of the Slime Online 2 server for the v0.106 client.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use dashmap::DashMap;
use tokio::net::TcpListener;

use tracing::{error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};
use uuid::Uuid;

mod admin;
mod anticheat;
mod config;
mod constants;
mod crypto;
mod db;
mod game;
mod handlers;
mod protocol;
mod rate_limit;
mod validation;

use admin::{AdminAction, AdminState, InventoryCategory, PointsMode};
use config::GameConfig;
use constants::*;
use db::DbPool;
use game::{GameState};
use tokio::sync::mpsc;

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub motd: String,
    pub max_connections: usize,
    pub max_connections_per_ip: usize,
    /// If true, position/room is auto-saved on disconnect and periodically.
    /// If false, position/room is ONLY saved at manual save points (MSG_SAVE).
    /// Points and inventory are always auto-saved regardless of this setting.
    pub auto_save_position: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: DEFAULT_PORT,
            database_url: "sqlite:slime_online2.db?mode=rwc".to_string(),
            motd: "Welcome to Slime Online 2 Private Server!".to_string(),
            max_connections: MAX_TOTAL_CONNECTIONS,
            max_connections_per_ip: MAX_CONNECTIONS_PER_IP,
            auto_save_position: false, // Save points only by default
        }
    }
}

/// Shared server state
pub struct Server {
    pub config: ServerConfig,
    pub game_config: Arc<GameConfig>,
    pub db: DbPool,
    pub game_state: Arc<GameState>,
    pub sessions: Arc<DashMap<Uuid, Arc<game::SessionHandle>>>,
    pub connections_by_ip: Arc<DashMap<String, usize>>,
    pub active_player_ids: Arc<DashMap<u16, Uuid>>,
    next_player_id: Arc<std::sync::atomic::AtomicU16>,
    pub rate_limiter: Arc<rate_limit::RateLimiter>,
    pub anticheat: Arc<anticheat::AntiCheat>,
}

impl Server {
    /// Create a new server instance.
    pub async fn new(config: ServerConfig, game_config: GameConfig) -> Result<Self> {
        // Create database connection pool
        let db = db::create_pool(&config.database_url).await?;

        // Run migrations
        db::init_database(&db).await?;

        Ok(Self {
            config,
            game_config: Arc::new(game_config),
            db,
            game_state: Arc::new(GameState::new()),
            sessions: Arc::new(DashMap::new()),
            connections_by_ip: Arc::new(DashMap::new()),
            active_player_ids: Arc::new(DashMap::new()),
            next_player_id: Arc::new(std::sync::atomic::AtomicU16::new(1)),
            rate_limiter: Arc::new(rate_limit::RateLimiter::new()),
            anticheat: Arc::new(anticheat::AntiCheat::new()),
        })
    }

    /// Get the next available player ID.
    pub fn next_player_id(&self) -> u16 {
        self.next_player_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Check if a player ID is in use.
    pub fn is_player_online(&self, username: &str) -> bool {
        // Check all sessions for matching username
        for session_ref in self.sessions.iter() {
            if let Ok(session) = session_ref.value().session.try_read() {
                if session.username.as_deref() == Some(username) {
                    return true;
                }
            }
        }
        false
    }

    /// Get total connection count.
    pub fn connection_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get connection count for an IP.
    pub fn ip_connection_count(&self, ip: &str) -> usize {
        self.connections_by_ip.get(ip).map(|r| *r).unwrap_or(0)
    }

    /// Increment IP connection count.
    pub fn add_ip_connection(&self, ip: &str) {
        self.connections_by_ip
            .entry(ip.to_string())
            .and_modify(|c| *c += 1)
            .or_insert(1);
    }

    /// Decrement IP connection count.
    pub fn remove_ip_connection(&self, ip: &str) {
        if let Some(mut count) = self.connections_by_ip.get_mut(ip) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                drop(count);
                self.connections_by_ip.remove(ip);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    // Default to INFO, override with RUST_LOG env var (e.g., RUST_LOG=debug or RUST_LOG=trace)
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!(
        "Starting Slime Online 2 Server v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Load game configuration from TOML files
    let game_config = match GameConfig::load("config") {
        Ok(cfg) => {
            info!("Loaded game configuration from config/");
            cfg
        }
        Err(e) => {
            error!("Failed to load game configuration: {}", e);
            error!("Make sure the config/ directory exists with all required TOML files.");
            return Err(anyhow::anyhow!("Configuration error: {}", e));
        }
    };

    // Build server configuration from loaded config
    let srv = &game_config.server.server;
    let config = ServerConfig {
        host: srv.host.clone(),
        port: srv.port,
        database_url: format!("sqlite:{}?mode=rwc", srv.database_path),
        motd: srv.motd.clone(),
        max_connections: srv.max_connections,
        max_connections_per_ip: if srv.max_connections_per_ip > 0 {
            srv.max_connections_per_ip
        } else {
            MAX_CONNECTIONS_PER_IP
        },
        auto_save_position: srv.auto_save_position,
    };

    // Create server
    let server = Arc::new(Server::new(config.clone(), game_config.clone()).await?);

    info!("Database initialized");

    // Bind to address
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;

    info!("Server listening on {}", addr);
    info!("MOTD: {}", config.motd);

    // Spawn background tasks
    spawn_background_tasks(server.clone());

    // Start admin API if enabled
    let admin_config = &game_config.server.admin;
    if admin_config.enabled {
        if admin_config.api_key.is_empty()
            || admin_config.api_key == "change-me-in-production-use-openssl-rand-hex-32"
        {
            warn!("Admin API enabled but using default/empty API key! Set a secure key in config/server.toml");
        }

        // Create channel for admin actions
        let (action_tx, action_rx) = mpsc::channel::<AdminAction>(100);

        let admin_state = Arc::new(AdminState {
            db: server.db.clone(),
            api_key: admin_config.api_key.clone(),
            action_tx,
            sessions: server.sessions.clone(),
            game_state: server.game_state.clone(),
        });

        // Spawn admin API server
        let admin_host = admin_config.host.clone();
        let admin_port = admin_config.port;
        tokio::spawn(async move {
            if let Err(e) = admin::start_server(&admin_host, admin_port, admin_state).await {
                error!("Admin API server error: {}", e);
            }
        });

        // Spawn admin action handler
        spawn_admin_action_handler(server.clone(), action_rx);
    }

    // Setup shutdown signal handler
    let shutdown_server = server.clone();
    tokio::spawn(async move {
        // Wait for Ctrl+C
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");

        info!("Shutdown signal received, saving all player data...");

        // Save ALL player data on shutdown (position + points) regardless of config
        for session_ref in shutdown_server.sessions.iter() {
            let handle = session_ref.value();
            let session = handle.session.read().await;
            if let (Some(char_id), true) = (session.character_id, session.is_authenticated) {
                // Always save position on server shutdown
                if let Err(e) = db::update_position(
                    &shutdown_server.db,
                    char_id,
                    session.x as i16,
                    session.y as i16,
                    session.room_id as i16,
                )
                .await
                {
                    error!("Failed to save position for character {}: {}", char_id, e);
                }

                // Always save points
                if let Err(e) =
                    db::update_points(&shutdown_server.db, char_id, session.points as i64).await
                {
                    error!("Failed to save points for character {}: {}", char_id, e);
                }

                if let Some(username) = &session.username {
                    info!("Saved data for player {}", username);
                }
            }
        }

        info!("All player data saved. Shutting down.");
        std::process::exit(0);
    });

    // Accept connections
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                let server = server.clone();

                // Check connection limits
                if server.connection_count() >= server.config.max_connections {
                    warn!("Connection limit reached, rejecting {}", addr);
                    continue;
                }

                let ip = addr.ip().to_string();
                if server.ip_connection_count(&ip) >= server.config.max_connections_per_ip {
                    warn!("IP connection limit reached for {}", ip);
                    continue;
                }

                // Spawn handler task
                tokio::spawn(async move {
                    if let Err(e) = handlers::handle_connection(socket, addr, server).await {
                        error!("Connection handler error for {}: {}", addr, e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

/// Spawn background maintenance tasks.
fn spawn_background_tasks(server: Arc<Server>) {
    // Periodic save task - save all player data
    let save_server = server.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(SAVE_INTERVAL_SECS));
        let auto_save_position = save_server.config.auto_save_position;

        loop {
            interval.tick().await;
            info!("Running periodic save...");

            // Save all active player data
            for session_ref in save_server.sessions.iter() {
                let handle = session_ref.value();
                let session = handle.session.read().await;
                if let (Some(char_id), true) = (session.character_id, session.is_authenticated) {
                    // Only save position if auto_save_position is enabled
                    if auto_save_position {
                        if let Err(e) = db::update_position(
                            &save_server.db,
                            char_id,
                            session.x as i16,
                            session.y as i16,
                            session.room_id as i16,
                        )
                        .await
                        {
                            error!("Failed to save position for character {}: {}", char_id, e);
                        }
                    }

                    // Always save points
                    if let Err(e) =
                        db::update_points(&save_server.db, char_id, session.points as i64).await
                    {
                        error!("Failed to save points for character {}: {}", char_id, e);
                    }
                }
            }

            info!(
                "Periodic save complete. Active sessions: {}",
                save_server.sessions.len()
            );
        }
    });

    // Cleanup task for stale connections
    let cleanup_server = server.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(CLEANUP_INTERVAL_SECS));
        loop {
            interval.tick().await;

            // Find and remove stale sessions
            let mut stale_sessions = Vec::new();

            for session_ref in cleanup_server.sessions.iter() {
                let handle = session_ref.value();
                let session = handle.session.read().await;
                if session.is_timed_out() {
                    stale_sessions.push(*session_ref.key());
                }
            }

            for session_id in stale_sessions {
                if let Some((_, handle)) = cleanup_server.sessions.remove(&session_id) {
                    let session_guard = handle.session.read().await;
                    if let Some(username) = &session_guard.username {
                        info!("Cleaning up stale session for {}", username);
                    }

                    // Remove from room
                    if let Some(player_id) = session_guard.player_id {
                        cleanup_server
                            .game_state
                            .remove_player_from_room(player_id, session_guard.room_id)
                            .await;
                        cleanup_server.active_player_ids.remove(&player_id);
                    }

                    // Remove IP connection count
                    cleanup_server.remove_ip_connection(&session_guard.ip_address);
                }
            }
        }
    });

    // Collectible respawn task - check every 30 seconds for collectibles that should respawn
    let respawn_server = server.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;

            // Get all collectibles that need to respawn from DB
            match db::get_collectibles_to_respawn(&respawn_server.db).await {
                Ok(collectibles) => {
                    for col in collectibles {
                        let room_id = col.room_id as u16;
                        let spawn_id = col.spawn_id as u8;

                        // Mark as respawned in DB
                        if let Err(e) =
                            db::respawn_collectible(&respawn_server.db, room_id, spawn_id).await
                        {
                            error!(
                                "Failed to respawn collectible {}/{}: {}",
                                room_id, spawn_id, e
                            );
                            continue;
                        }

                        // Update in-memory state if room is loaded
                        if let Some(room) = respawn_server.game_state.get_room(room_id) {
                            let mut collectibles = room.collectibles.write().await;
                            if let Some(active_col) = collectibles.get_mut(&spawn_id) {
                                active_col.taken_at = None;
                            }
                        }

                        // Notify players in the room about the respawn
                        let room_players =
                            respawn_server.game_state.get_room_players(room_id).await;
                        if !room_players.is_empty() {
                            // Get the collectible info to send
                            if let Some(room) = respawn_server.game_state.get_room(room_id) {
                                let collectibles = room.collectibles.read().await;
                                if let Some(active_col) = collectibles.get(&spawn_id) {
                                    // Send MSG_COLLECTIBLE_INFO with just this one collectible
                                    let mut writer = protocol::MessageWriter::new();
                                    writer.write_u16(protocol::MessageType::CollectibleInfo.id());
                                    writer.write_u8(1); // count = 1
                                    writer.write_u8(active_col.spawn.col_id);
                                    writer.write_u16(active_col.spawn.item_id);
                                    writer.write_u16(active_col.spawn.x);
                                    writer.write_u16(active_col.spawn.y);
                                    let msg = writer.into_bytes();

                                    for player_id in room_players {
                                        if let Some(session_id) =
                                            respawn_server.game_state.players_by_id.get(&player_id)
                                        {
                                            if let Some(handle) =
                                                respawn_server.sessions.get(&session_id)
                                            {
                                                handle.queue_message(msg.clone()).await;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to check for collectible respawns: {}", e);
                }
            }

            // Cleanup expired ground items from DB, notifying clients first
            match db::get_expired_ground_items(&respawn_server.db).await {
                Ok(expired_items) => {
                    if !expired_items.is_empty() {
                        // Group expired items by room for efficient notification
                        use std::collections::HashMap;
                        let mut items_by_room: HashMap<u16, Vec<i64>> = HashMap::new();
                        for item in &expired_items {
                            items_by_room
                                .entry(item.room_id as u16)
                                .or_default()
                                .push(item.id);
                        }

                        // Notify players in each room about expired items
                        for (room_id, item_ids) in items_by_room {
                            let room_players =
                                respawn_server.game_state.get_room_players(room_id).await;
                            for db_id in &item_ids {
                                // instance_id sent to client is DB id cast to u16
                                let instance_id = *db_id as u16;
                                // Send MSG_DISCARDED_ITEM_TAKE to remove from client
                                let mut writer = protocol::MessageWriter::new();
                                writer.write_u16(protocol::MessageType::DiscardedItemTake.id());
                                writer.write_u16(instance_id);
                                let msg = writer.into_bytes();

                                for player_id in &room_players {
                                    if let Some(session_id) =
                                        respawn_server.game_state.players_by_id.get(player_id)
                                    {
                                        if let Some(handle) =
                                            respawn_server.sessions.get(&session_id)
                                        {
                                            handle.queue_message(msg.clone()).await;
                                        }
                                    }
                                }
                            }
                            tracing::debug!(
                                "Expired {} dropped items in room {}",
                                item_ids.len(),
                                room_id
                            );
                        }

                        // Now delete expired items from DB
                        match db::cleanup_expired_ground_items(&respawn_server.db).await {
                            Ok(count) => {
                                info!("Cleaned up {} expired ground items from DB", count);
                            }
                            Err(e) => {
                                error!("Failed to cleanup ground items: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to get expired ground items: {}", e);
                }
            }
        }
    });

    // Daily shop restock task - restocks all shops when day changes
    let restock_server = server.clone();
    tokio::spawn(async move {
        use chrono::Local;

        // Load last restock date from database on startup
        let mut last_restock_date: Option<String> = db::get_last_restock_date(&restock_server.db)
            .await
            .unwrap_or(None);

        let mut interval = tokio::time::interval(Duration::from_secs(60)); // Check every minute

        loop {
            interval.tick().await;

            let now = Local::now();
            let current_date = now.format("%Y-%m-%d").to_string(); // YYYY-MM-DD format

            // Check if we need to restock (new day compared to last restock)
            let should_restock = match &last_restock_date {
                None => {
                    // No record of previous restock - this is first run ever
                    // Store current date but don't restock (matches original server behavior)
                    if let Err(e) =
                        db::set_last_restock_date(&restock_server.db, &current_date).await
                    {
                        error!("Failed to set initial restock date: {}", e);
                    }
                    last_restock_date = Some(current_date.clone());
                    info!(
                        "First server run - initialized restock date to {}",
                        current_date
                    );
                    false
                }
                Some(last_date) => current_date != *last_date,
            };

            if should_restock {
                info!(
                    "Daily shop restock triggered (date {} -> {})",
                    last_restock_date.as_deref().unwrap_or("none"),
                    current_date
                );

                // Restock all shops from config
                let mut restocked_rooms = Vec::new();

                for (room_id, shop_config) in &restock_server.game_config.shops.rooms {
                    for (slot_idx, slot) in shop_config.slots.iter().enumerate() {
                        if slot.stock > 0 {
                            // Only restock limited items (stock > 0 means limited)
                            let slot_id = (slot_idx + 1) as u8;
                            if let Err(e) = db::restock_shop_slot(
                                &restock_server.db,
                                *room_id,
                                slot_id,
                                slot.stock,
                            )
                            .await
                            {
                                error!("Failed to restock shop {}/{}: {}", room_id, slot_id, e);
                            }
                        }
                    }
                    restocked_rooms.push(*room_id);
                }

                // Notify players in shop rooms about restock
                for room_id in restocked_rooms {
                    let room_players = restock_server.game_state.get_room_players(room_id).await;
                    if !room_players.is_empty() {
                        // Send MSG_SHOP_STOCK with code 2 (restocked)
                        let mut writer = protocol::MessageWriter::new();
                        writer.write_u16(protocol::MessageType::ShopStock.id());
                        writer.write_u8(2); // 2 = restocked
                        let msg = writer.into_bytes();

                        for player_id in room_players {
                            if let Some(session_id) =
                                restock_server.game_state.players_by_id.get(&player_id)
                            {
                                if let Some(handle) = restock_server.sessions.get(&session_id) {
                                    handle.queue_message(msg.clone()).await;
                                }
                            }
                        }
                    }
                }

                // Save the new restock date to database
                if let Err(e) = db::set_last_restock_date(&restock_server.db, &current_date).await {
                    error!("Failed to save restock date: {}", e);
                }
                last_restock_date = Some(current_date.clone());
                info!("Daily shop restock complete");
            }
        }
    });
}

/// Spawn a task to handle admin actions from the API
fn spawn_admin_action_handler(server: Arc<Server>, mut action_rx: mpsc::Receiver<AdminAction>) {
    tokio::spawn(async move {
        while let Some(action) = action_rx.recv().await {
            match action {
                AdminAction::KickPlayer { username, reason } => {
                    handle_admin_kick(&server, &username, reason.as_deref()).await;
                }
                AdminAction::TeleportPlayer {
                    username,
                    room_id,
                    x,
                    y,
                } => {
                    handle_admin_teleport(&server, &username, room_id, x, y).await;
                }
                AdminAction::SetPoints {
                    username,
                    points,
                    mode,
                } => {
                    handle_admin_set_points(&server, &username, points, mode).await;
                }
                AdminAction::GiveItem {
                    username,
                    category,
                    slot,
                    item_id,
                } => {
                    handle_admin_give_item(&server, &username, category, slot, item_id).await;
                }
                AdminAction::SetBank {
                    username,
                    balance,
                    mode,
                } => {
                    handle_admin_set_bank(&server, &username, balance, mode).await;
                }
                AdminAction::SendMail {
                    to_username,
                    sender_name,
                    message,
                    points,
                    item_id,
                    item_category,
                } => {
                    handle_admin_send_mail(
                        &server,
                        &to_username,
                        &sender_name,
                        &message,
                        points,
                        item_id,
                        item_category,
                    )
                    .await;
                }
                AdminAction::SetAppearance {
                    username,
                    body_id,
                    acs1_id,
                    acs2_id,
                } => {
                    handle_admin_set_appearance(&server, &username, body_id, acs1_id, acs2_id)
                        .await;
                }
            }
        }
    });
}

/// Handle admin kick action
async fn handle_admin_kick(server: &Server, username: &str, reason: Option<&str>) {
    let username_lower = username.to_lowercase();

    for session_ref in server.sessions.iter() {
        let handle = session_ref.value();
        let mut session = handle.session.write().await;
        if session.username.as_deref() == Some(&username_lower) && session.is_authenticated {
            let kick_reason = reason.unwrap_or("Kicked by administrator");
            session.kick(kick_reason);
            info!("Admin kicked player {}: {}", username, kick_reason);
            return;
        }
    }
}

/// Handle admin teleport action
async fn handle_admin_teleport(server: &Server, username: &str, room_id: u16, x: u16, y: u16) {
    let username_lower = username.to_lowercase();

    for session_ref in server.sessions.iter() {
        let session_id = *session_ref.key();
        let handle = session_ref.value();
        let mut session = handle.session.write().await;

        if session.username.as_deref() == Some(&username_lower) && session.is_authenticated {
            let old_room = session.room_id;
            let player_id = match session.player_id {
                Some(pid) => pid,
                None => return,
            };

            // Update session state
            session.room_id = room_id;
            session.x = x;
            session.y = y;

            // Update game state - remove from old room, add to new
            if old_room != room_id {
                server
                    .game_state
                    .remove_player_from_room(player_id, old_room)
                    .await;
                server
                    .game_state
                    .add_player_to_room(player_id, room_id, session_id)
                    .await;
            }

            // Send warp message to client using MSG_WARP_CENTER_USE_SLOT (77)
            // This is the same message used by warp centers to teleport players
            // Format: room_id (u16), x (u16), y (u16)
            let mut writer = protocol::MessageWriter::new();
            writer.write_u16(protocol::MessageType::WarpCenterUseSlot.id());
            writer.write_u16(room_id);
            writer.write_u16(x);
            writer.write_u16(y);
            session.queue_message(writer.into_bytes());

            // Update DB
            if let Some(char_id) = session.character_id {
                let _ =
                    db::update_position(&server.db, char_id, x as i16, y as i16, room_id as i16)
                        .await;
            }

            info!(
                "Admin teleported {} to room {} ({}, {})",
                username, room_id, x, y
            );
            return;
        }
    }
}

/// Handle admin set points action
async fn handle_admin_set_points(server: &Server, username: &str, points: i64, mode: PointsMode) {
    let username_lower = username.to_lowercase();

    // First get character info from DB
    let (char_id, current_points) = match db::find_account_by_username(&server.db, &username_lower)
        .await
    {
        Ok(Some(account)) => match db::find_character_by_account(&server.db, account.id).await {
            Ok(Some(char)) => (char.id, char.points),
            _ => return,
        },
        _ => return,
    };

    // Calculate new points
    let new_points = mode.apply(current_points, points).clamp(0, 999_999_999);

    // Update DB
    if db::update_points(&server.db, char_id, new_points)
        .await
        .is_err()
    {
        return;
    }

    // Find online session and update + notify
    for session_ref in server.sessions.iter() {
        let handle = session_ref.value();
        let mut session = handle.session.write().await;
        if session.username.as_deref() == Some(&username_lower) && session.is_authenticated {
            let old_points = session.points;
            session.points = new_points as u32;
            drop(session);

            // Send points update to client
            handle.queue_message(protocol::build_points_update(new_points as u32, false)).await;

            info!(
                "Admin set {} points: {} -> {}",
                username, old_points, new_points
            );
            return;
        }
    }

    info!("Admin set {} points to {} (offline)", username, new_points);
}

/// Handle admin give item action
async fn handle_admin_give_item(
    server: &Server,
    username: &str,
    category: InventoryCategory,
    slot: u8,
    item_id: u16,
) {
    let username_lower = username.to_lowercase();

    // Get character ID
    let char_id = match db::find_account_by_username(&server.db, &username_lower).await {
        Ok(Some(account)) => match db::find_character_by_account(&server.db, account.id).await {
            Ok(Some(char)) => char.id,
            _ => return,
        },
        _ => return,
    };

    // Update DB based on category
    let result = match category {
        InventoryCategory::Item => {
            db::update_item_slot(&server.db, char_id, slot, item_id as i16).await
        }
        InventoryCategory::Outfit => {
            db::update_outfit_slot(&server.db, char_id, slot, item_id as i16).await
        }
        InventoryCategory::Accessory => {
            db::update_accessory_slot(&server.db, char_id, slot, item_id as i16).await
        }
        InventoryCategory::Tool => {
            db::update_tool_slot(&server.db, char_id, slot, item_id as i16).await
        }
        InventoryCategory::Emote => {
            let column = match slot {
                1 => "emote_1",
                2 => "emote_2",
                3 => "emote_3",
                4 => "emote_4",
                5 => "emote_5",
                _ => return,
            };
            sqlx::query(&format!(
                "UPDATE inventories SET {} = ? WHERE character_id = ?",
                column
            ))
            .bind(item_id as i16)
            .bind(char_id)
            .execute(&server.db)
            .await
            .map(|_| ())
        }
    };

    if result.is_err() {
        error!("Failed to give item to {}", username);
        return;
    }

    // Note: Client would need to relog to see inventory changes
    // There's no simple "refresh inventory" message in the protocol
    info!(
        "Admin gave {:?} slot {} = {} to {}",
        category, slot, item_id, username
    );
}

/// Handle admin set bank action
async fn handle_admin_set_bank(server: &Server, username: &str, balance: i64, mode: PointsMode) {
    let username_lower = username.to_lowercase();

    // Get character info
    let (char_id, current_balance) = match db::find_account_by_username(&server.db, &username_lower)
        .await
    {
        Ok(Some(account)) => match db::find_character_by_account(&server.db, account.id).await {
            Ok(Some(char)) => (char.id, char.bank_balance),
            _ => return,
        },
        _ => return,
    };

    let new_balance = mode.apply(current_balance, balance).clamp(0, 999_999_999);

    if db::update_bank_balance(&server.db, char_id, new_balance)
        .await
        .is_ok()
    {
        info!(
            "Admin set {} bank balance: {} -> {}",
            username, current_balance, new_balance
        );
    }
}

/// Handle admin send mail action
async fn handle_admin_send_mail(
    server: &Server,
    to_username: &str,
    sender_name: &str,
    message: &str,
    points: i64,
    item_id: u16,
    item_category: u8,
) {
    // Get recipient character ID
    let to_char_id = match db::find_account_by_username(&server.db, to_username).await {
        Ok(Some(account)) => match db::find_character_by_account(&server.db, account.id).await {
            Ok(Some(char)) => char.id,
            _ => {
                error!(
                    "Cannot send mail: recipient {} has no character",
                    to_username
                );
                return;
            }
        },
        _ => {
            error!("Cannot send mail: recipient {} not found", to_username);
            return;
        }
    };

    // Create mail (from_character_id = None for system mail)
    match db::send_mail(
        &server.db,
        db::SendMailParams {
            from_character_id: None, // system sender
            to_character_id: to_char_id,
            sender_name,
            message,
            item_id: item_id as i64,
            item_cat: item_category as i64,
            points,
            paper: 1, // default paper
            font_color: 0, // default font color
        },
    )
    .await
    {
        Ok(mail_id) => {
            info!(
                "Admin sent mail {} to {} from '{}'",
                mail_id, to_username, sender_name
            );

            // Notify player if online (they have new mail)
            // The client checks mail count, there's no push notification in the protocol
        }
        Err(e) => {
            error!("Failed to send admin mail: {}", e);
        }
    }
}

/// Handle admin set appearance action
async fn handle_admin_set_appearance(
    server: &Server,
    username: &str,
    body_id: Option<u16>,
    acs1_id: Option<u16>,
    acs2_id: Option<u16>,
) {
    let username_lower = username.to_lowercase();

    // Get character ID
    let char_id = match db::find_account_by_username(&server.db, &username_lower).await {
        Ok(Some(account)) => match db::find_character_by_account(&server.db, account.id).await {
            Ok(Some(char)) => char.id,
            _ => return,
        },
        _ => return,
    };

    // Update each provided field
    if let Some(body) = body_id {
        let _ = db::update_body_id(&server.db, char_id, body as i16).await;
    }
    if let Some(acs1) = acs1_id {
        let _ = db::update_accessory1_id(&server.db, char_id, acs1 as i16).await;
    }
    if let Some(acs2) = acs2_id {
        let _ = db::update_accessory2_id(&server.db, char_id, acs2 as i16).await;
    }

    // Update online session and broadcast appearance change
    for session_ref in server.sessions.iter() {
        let handle = session_ref.value();
        let mut session = handle.session.write().await;
        if session.username.as_deref() == Some(&username_lower) && session.is_authenticated {
            if let Some(body) = body_id {
                session.body_id = body;
            }
            if let Some(acs1) = acs1_id {
                session.acs1_id = acs1;
            }
            if let Some(acs2) = acs2_id {
                session.acs2_id = acs2;
            }

            // Broadcast appearance change to room
            if let Some(player_id) = session.player_id {
                let room_id = session.room_id;
                let body = session.body_id;
                let acs1 = session.acs1_id;
                let acs2 = session.acs2_id;

                drop(session); // Release lock before broadcasting

                let room_players = server.game_state.get_room_players(room_id).await;

                // Send MSG_CHANGE_OUTFIT to room if body was changed
                if body_id.is_some() {
                    let mut writer = protocol::MessageWriter::new();
                    writer.write_u16(protocol::MessageType::ChangeOutfit.id());
                    writer.write_u16(player_id);
                    writer.write_u16(body);
                    let msg = writer.into_bytes();

                    for pid in &room_players {
                        if let Some(sid) = server.game_state.players_by_id.get(pid) {
                            if let Some(h) = server.sessions.get(&sid) {
                                h.queue_message(msg.clone()).await;
                            }
                        }
                    }
                }

                // Send MSG_CHANGE_ACCESSORY1 to room if acs1 was changed
                if acs1_id.is_some() {
                    let mut writer = protocol::MessageWriter::new();
                    writer.write_u16(protocol::MessageType::ChangeAccessory1.id());
                    writer.write_u16(player_id);
                    writer.write_u16(acs1);
                    let msg = writer.into_bytes();

                    for pid in &room_players {
                        if let Some(sid) = server.game_state.players_by_id.get(pid) {
                            if let Some(h) = server.sessions.get(&sid) {
                                h.queue_message(msg.clone()).await;
                            }
                        }
                    }
                }

                // Send MSG_CHANGE_ACCESSORY2 to room if acs2 was changed
                if acs2_id.is_some() {
                    let mut writer = protocol::MessageWriter::new();
                    writer.write_u16(protocol::MessageType::ChangeAccessory2.id());
                    writer.write_u16(player_id);
                    writer.write_u16(acs2);
                    let msg = writer.into_bytes();

                    for pid in &room_players {
                        if let Some(sid) = server.game_state.players_by_id.get(pid) {
                            if let Some(h) = server.sessions.get(&sid) {
                                h.queue_message(msg.clone()).await;
                            }
                        }
                    }
                }
            }

            info!(
                "Admin set {} appearance: body={:?}, acs1={:?}, acs2={:?}",
                username, body_id, acs1_id, acs2_id
            );
            return;
        }
    }

    info!(
        "Admin set {} appearance (offline): body={:?}, acs1={:?}, acs2={:?}",
        username, body_id, acs1_id, acs2_id
    );
}
