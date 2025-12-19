# SQLite Adaptation for Local Development

## Overview

SQLite is perfectly suitable for local development and small-scale deployments of the Slime Online 2 server. This document explains how to adapt the PostgreSQL schema for SQLite and when to use each database.

## When to Use SQLite vs PostgreSQL

### Use SQLite for:
✅ **Local development** - Fast setup, no separate DB server  
✅ **Testing** - Easy to reset, in-memory databases  
✅ **Small servers** - < 50 concurrent players  
✅ **Single instance** - No horizontal scaling needed  
✅ **Prototyping** - Quick iteration  

### Use PostgreSQL for:
✅ **Production servers** - > 50 concurrent players  
✅ **High concurrency** - Better write performance under load  
✅ **Horizontal scaling** - Multiple server instances  
✅ **Advanced features** - Full-text search, JSON queries, etc.  
✅ **Data integrity** - More robust constraint checking  

## SQLite Schema Adaptations

### Key Differences

| Feature | PostgreSQL | SQLite |
|---------|------------|--------|
| Auto-increment | `SERIAL` | `INTEGER PRIMARY KEY AUTOINCREMENT` |
| Boolean | `BOOLEAN` | `INTEGER` (0/1) |
| UUID | `UUID` | `TEXT` |
| Timestamp | `TIMESTAMP` | `TEXT` (ISO8601) or `INTEGER` (Unix) |
| IP Address | `INET` | `TEXT` |
| Constraints | Full support | Limited CHECK support |
| Concurrent writes | Excellent | Limited (one writer at a time) |

### Converted Schema

