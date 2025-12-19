# Architecture Overview

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Slime Online 2 Server                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────┐  │
│  │   TCP Listener   │  │   Message Queue  │  │   Database   │  │
│  │   (Port 5555)    │  │   (Async Chan)   │  │  (Postgres)  │  │
│  └────────┬─────────┘  └────────┬─────────┘  └──────┬───────┘  │
│           │                     │                    │           │
│           v                     v                    v           │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │            Connection Manager                            │   │
│  │  - Accepts TCP connections                              │   │
│  │  - RC4 encryption/decryption                            │   │
│  │  - Rate limiting & DDoS protection                      │   │
│  │  - Connection state tracking                            │   │
│  └─────────────────────────────────────────────────────────┘   │
│           │                                                      │
│           v                                                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │            Message Router                                │   │
│  │  - Parse message type (u16)                             │   │
│  │  - Route to appropriate handler                         │   │
│  │  - Error handling & validation                          │   │
│  └─────────────────────────────────────────────────────────┘   │
│           │                                                      │
│           ├──────┬───────┬───────┬───────┬───────┬──────────┐  │
│           v      v       v       v       v       v          v  │
│  ┌───────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌──────┐ │
│  │ Auth  │ │Move │ │Item │ │Clan │ │Shop │ │Quest│ │ BBS  │ │
│  │Handler│ │Hand.│ │Hand.│ │Hand.│ │Hand.│ │Hand.│ │Hand. │ │
│  └───┬───┘ └──┬──┘ └──┬──┘ └──┬──┘ └──┬──┘ └──┬──┘ └───┬──┘ │
│      └─────────┴────────┴───────┴───────┴───────┴────────┘    │
│                          │                                      │
│                          v                                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │            Game State Manager                            │   │
│  │  - World state (rooms, players, NPCs)                   │   │
│  │  - Player sessions                                       │   │
│  │  - Clan data                                             │   │
│  │  - Active quests                                         │   │
│  └─────────────────────────────────────────────────────────┘   │
│                          │                                      │
│                          v                                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │            Broadcast Manager                             │   │
│  │  - Room-based broadcasting                              │   │
│  │  - Global announcements                                  │   │
│  │  - Targeted messaging                                    │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Connection Manager
**Responsibility:** Handle all TCP connections

**Key Tasks:**
- Accept incoming TCP connections
- Perform RC4 encryption/decryption
- Track connection state (unauthenticated → authenticated)
- Enforce rate limits and connection limits
- Detect and handle disconnects
- Buffer incomplete messages

**Concurrency Model:**
- One tokio task per connection
- Shared state via Arc<DashMap>
- Message passing via mpsc channels

### 2. Message Router
**Responsibility:** Parse and route messages

**Key Tasks:**
- Read message type (first 2 bytes)
- Deserialize payload based on type
- Validate message structure
- Route to appropriate handler
- Return responses to sender

**Design Pattern:** Command pattern with message handlers

### 3. Game State Manager
**Responsibility:** Maintain server-side game state

**Key Data Structures:**
```rust
pub struct GameState {
    rooms: Arc<DashMap<u16, Room>>,
    players: Arc<DashMap<u16, Player>>,
    clans: Arc<DashMap<u16, Clan>>,
    sessions: Arc<DashMap<Uuid, Session>>,
    world_objects: Arc<DashMap<(u16, u16), WorldObject>>,
}

pub struct Room {
    id: u16,
    players: HashSet<u16>,
    collectibles: HashMap<u8, Collectible>,
    discarded_items: HashMap<u16, DiscardedItem>,
    plant_spots: HashMap<u8, PlantSpot>,
    build_spots: HashMap<u8, BuildSpot>,
}

pub struct Player {
    id: u16,
    account_id: u32,
    username: String,
    position: Position,
    room_id: u16,
    clan_id: Option<u16>,
    inventory: Inventory,
    quest_state: QuestState,
    // ... full state
}
```

### 4. Database Layer
**Responsibility:** Persist and retrieve data

**Operations:**
- Account authentication (bcrypt verification)
- Player data load/save
- Inventory transactions
- Clan CRUD operations
- Mail delivery
- BBS posts

**Design:** Repository pattern with async sqlx

## Threading & Concurrency

### Task Model

```rust
#[tokio::main]
async fn main() {
    // Global state
    let game_state = Arc::new(GameState::new());
    let db_pool = create_db_pool().await;
    
    // Spawn background tasks
    tokio::spawn(tick_loop(game_state.clone()));
    tokio::spawn(save_loop(game_state.clone(), db_pool.clone()));
    tokio::spawn(cleanup_loop(game_state.clone()));
    
    // Accept connections
    let listener = TcpListener::bind("0.0.0.0:5555").await.unwrap();
    
    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        let state = game_state.clone();
        let pool = db_pool.clone();
        
        tokio::spawn(handle_connection(socket, addr, state, pool));
    }
}
```

### Shared State
- **Arc<DashMap>:** Concurrent hashmap for shared mutable state
- **Arc<RwLock>:** For read-heavy data (item definitions, room data)
- **mpsc channels:** For message passing between tasks

### Synchronization Points
- **Room broadcasts:** Lock room player list, send to all
- **Inventory transactions:** Lock player inventory, update atomically
- **Clan operations:** Lock clan data, update members list

## Message Flow

### Example: Player Movement

