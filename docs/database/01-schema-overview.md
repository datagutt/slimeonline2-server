# Database Schema Overview

## Database Selection

**Recommended:** PostgreSQL 14+

**Reasons:**
- ACID compliance for currency/inventory transactions
- JSON/JSONB support for flexible data (quest variables, etc.)
- Excellent performance for concurrent players
- Strong data integrity constraints
- Well-supported by Rust (sqlx)

## Schema Organization

```
slime_online2/
├── accounts          - User authentication
├── characters        - Player data & state
├── items             - Item definitions (read-only)
├── inventories       - Player item ownership
├── clans             - Clan system
├── mail              - Mail messages
├── bbs               - Bulletin board
├── world_state       - Dynamic world objects
├── collectibles      - Collectible spawns
├── bans              - IP/account bans
└── sessions          - Active player sessions
```

## Core Tables

### accounts
```sql
CREATE TABLE accounts (
    id SERIAL PRIMARY KEY,
    username VARCHAR(20) UNIQUE NOT NULL,
    password_hash VARCHAR(60) NOT NULL,  -- bcrypt hash
    email VARCHAR(255),                   -- optional
    mac_address VARCHAR(17) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    last_login TIMESTAMP,
    is_banned BOOLEAN DEFAULT FALSE,
    ban_reason TEXT,
    CONSTRAINT username_format CHECK (username ~ '^[a-zA-Z0-9_]{3,20}$')
);

CREATE INDEX idx_accounts_username ON accounts(username);
CREATE INDEX idx_accounts_mac ON accounts(mac_address);
```

### characters
```sql
CREATE TABLE characters (
    id SERIAL PRIMARY KEY,
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    username VARCHAR(20) NOT NULL,  -- Denormalized for performance
    
    -- Position
    x SMALLINT NOT NULL DEFAULT 160,
    y SMALLINT NOT NULL DEFAULT 120,
    room_id SMALLINT NOT NULL DEFAULT 1,
    
    -- Appearance
    body_id SMALLINT NOT NULL DEFAULT 1,
    acs1_id SMALLINT NOT NULL DEFAULT 0,
    acs2_id SMALLINT NOT NULL DEFAULT 0,
    
    -- Currency & Stats
    points INTEGER NOT NULL DEFAULT 0 CHECK (points >= 0),
    bank_balance INTEGER NOT NULL DEFAULT 0 CHECK (bank_balance >= 0),
    trees_planted SMALLINT NOT NULL DEFAULT 0,
    objects_built SMALLINT NOT NULL DEFAULT 0,
    
    -- Quest State
    quest_id SMALLINT NOT NULL DEFAULT 0,
    quest_step SMALLINT NOT NULL DEFAULT 0,
    quest_var SMALLINT NOT NULL DEFAULT 0,
    
    -- Permissions
    has_signature BOOLEAN DEFAULT FALSE,
    is_moderator BOOLEAN DEFAULT FALSE,
    
    -- Clan
    clan_id INTEGER REFERENCES clans(id) ON DELETE SET NULL,
    
    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    
    CONSTRAINT one_char_per_account UNIQUE (account_id)
);

CREATE INDEX idx_characters_account ON characters(account_id);
CREATE INDEX idx_characters_room ON characters(room_id);
CREATE INDEX idx_characters_clan ON characters(clan_id);
```