```sql
-- accounts table (SQLite version)
CREATE TABLE accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT UNIQUE NOT NULL COLLATE NOCASE,
    password_hash TEXT NOT NULL,
    email TEXT,
    mac_address TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_login TEXT,
    is_banned INTEGER NOT NULL DEFAULT 0,
    ban_reason TEXT,
    CHECK (length(username) >= 3 AND length(username) <= 20)
);

CREATE INDEX idx_accounts_username ON accounts(username COLLATE NOCASE);
CREATE INDEX idx_accounts_mac ON accounts(mac_address);

-- characters table (SQLite version)
CREATE TABLE characters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL,
    username TEXT NOT NULL,
    
    -- Position
    x INTEGER NOT NULL DEFAULT 160,
    y INTEGER NOT NULL DEFAULT 120,
    room_id INTEGER NOT NULL DEFAULT 1,
    
    -- Appearance
    body_id INTEGER NOT NULL DEFAULT 1,
    acs1_id INTEGER NOT NULL DEFAULT 0,
    acs2_id INTEGER NOT NULL DEFAULT 0,
    
    -- Currency & Stats
    points INTEGER NOT NULL DEFAULT 0,
    bank_balance INTEGER NOT NULL DEFAULT 0,
    trees_planted INTEGER NOT NULL DEFAULT 0,
    objects_built INTEGER NOT NULL DEFAULT 0,
    
    -- Quest State
    quest_id INTEGER NOT NULL DEFAULT 0,
    quest_step INTEGER NOT NULL DEFAULT 0,
    quest_var INTEGER NOT NULL DEFAULT 0,
    
    -- Permissions
    has_signature INTEGER NOT NULL DEFAULT 0,
    is_moderator INTEGER NOT NULL DEFAULT 0,
    
    -- Clan
    clan_id INTEGER,
    
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    FOREIGN KEY (clan_id) REFERENCES clans(id) ON DELETE SET NULL,
    CHECK (points >= 0),
    CHECK (bank_balance >= 0),
    UNIQUE (account_id)
);

CREATE INDEX idx_characters_account ON characters(account_id);
CREATE INDEX idx_characters_room ON characters(room_id);
CREATE INDEX idx_characters_clan ON characters(clan_id);

-- inventories table (SQLite version)
CREATE TABLE inventories (
    character_id INTEGER PRIMARY KEY,
    
    -- Emote Slots (5)
    emote_1 INTEGER NOT NULL DEFAULT 0,
    emote_2 INTEGER NOT NULL DEFAULT 0,
    emote_3 INTEGER NOT NULL DEFAULT 0,
    emote_4 INTEGER NOT NULL DEFAULT 0,
    emote_5 INTEGER NOT NULL DEFAULT 0,
    
    -- Outfit Slots (9)
    outfit_1 INTEGER NOT NULL DEFAULT 0,
    outfit_2 INTEGER NOT NULL DEFAULT 0,
    outfit_3 INTEGER NOT NULL DEFAULT 0,
    outfit_4 INTEGER NOT NULL DEFAULT 0,
    outfit_5 INTEGER NOT NULL DEFAULT 0,
    outfit_6 INTEGER NOT NULL DEFAULT 0,
    outfit_7 INTEGER NOT NULL DEFAULT 0,
    outfit_8 INTEGER NOT NULL DEFAULT 0,
    outfit_9 INTEGER NOT NULL DEFAULT 0,
    
    -- Accessory Slots (9)
    accessory_1 INTEGER NOT NULL DEFAULT 0,
    accessory_2 INTEGER NOT NULL DEFAULT 0,
    accessory_3 INTEGER NOT NULL DEFAULT 0,
    accessory_4 INTEGER NOT NULL DEFAULT 0,
    accessory_5 INTEGER NOT NULL DEFAULT 0,
    accessory_6 INTEGER NOT NULL DEFAULT 0,
    accessory_7 INTEGER NOT NULL DEFAULT 0,
    accessory_8 INTEGER NOT NULL DEFAULT 0,
    accessory_9 INTEGER NOT NULL DEFAULT 0,
    
    -- Item Slots (9)
    item_1 INTEGER NOT NULL DEFAULT 0,
    item_2 INTEGER NOT NULL DEFAULT 0,
    item_3 INTEGER NOT NULL DEFAULT 0,
    item_4 INTEGER NOT NULL DEFAULT 0,
    item_5 INTEGER NOT NULL DEFAULT 0,
    item_6 INTEGER NOT NULL DEFAULT 0,
    item_7 INTEGER NOT NULL DEFAULT 0,
    item_8 INTEGER NOT NULL DEFAULT 0,
    item_9 INTEGER NOT NULL DEFAULT 0,
    
    -- Tool Slots (9)
    tool_1 INTEGER NOT NULL DEFAULT 0,
    tool_2 INTEGER NOT NULL DEFAULT 0,
    tool_3 INTEGER NOT NULL DEFAULT 0,
    tool_4 INTEGER NOT NULL DEFAULT 0,
    tool_5 INTEGER NOT NULL DEFAULT 0,
    tool_6 INTEGER NOT NULL DEFAULT 0,
    tool_7 INTEGER NOT NULL DEFAULT 0,
    tool_8 INTEGER NOT NULL DEFAULT 0,
    tool_9 INTEGER NOT NULL DEFAULT 0,
    
    -- Equipped Tool
    equipped_tool INTEGER NOT NULL DEFAULT 0,
    
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE
);

-- clans table (SQLite version)
CREATE TABLE clans (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL COLLATE NOCASE,
    leader_id INTEGER NOT NULL,
    
    -- Display
    color_inner INTEGER NOT NULL,
    color_outer INTEGER NOT NULL,
    
    -- Stats
    level INTEGER NOT NULL DEFAULT 1,
    points INTEGER NOT NULL DEFAULT 0,
    max_members INTEGER NOT NULL DEFAULT 5,
    
    -- Info
    description TEXT,
    news TEXT,
    show_name INTEGER NOT NULL DEFAULT 1,
    has_base INTEGER NOT NULL DEFAULT 0,
    
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (leader_id) REFERENCES characters(id)
);

CREATE INDEX idx_clans_leader ON clans(leader_id);

-- mail table (SQLite version)
CREATE TABLE mail (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sender_id INTEGER,
    sender_name TEXT NOT NULL,
    receiver_id INTEGER NOT NULL,
    
    subject TEXT NOT NULL,
    body TEXT NOT NULL,
    
    -- Attachments
    points_attached INTEGER DEFAULT 0,
    item_category INTEGER DEFAULT 0,
    item_id INTEGER DEFAULT 0,
    
    -- Metadata
    paper_style INTEGER DEFAULT 1,
    font_color INTEGER DEFAULT 1,
    is_read INTEGER DEFAULT 0,
    
    sent_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (sender_id) REFERENCES characters(id) ON DELETE SET NULL,
    FOREIGN KEY (receiver_id) REFERENCES characters(id) ON DELETE CASCADE,
    CHECK (points_attached >= 0)
);

CREATE INDEX idx_mail_receiver ON mail(receiver_id, is_read);
CREATE INDEX idx_mail_sent_at ON mail(sent_at DESC);

-- bbs_posts table (SQLite version)
CREATE TABLE bbs_posts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    category_id INTEGER NOT NULL,
    author_id INTEGER,
    author_name TEXT NOT NULL,
    
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    
    is_reported INTEGER DEFAULT 0,
    report_count INTEGER DEFAULT 0,
    
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    
    FOREIGN KEY (author_id) REFERENCES characters(id) ON DELETE SET NULL
);

CREATE INDEX idx_bbs_category ON bbs_posts(category_id, created_at DESC);
CREATE INDEX idx_bbs_reported ON bbs_posts(is_reported) WHERE is_reported = 1;

-- sessions table (SQLite version)
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,  -- UUID as TEXT
    character_id INTEGER NOT NULL,
    ip_address TEXT NOT NULL,
    
    -- Connection info
    connected_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_activity TEXT NOT NULL DEFAULT (datetime('now')),
    
    -- State
    current_room INTEGER,
    is_active INTEGER DEFAULT 1,
    
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE
);

CREATE INDEX idx_sessions_character ON sessions(character_id);
CREATE INDEX idx_sessions_ip ON sessions(ip_address);
CREATE INDEX idx_sessions_activity ON sessions(last_activity) WHERE is_active = 1;

-- bans table (SQLite version)
CREATE TABLE bans (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ban_type TEXT NOT NULL,  -- 'ip', 'account', 'mac'
    value TEXT NOT NULL,
    reason TEXT NOT NULL,
    banned_by TEXT,
    banned_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT,  -- NULL = permanent
    
    UNIQUE (ban_type, value)
);

CREATE INDEX idx_bans_type_value ON bans(ban_type, value) 
    WHERE expires_at IS NULL OR expires_at > datetime('now');
```