```
1. Client sends MSG_MOVE_PLAYER (encrypted)
   ↓
2. Connection task receives bytes
   ↓
3. RC4 decrypt
   ↓
4. Parse message type (2) and payload (direction, x, y)
   ↓
5. Validate:
   - Is player authenticated?
   - Is position in room bounds?
   - Is movement physically possible?
   ↓
6. Update player position in GameState
   ↓
7. Broadcast to all players in same room (except sender):
   - Build MSG_MOVE_PLAYER with player_id
   - RC4 encrypt
   - Send via TCP
   ↓
8. Log movement event (DEBUG level)
```

## Error Handling Strategy

### Error Types
```rust
#[derive(Debug)]
pub enum ServerError {
    // Network errors
    ConnectionClosed,
    ReadTimeout,
    WriteTimeout,
    
    // Protocol errors
    InvalidMessageType(u16),
    MalformedMessage,
    UnexpectedMessageType(u16),
    
    // Game errors
    PlayerNotFound(u16),
    RoomNotFound(u16),
    InsufficientPermissions,
    InvalidGameState,
    
    // Database errors
    DatabaseError(sqlx::Error),
    TransactionFailed,
}
```

### Error Handling
- **Network errors:** Log, disconnect client
- **Protocol errors:** Log, increment strike counter, disconnect after 3
- **Game errors:** Log, send error message to client
- **Database errors:** Log, retry once, fail gracefully

## Performance Optimizations

### 1. Message Batching
```rust
// Instead of sending each message immediately
send_queue.push(message);

// Flush every 16ms (60 ticks/sec) or when queue > 10
if queue.len() > 10 || last_flush.elapsed() > 16ms {
    flush_send_queue();
}
```

### 2. Database Connection Pooling
- Pool size: 50 connections
- Reuse connections across transactions
- Prepared statement caching

### 3. Room-Based Sharding
- Divide world into rooms
- Only broadcast within room
- Reduces O(n²) to O(r²) where r = players per room

### 4. Lazy Loading
- Load inventory only when needed (shop, item use)
- Load clan data on demand
- Cache frequently accessed data

## Scalability Considerations

### Horizontal Scaling (Future)
- Multiple server instances
- Redis for shared state (player positions, room lists)
- Database read replicas
- Load balancer for connections

### Vertical Scaling (Current)
- Single server handles 500+ concurrent players
- Multi-threaded via tokio
- Efficient data structures (DashMap)

## Monitoring & Observability

### Metrics
```rust
pub struct ServerMetrics {
    active_connections: AtomicU64,
    messages_per_second: AtomicU64,
    average_ping: AtomicU64,
    database_query_time: Histogram,
    error_count: Counter,
}
```

### Logging
```rust
tracing::info!(
    player_id = %player.id,
    room_id = %room.id,
    "Player moved"
);

tracing::warn!(
    ip = %addr,
    attempts = %count,
    "Rate limit exceeded"
);

tracing::error!(
    error = %e,
    "Database transaction failed"
);
```

### Health Checks
```rust
// HTTP health endpoint (port 8080)
GET /health
{
    "status": "healthy",
    "active_players": 42,
    "uptime_seconds": 86400,
    "database_connected": true
}
```

## Configuration Management

### Config File (config.toml)
```toml
[server]
host = "0.0.0.0"
port = 5555
max_connections = 1000

[database]
url = "postgres://user:pass@localhost/slime_online2"
max_connections = 50

[game]
motd = "Welcome to Slime Online 2!"
max_players_per_room = 50
save_interval_seconds = 300

[security]
max_login_attempts = 3
rate_limit_messages_per_second = 60
connection_timeout_seconds = 300

[features]
enable_clans = true
enable_bbs = true
enable_planting = true
```

## Deployment Architecture

### Development
```
Local Machine
├── Rust server (cargo run)
├── PostgreSQL (Docker)
└── Client (Windows VM or Wine)
```

### Production
```
VPS/Cloud Server
├── Rust server (systemd service)
├── PostgreSQL (managed service or local)
├── Nginx (reverse proxy for health checks)
└── Monitoring (Prometheus + Grafana)
```

## Directory Structure

```
rust_server/
├── src/
│   ├── main.rs                 # Entry point, TCP listener
│   ├── connection.rs           # Connection handling
│   ├── crypto.rs               # RC4 encryption
│   ├── protocol/
│   │   ├── mod.rs
│   │   ├── reader.rs           # MessageReader
│   │   ├── writer.rs           # MessageWriter
│   │   └── messages.rs         # Message structs
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── auth.rs             # Login/register
│   │   ├── movement.rs         # Player movement
│   │   ├── chat.rs             # Chat messages
│   │   ├── items.rs            # Item operations
│   │   ├── shop.rs             # Shop system
│   │   ├── clan.rs             # Clan system
│   │   ├── quest.rs            # Quest system
│   │   ├── mail.rs             # Mail system
│   │   └── bbs.rs              # Bulletin board
│   ├── game/
│   │   ├── mod.rs
│   │   ├── state.rs            # GameState
│   │   ├── room.rs             # Room management
│   │   ├── player.rs           # Player state
│   │   └── validation.rs       # Game rules validation
│   ├── db/
│   │   ├── mod.rs
│   │   ├── accounts.rs         # Account queries
│   │   ├── characters.rs       # Character queries
│   │   ├── clans.rs            # Clan queries
│   │   └── mail.rs             # Mail queries
│   └── util/
│       ├── mod.rs
│       ├── metrics.rs          # Prometheus metrics
│       └── config.rs           # Config loading
├── migrations/                 # SQL migration files
├── config.toml                 # Server configuration
├── Cargo.toml                  # Dependencies
└── docs/                       # This documentation
```

---

**Next:** Read `02-connection-manager.md` for connection handling details.