### inventories
```sql
CREATE TABLE inventories (
    character_id INTEGER PRIMARY KEY REFERENCES characters(id) ON DELETE CASCADE,
    
    -- Emote Slots (5)
    emote_1 SMALLINT NOT NULL DEFAULT 0,
    emote_2 SMALLINT NOT NULL DEFAULT 0,
    emote_3 SMALLINT NOT NULL DEFAULT 0,
    emote_4 SMALLINT NOT NULL DEFAULT 0,
    emote_5 SMALLINT NOT NULL DEFAULT 0,
    
    -- Outfit Slots (9)
    outfit_1 SMALLINT NOT NULL DEFAULT 0,
    outfit_2 SMALLINT NOT NULL DEFAULT 0,
    outfit_3 SMALLINT NOT NULL DEFAULT 0,
    outfit_4 SMALLINT NOT NULL DEFAULT 0,
    outfit_5 SMALLINT NOT NULL DEFAULT 0,
    outfit_6 SMALLINT NOT NULL DEFAULT 0,
    outfit_7 SMALLINT NOT NULL DEFAULT 0,
    outfit_8 SMALLINT NOT NULL DEFAULT 0,
    outfit_9 SMALLINT NOT NULL DEFAULT 0,
    
    -- Accessory Slots (9)
    accessory_1 SMALLINT NOT NULL DEFAULT 0,
    accessory_2 SMALLINT NOT NULL DEFAULT 0,
    accessory_3 SMALLINT NOT NULL DEFAULT 0,
    accessory_4 SMALLINT NOT NULL DEFAULT 0,
    accessory_5 SMALLINT NOT NULL DEFAULT 0,
    accessory_6 SMALLINT NOT NULL DEFAULT 0,
    accessory_7 SMALLINT NOT NULL DEFAULT 0,
    accessory_8 SMALLINT NOT NULL DEFAULT 0,
    accessory_9 SMALLINT NOT NULL DEFAULT 0,
    
    -- Item Slots (9)
    item_1 SMALLINT NOT NULL DEFAULT 0,
    item_2 SMALLINT NOT NULL DEFAULT 0,
    item_3 SMALLINT NOT NULL DEFAULT 0,
    item_4 SMALLINT NOT NULL DEFAULT 0,
    item_5 SMALLINT NOT NULL DEFAULT 0,
    item_6 SMALLINT NOT NULL DEFAULT 0,
    item_7 SMALLINT NOT NULL DEFAULT 0,
    item_8 SMALLINT NOT NULL DEFAULT 0,
    item_9 SMALLINT NOT NULL DEFAULT 0,
    
    -- Tool Slots (9)
    tool_1 SMALLINT NOT NULL DEFAULT 0,
    tool_2 SMALLINT NOT NULL DEFAULT 0,
    tool_3 SMALLINT NOT NULL DEFAULT 0,
    tool_4 SMALLINT NOT NULL DEFAULT 0,
    tool_5 SMALLINT NOT NULL DEFAULT 0,
    tool_6 SMALLINT NOT NULL DEFAULT 0,
    tool_7 SMALLINT NOT NULL DEFAULT 0,
    tool_8 SMALLINT NOT NULL DEFAULT 0,
    tool_9 SMALLINT NOT NULL DEFAULT 0,
    
    -- Equipped Tool
    equipped_tool SMALLINT NOT NULL DEFAULT 0,
    
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);
```

### clans
```sql
CREATE TABLE clans (
    id SERIAL PRIMARY KEY,
    name VARCHAR(20) UNIQUE NOT NULL,
    leader_id INTEGER NOT NULL REFERENCES characters(id),
    
    -- Display
    color_inner INTEGER NOT NULL,  -- RGB as integer
    color_outer INTEGER NOT NULL,  -- RGB as integer
    
    -- Stats
    level SMALLINT NOT NULL DEFAULT 1,
    points INTEGER NOT NULL DEFAULT 0,
    max_members SMALLINT NOT NULL DEFAULT 5,
    
    -- Info
    description TEXT,
    news TEXT,
    show_name BOOLEAN DEFAULT TRUE,
    has_base BOOLEAN DEFAULT FALSE,
    
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_clans_leader ON clans(leader_id);
```

### mail
```sql
CREATE TABLE mail (
    id SERIAL PRIMARY KEY,
    sender_id INTEGER REFERENCES characters(id) ON DELETE SET NULL,
    sender_name VARCHAR(20) NOT NULL,  -- Denormalized
    receiver_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    
    subject VARCHAR(50) NOT NULL,
    body TEXT NOT NULL,
    
    -- Attachments
    points_attached INTEGER DEFAULT 0,
    item_category SMALLINT DEFAULT 0,  -- 0=none, 1=outfit, 2=item, 3=acs, 4=card
    item_id SMALLINT DEFAULT 0,
    
    -- Metadata
    paper_style SMALLINT DEFAULT 1,
    font_color SMALLINT DEFAULT 1,
    is_read BOOLEAN DEFAULT FALSE,
    
    sent_at TIMESTAMP NOT NULL DEFAULT NOW(),
    
    CONSTRAINT valid_points CHECK (points_attached >= 0)
);

CREATE INDEX idx_mail_receiver ON mail(receiver_id, is_read);
CREATE INDEX idx_mail_sent_at ON mail(sent_at DESC);
```