## Rust sqlx Configuration

### Cargo.toml

```toml
[dependencies]
# Use either PostgreSQL or SQLite (or both with features)
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "migrate"] }

# OR for both databases:
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "sqlite", "migrate"] }
```

### Database URL Format

**SQLite:**
```bash
# File-based database
DATABASE_URL=sqlite:slime_online2.db

# In-memory (testing)
DATABASE_URL=sqlite::memory:
```

**PostgreSQL:**
```bash
DATABASE_URL=postgres://user:password@localhost/slime_online2
```

### Connection Pool Setup

```rust
use sqlx::SqlitePool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // SQLite pool
    let pool = SqlitePool::connect("sqlite:slime_online2.db").await?;
    
    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;
    
    // Configure SQLite for better concurrency
    sqlx::query("PRAGMA journal_mode = WAL;")
        .execute(&pool)
        .await?;
    
    sqlx::query("PRAGMA synchronous = NORMAL;")
        .execute(&pool)
        .await?;
    
    sqlx::query("PRAGMA cache_size = -64000;")  // 64MB cache
        .execute(&pool)
        .await?;
    
    sqlx::query("PRAGMA foreign_keys = ON;")
        .execute(&pool)
        .await?;
    
    Ok(())
}
```

## SQLite Performance Optimization

### WAL Mode (Write-Ahead Logging)

```sql
PRAGMA journal_mode = WAL;
```

**Benefits:**
- Multiple readers can access DB while writer is active
- Better concurrency (critical for game server)
- Faster writes

### Other Performance PRAGMAs

```sql
-- Use faster synchronization (still crash-safe)
PRAGMA synchronous = NORMAL;

-- Increase cache size (default is small)
PRAGMA cache_size = -64000;  -- 64 MB

-- Enable memory-mapped I/O
PRAGMA mmap_size = 268435456;  -- 256 MB

-- Enable foreign keys
PRAGMA foreign_keys = ON;

-- Temporary tables in memory
PRAGMA temp_store = MEMORY;
```

### Initialize at Startup

