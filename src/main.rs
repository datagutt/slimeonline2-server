//! Slime Online 2 Private Server
//!
//! A Rust implementation of the Slime Online 2 server for the v0.106 client.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use dashmap::DashMap;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use tracing_subscriber::{EnvFilter, fmt};
use uuid::Uuid;

mod constants;
mod crypto;
mod db;
mod protocol;
mod handlers;
mod game;

use constants::*;
use db::DbPool;
use game::{GameState, PlayerSession};

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub motd: String,
    pub max_connections: usize,
    pub max_connections_per_ip: usize,
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
        }
    }
}

/// Shared server state
pub struct Server {
    pub config: ServerConfig,
    pub db: DbPool,
    pub game_state: Arc<GameState>,
    pub sessions: Arc<DashMap<Uuid, Arc<RwLock<PlayerSession>>>>,
    pub connections_by_ip: Arc<DashMap<String, usize>>,
    pub active_player_ids: Arc<DashMap<u16, Uuid>>,
    next_player_id: Arc<std::sync::atomic::AtomicU16>,
}

impl Server {
    /// Create a new server instance.
    pub async fn new(config: ServerConfig) -> Result<Self> {
        // Create database connection pool
        let db = db::create_pool(&config.database_url).await?;
        
        // Run migrations
        db::init_database(&db).await?;
        
        Ok(Self {
            config,
            db,
            game_state: Arc::new(GameState::new()),
            sessions: Arc::new(DashMap::new()),
            connections_by_ip: Arc::new(DashMap::new()),
            active_player_ids: Arc::new(DashMap::new()),
            next_player_id: Arc::new(std::sync::atomic::AtomicU16::new(1)),
        })
    }

    /// Get the next available player ID.
    pub fn next_player_id(&self) -> u16 {
        self.next_player_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Check if a player ID is in use.
    pub fn is_player_online(&self, username: &str) -> bool {
        // Check all sessions for matching username
        for session_ref in self.sessions.iter() {
            if let Ok(session) = session_ref.value().try_read() {
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
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Slime Online 2 Server v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = ServerConfig::default();
    
    // Create server
    let server = Arc::new(Server::new(config.clone()).await?);
    
    info!("Database initialized");

    // Bind to address
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;
    
    info!("Server listening on {}", addr);
    info!("MOTD: {}", config.motd);

    // Spawn background tasks
    spawn_background_tasks(server.clone());

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
        loop {
            interval.tick().await;
            info!("Running periodic save...");
            
            // Save all active player data
            for session_ref in save_server.sessions.iter() {
                let session = session_ref.value().read().await;
                if let (Some(char_id), true) = (session.character_id, session.is_authenticated) {
                    if let Err(e) = db::update_position(
                        &save_server.db,
                        char_id,
                        session.x as i16,
                        session.y as i16,
                        session.room_id as i16,
                    ).await {
                        error!("Failed to save position for character {}: {}", char_id, e);
                    }
                    
                    if let Err(e) = db::update_points(&save_server.db, char_id, session.points as i64).await {
                        error!("Failed to save points for character {}: {}", char_id, e);
                    }
                }
            }
            
            info!("Periodic save complete. Active sessions: {}", save_server.sessions.len());
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
                let session = session_ref.value().read().await;
                if session.is_timed_out() {
                    stale_sessions.push(*session_ref.key());
                }
            }
            
            for session_id in stale_sessions {
                if let Some((_, session)) = cleanup_server.sessions.remove(&session_id) {
                    let session_guard = session.read().await;
                    if let Some(username) = &session_guard.username {
                        info!("Cleaning up stale session for {}", username);
                    }
                    
                    // Remove from room
                    if let Some(player_id) = session_guard.player_id {
                        cleanup_server.game_state
                            .remove_player_from_room(player_id, session_guard.room_id).await;
                        cleanup_server.active_player_ids.remove(&player_id);
                    }
                    
                    // Remove IP connection count
                    cleanup_server.remove_ip_connection(&session_guard.ip_address);
                }
            }
        }
    });
}