### bbs_posts
```sql
CREATE TABLE bbs_posts (
    id SERIAL PRIMARY KEY,
    category_id SMALLINT NOT NULL,
    author_id INTEGER REFERENCES characters(id) ON DELETE SET NULL,
    author_name VARCHAR(20) NOT NULL,
    
    title VARCHAR(100) NOT NULL,
    content TEXT NOT NULL,
    
    is_reported BOOLEAN DEFAULT FALSE,
    report_count SMALLINT DEFAULT 0,
    
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_bbs_category ON bbs_posts(category_id, created_at DESC);
CREATE INDEX idx_bbs_reported ON bbs_posts(is_reported) WHERE is_reported = TRUE;
```

### sessions
```sql
CREATE TABLE sessions (
    id UUID PRIMARY KEY,
    character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    ip_address INET NOT NULL,
    
    -- Connection info
    connected_at TIMESTAMP NOT NULL DEFAULT NOW(),
    last_activity TIMESTAMP NOT NULL DEFAULT NOW(),
    
    -- State
    current_room SMALLINT,
    is_active BOOLEAN DEFAULT TRUE
);

CREATE INDEX idx_sessions_character ON sessions(character_id);
CREATE INDEX idx_sessions_ip ON sessions(ip_address);
CREATE INDEX idx_sessions_activity ON sessions(last_activity) WHERE is_active = TRUE;
```

### bans
```sql
CREATE TABLE bans (
    id SERIAL PRIMARY KEY,
    ban_type VARCHAR(10) NOT NULL,  -- 'ip', 'account', 'mac'
    value VARCHAR(50) NOT NULL,      -- IP address, account_id, or MAC
    reason TEXT NOT NULL,
    banned_by VARCHAR(20),           -- Admin username
    banned_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP,            -- NULL = permanent
    
    CONSTRAINT unique_ban UNIQUE (ban_type, value)
);

CREATE INDEX idx_bans_type_value ON bans(ban_type, value) WHERE expires_at IS NULL OR expires_at > NOW();
```

## Data Migration Strategy

### Initial Setup
```bash
# Create database
createdb slime_online2

# Run migrations
sqlx migrate run
```

### Migration Files

Create in `migrations/` directory:

```
migrations/
├── 20240101000001_create_accounts.sql
├── 20240101000002_create_characters.sql
├── 20240101000003_create_inventories.sql
├── 20240101000004_create_clans.sql
├── 20240101000005_create_mail.sql
├── 20240101000006_create_bbs.sql
├── 20240101000007_create_sessions.sql
├── 20240101000008_create_bans.sql
└── 20240101000009_seed_data.sql
```

## Performance Considerations

### Indexes
- Primary keys: automatic indexes
- Foreign keys: explicit indexes for JOIN performance
- Frequently queried columns: username, room_id, clan_id
- Time-based queries: last_activity, created_at

### Partitioning
For high-traffic servers, consider partitioning:
- `sessions` by month (delete old partitions)
- `mail` by year (archive old mail)
- `bbs_posts` by year

### Caching Strategy
- Cache active player data in Redis
- Cache room player lists in memory
- Cache item definitions (read-only)
- Invalidate on logout/room change

## Backup Strategy

```bash
# Daily full backup
pg_dump -Fc slime_online2 > backup_$(date +%Y%m%d).dump

# Continuous WAL archiving
wal_level = replica
archive_mode = on
archive_command = 'cp %p /mnt/backup/wal/%f'
```

## Connection Pool

```rust
// In main.rs
let pool = sqlx::postgres::PgPoolOptions::new()
    .max_connections(50)
    .connect(&database_url)
    .await?;
```

**Pool Sizing:**
- Max connections: 50 (for 500 concurrent players)
- Min idle: 10
- Acquire timeout: 5 seconds
- Idle timeout: 10 minutes

---

**Next:** See `02-accounts.md` for authentication table details.