```rust
pub async fn init_sqlite_performance(pool: &SqlitePool) -> Result<()> {
    sqlx::query("PRAGMA journal_mode = WAL;").execute(pool).await?;
    sqlx::query("PRAGMA synchronous = NORMAL;").execute(pool).await?;
    sqlx::query("PRAGMA cache_size = -64000;").execute(pool).await?;
    sqlx::query("PRAGMA foreign_keys = ON;").execute(pool).await?;
    sqlx::query("PRAGMA temp_store = MEMORY;").execute(pool).await?;
    Ok(())
}
```

## Migration Strategy

### Separate Migration Directories

```
migrations/
├── postgres/
│   ├── 20240101000001_create_accounts.sql
│   ├── 20240101000002_create_characters.sql
│   └── ...
└── sqlite/
    ├── 20240101000001_create_accounts.sql
    ├── 20240101000002_create_characters.sql
    └── ...
```

### Runtime Database Selection

```rust
pub enum DatabaseType {
    Postgres,
    Sqlite,
}

pub async fn create_pool(db_type: DatabaseType, url: &str) -> Result<Box<dyn Database>> {
    match db_type {
        DatabaseType::Postgres => {
            let pool = PgPool::connect(url).await?;
            Ok(Box::new(PostgresDatabase { pool }))
        }
        DatabaseType::Sqlite => {
            let pool = SqlitePool::connect(url).await?;
            init_sqlite_performance(&pool).await?;
            Ok(Box::new(SqliteDatabase { pool }))
        }
    }
}
```

## Type Conversions

### Boolean Fields

**PostgreSQL:**
```rust
let is_banned: bool = row.get("is_banned");
```

**SQLite:**
```rust
let is_banned: i32 = row.get("is_banned");
let is_banned: bool = is_banned != 0;

// Or when inserting:
sqlx::query("UPDATE accounts SET is_banned = ? WHERE id = ?")
    .bind(is_banned as i32)
    .bind(account_id)
    .execute(&pool)
    .await?;
```

### Timestamps

**PostgreSQL:**
```rust
use chrono::{DateTime, Utc};
let created_at: DateTime<Utc> = row.get("created_at");
```

**SQLite (as TEXT):**
```rust
let created_at: String = row.get("created_at");
let created_at = chrono::NaiveDateTime::parse_from_str(&created_at, "%Y-%m-%d %H:%M:%S")?;
```

**SQLite (as INTEGER Unix timestamp):**
```rust
let created_at: i64 = row.get("created_at");
let created_at = chrono::NaiveDateTime::from_timestamp(created_at, 0);
```

### UUIDs

**PostgreSQL:**
```rust
use uuid::Uuid;
let session_id: Uuid = row.get("id");
```

**SQLite:**
```rust
let session_id: String = row.get("id");
let session_id = Uuid::parse_str(&session_id)?;

// When inserting:
let session_id = Uuid::new_v4();
sqlx::query("INSERT INTO sessions (id, ...) VALUES (?, ...)")
    .bind(session_id.to_string())
    .execute(&pool)
    .await?;
```

## Database Abstraction Layer

Create a trait to abstract database differences:

```rust
#[async_trait]
pub trait Database: Send + Sync {
    async fn create_account(&self, username: &str, password_hash: &str, mac: &str) -> Result<u32>;
    async fn get_account(&self, username: &str) -> Result<Option<Account>>;
    async fn create_character(&self, account_id: u32, username: &str) -> Result<u16>;
    async fn load_character(&self, account_id: u32) -> Result<Character>;
    async fn save_character(&self, character: &Character) -> Result<()>;
    // ... etc
}

pub struct SqliteDatabase {
    pool: SqlitePool,
}

pub struct PostgresDatabase {
    pool: PgPool,
}

#[async_trait]
impl Database for SqliteDatabase {
    async fn create_account(&self, username: &str, password_hash: &str, mac: &str) -> Result<u32> {
        let result = sqlx::query!(
            "INSERT INTO accounts (username, password_hash, mac_address) VALUES (?, ?, ?)",
            username,
            password_hash,
            mac
        )
        .execute(&self.pool)
        .await?;
        
        Ok(result.last_insert_rowid() as u32)
    }
    
    // ... implement other methods
}

#[async_trait]
impl Database for PostgresDatabase {
    async fn create_account(&self, username: &str, password_hash: &str, mac: &str) -> Result<u32> {
        let row = sqlx::query!(
            "INSERT INTO accounts (username, password_hash, mac_address) 
             VALUES ($1, $2, $3) RETURNING id",
            username,
            password_hash,
            mac
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(row.id as u32)
    }
    
    // ... implement other methods
}
```

## Concurrency Limitations

### SQLite Write Limitations

**Single Writer:** Only one transaction can write at a time.

**Mitigation:**
1. **WAL mode** - Readers don't block writer
2. **Short transactions** - Minimize lock time
3. **Batch writes** - Combine multiple updates
4. **Write queue** - Serialize write operations

```rust
use tokio::sync::Mutex;

pub struct SqliteWriteQueue {
    pool: SqlitePool,
    lock: Mutex<()>,
}

impl SqliteWriteQueue {
    pub async fn execute_write<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&SqlitePool) -> BoxFuture<'_, Result<T>>,
    {
        let _guard = self.lock.lock().await;
        f(&self.pool).await
    }
}
```

### When SQLite Becomes Limited

**Symptoms:**
- "Database is locked" errors
- Slow save operations
- Write timeouts

**Solutions:**
1. Optimize transaction size
2. Use connection pooling carefully (max 1 writer)
3. Batch operations
4. Migrate to PostgreSQL

**Connection Pool for SQLite:**
```rust
let pool = SqlitePool::connect_with(
    SqliteConnectOptions::from_str("sqlite:slime_online2.db")?
        .journal_mode(SqliteJournalMode::Wal)
        .create_if_missing(true)
)
.max_connections(5)  // Multiple readers, but only 1 writer at a time
.await?;
```

## Testing with In-Memory SQLite

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        
        // Run migrations
        sqlx::migrate!("./migrations/sqlite")
            .run(&pool)
            .await
            .unwrap();
        
        init_sqlite_performance(&pool).await.unwrap();
        
        pool
    }
    
    #[tokio::test]
    async fn test_create_account() {
        let pool = setup_test_db().await;
        let db = SqliteDatabase { pool };
        
        let account_id = db.create_account("testuser", "hash", "00:11:22:33:44:55")
            .await
            .unwrap();
        
        assert_eq!(account_id, 1);
    }
}
```

## Backup Strategy

### SQLite Backup

```bash
# Simple copy (while server is stopped)
cp slime_online2.db slime_online2.db.backup

# Online backup (while server running)
sqlite3 slime_online2.db ".backup slime_online2.db.backup"

# WAL checkpoint before backup
sqlite3 slime_online2.db "PRAGMA wal_checkpoint(FULL);"
```

### Automated Backup Script

```rust
use std::fs;
use chrono::Local;

pub async fn backup_sqlite_db(db_path: &str, backup_dir: &str) -> Result<()> {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let backup_path = format!("{}/slime_online2_{}.db", backup_dir, timestamp);
    
    fs::copy(db_path, &backup_path)?;
    
    println!("Database backed up to: {}", backup_path);
    Ok(())
}

// Run daily
tokio::spawn(async {
    let mut interval = tokio::time::interval(Duration::from_secs(86400));
    loop {
        interval.tick().await;
        if let Err(e) = backup_sqlite_db("slime_online2.db", "./backups").await {
            error!("Backup failed: {}", e);
        }
    }
});
```

## Migration from SQLite to PostgreSQL

When your server grows beyond SQLite's capabilities:

```bash
# Export SQLite data
sqlite3 slime_online2.db .dump > dump.sql

# Convert to PostgreSQL (manual editing needed)
# Change AUTOINCREMENT to SERIAL
# Change INTEGER for booleans to BOOLEAN
# Change TEXT timestamps to TIMESTAMP
# Adjust syntax differences

# Import to PostgreSQL
psql slime_online2 < dump.sql
```

## Recommendation

**For Development:** Start with SQLite
- Fast setup
- Easy testing
- Good for < 50 players

**For Production (>50 players):** Use PostgreSQL
- Better concurrency
- More reliable under load
- Industry standard

**Hybrid Approach:**
- Develop with SQLite
- Test with PostgreSQL before deployment
- Use database abstraction layer for easy switching

---

**Summary:** SQLite is excellent for local development and small deployments. Use the provided schema adaptations and performance tuning. Migrate to PostgreSQL when you need to scale beyond 50 concurrent players.
