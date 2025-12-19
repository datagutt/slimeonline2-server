# Slime Online 2 Server - Implementation Guide

## Quick Reference

This guide provides a roadmap for implementing the complete Slime Online 2 server in Rust.

## Documentation Index

### 📡 Protocol Specifications
1. **[Connection & Encryption](protocol/01-connection.md)** - TCP, RC4, connection lifecycle
2. **[Message Format](protocol/02-message-format.md)** - Binary protocol, data types, parsing
3. **[Authentication](protocol/03-authentication.md)** - Login/register flows *(to be created)*
4. **[Message Catalog](protocol/04-message-catalog.md)** - All 141 message types
5. **[Movement Protocol](protocol/05-movement-protocol.md)** - Player movement sync *(to be created)*

### 🏗️ Architecture
1. **[System Overview](architecture/01-overview.md)** - High-level design, components, threading
2. **[Connection Manager](architecture/02-connection-manager.md)** - TCP handling *(to be created)*
3. **[World Manager](architecture/03-world-manager.md)** - Game state *(to be created)*
4. **[Player Manager](architecture/04-player-manager.md)** - Sessions *(to be created)*
5. **[Room System](architecture/05-room-system.md)** - Room instances *(to be created)*

### 🗄️ Database
1. **[Schema Overview](database/01-schema-overview.md)** - Full database schema
2. **[Accounts](database/02-accounts.md)** - Authentication tables *(to be created)*
3. **[Characters](database/03-characters.md)** - Player data *(to be created)*
4. **[Items & Inventory](database/04-items-inventory.md)** - Item system *(to be created)*
5. **[Clans](database/05-clans.md)** - Clan tables *(to be created)*

### 🛡️ Security
1. **[Threat Model](security/01-threat-model.md)** - Security considerations *(to be created)*
2. **[Server Validation](security/02-server-validation.md)** - Comprehensive input validation
3. **[Anti-Cheat](security/03-anti-cheat.md)** - Cheat detection *(to be created)*
4. **[Rate Limiting](security/04-rate-limiting.md)** - DDoS protection *(to be created)*

### 📚 Reference
1. **[Constants](reference/01-constants.md)** - All game constants (141 message types, limits, etc.)
2. **[Item Database](reference/02-item-database.md)** - Item definitions *(to be created)*
3. **[Room Database](reference/03-room-database.md)** - Map data *(to be created)*

## Implementation Phases

### Phase 1: Foundation (Week 1-2)

**Goal:** TCP server with authentication

**Tasks:**
1. Set up Rust project with dependencies
   ```toml
   [dependencies]
   tokio = { version = "1", features = ["full"] }
   sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres"] }
   rc4 = "0.1"
   bcrypt = "0.15"
   bytes = "1"
   dashmap = "5"
   tracing = "0.1"
   tracing-subscriber = "0.3"
   serde = { version = "1", features = ["derive"] }
   uuid = { version = "1", features = ["v4"] }
   chrono = "0.4"
   ```

2. Implement RC4 encryption (see `protocol/01-connection.md`)
3. Create MessageReader/Writer (see `protocol/02-message-format.md`)
4. Set up PostgreSQL database with schemas (see `database/01-schema-overview.md`)
5. Implement MSG_LOGIN and MSG_REGISTER (see `protocol/04-message-catalog.md`)
6. Create connection manager with rate limiting

**Deliverables:**
- [ ] Client can connect and login
- [ ] Client can create account
- [ ] Server validates credentials with bcrypt
- [ ] Server enforces connection limits
- [ ] Database persists accounts and characters

**Testing:**
```bash
# Should be able to:
1. Connect with real client
2. Register new account
3. Login with account
4. See login failure messages for wrong password
5. See "already logged in" if logging in twice
```

### Phase 2: Core Gameplay (Week 3-5)

**Goal:** Movement, chat, rooms

**Tasks:**
1. Implement MSG_NEW_PLAYER broadcast
2. Implement MSG_MOVE_PLAYER with validation (see `security/02-server-validation.md`)
3. Implement MSG_CHAT with profanity filter
4. Create Room system for player grouping
5. Implement MSG_WARP for room changes
6. Implement MSG_PING for keepalive
7. Add player position tracking in database

**Deliverables:**
- [ ] Multiple players can login simultaneously
- [ ] Players see each other in same room
- [ ] Movement is synchronized across clients
- [ ] Chat messages broadcast to room
- [ ] Players can warp between rooms
- [ ] Ping/pong keeps connections alive

**Testing:**
```bash
# With 2 clients:
1. Both login to same room
2. Both see each other
3. One moves, other sees movement
4. One types chat, other sees message
5. One warps, disappears from first client
```

### Phase 3: Items & Economy (Week 6-8)

**Goal:** Inventory, shops, items

**Tasks:**
1. Implement item system (use, discard, pick up)
2. Implement shop system (browse, buy)
3. Implement bank system (deposit, withdraw, transfer)
4. Implement storage system
5. Add item effects (warp-wing, sodas, etc.)
6. Implement point transactions with overflow protection

**Deliverables:**
- [ ] Players can use items from inventory
- [ ] Players can buy from shops
- [ ] Players can deposit/withdraw from bank
- [ ] Players can store items in storage
- [ ] Items have proper effects (teleport, invisibility, etc.)
- [ ] Currency transactions are validated

**Testing:**
```bash
1. Buy item from shop (points decrease)
2. Use consumable item (item removed, effect applied)
3. Discard item (appears on ground)
4. Pick up discarded item
5. Deposit points to bank
6. Try to buy with insufficient funds (fails)
```

### Phase 4: Social Features (Week 9-11)

**Goal:** Clans, mail, BBS

**Tasks:**
1. Implement clan creation/dissolution
2. Implement clan invites/kicks
3. Implement clan leveling system
4. Implement mail sending/receiving
5. Implement mail attachments (items, points)
6. Implement BBS (categories, posts, reading)
7. Implement BBS reporting system

**Deliverables:**
- [ ] Players can create clans
- [ ] Players can invite others to clans
- [ ] Clan names display above players
- [ ] Players can send mail with attachments
- [ ] Players can read/delete mail
- [ ] Players can post to BBS
- [ ] Players can report inappropriate BBS posts

**Testing:**
```bash
1. Create clan (costs 10k points, requires items)
2. Invite another player
3. Send mail with attached item
4. Receive mail and take attachment
5. Post to BBS
6. Report offensive post
```

### Phase 5: Game Systems (Week 12-14)

**Goal:** Quests, planting, special features

**Tasks:**
1. Implement quest system (start, progress, complete)
2. Implement planting system (seeds, growth, harvest)
3. Implement collectible spawning
4. Implement cannon system
5. Implement racing system
6. Implement building system
7. Add all remaining message handlers

**Deliverables:**
- [ ] Players can start and complete quests
- [ ] Players can plant trees and harvest fruits
- [ ] Collectibles spawn in rooms
- [ ] Cannons work (enter, aim, shoot)
- [ ] Racing checkpoints work
- [ ] Players can build objects

**Testing:**
```bash
1. Talk to NPC, start quest
2. Complete quest objective
3. Plant seed at plant spot
4. Wait for tree to grow
5. Harvest fruit
6. Collect collectible from room
```

### Phase 6: Polish & Production (Week 15-16)

**Goal:** Performance, monitoring, deployment

**Tasks:**
1. Comprehensive input validation (see `security/02-server-validation.md`)
2. Rate limiting on all operations
3. Anti-cheat detection (teleporting, item duping, etc.)
4. Performance optimization (benchmarking, profiling)
5. Load testing (500 concurrent players)
6. Monitoring setup (Prometheus metrics)
7. Deployment guide (systemd service, backups)
8. Admin commands (kick, ban, announce)

**Deliverables:**
- [ ] All inputs validated server-side
- [ ] Rate limits prevent spam/DDoS
- [ ] Suspicious activity logged
- [ ] Server handles 500+ concurrent players
- [ ] Metrics exported to Prometheus
- [ ] Automated backups configured
- [ ] Admin can kick/ban players

**Testing:**
```bash
1. Try to teleport (server rejects)
2. Try to send 100 messages/sec (rate limited)
3. Try to buy item with modified points value (rejected)
4. Run load test: 500 concurrent players
5. Monitor metrics during load test
6. Admin can ban player by username/IP
```

## Critical Implementation Notes

### 🚨 NEVER Change Client

The client is **fixed at version 0.106** and **cannot be modified**. All improvements must be server-side.

**What you CANNOT change:**
- ❌ Encryption algorithm (must use RC4)
- ❌ Encryption keys (hardcoded in client)
- ❌ Message format (binary protocol)
- ❌ Message types (must handle all 141)
- ❌ Protocol version string ("0.106")

**What you CAN change:**
- ✅ Server-side validation logic
- ✅ Database schema
- ✅ Rate limiting parameters
- ✅ Ban system
- ✅ Anti-cheat detection
- ✅ Game balance (shop prices, etc.)

### 🔐 Security is Critical

The client uses weak encryption (RC4 with public keys). Assume **all traffic can be decrypted and modified** by attackers.

**Defense strategy:**
1. **Server-authoritative:** Server validates ALL actions
2. **Never trust client:** Validate every message field
3. **Rate limiting:** Prevent spam/DDoS
4. **Anomaly detection:** Log suspicious behavior
5. **Quick banning:** Auto-ban on cheat detection

### 📊 Performance Targets

- **Latency:** < 50ms message processing
- **Throughput:** 10,000 messages/second
- **Concurrent Players:** 500+ per instance
- **Memory:** < 2GB for 500 players
- **Database:** < 10ms query time (p95)

### 🧪 Testing Strategy

**Unit Tests:**
- Message parsing/serialization
- RC4 encryption/decryption
- Validation logic
- Database queries

**Integration Tests:**
- Login/logout flow
- Movement synchronization
- Item transactions
- Clan operations

**Load Tests:**
- 100/500/1000 concurrent connections
- Message throughput under load
- Database performance under load

**Client Tests:**
- Test with REAL v0.106 client
- Verify all features work end-to-end

## Development Environment

### Required Software

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# PostgreSQL
sudo apt-get install postgresql postgresql-contrib

# sqlx-cli for migrations
cargo install sqlx-cli --no-default-features --features postgres

# Optional: Docker for isolated DB
docker run -d \
  -e POSTGRES_PASSWORD=password \
  -e POSTGRES_DB=slime_online2 \
  -p 5432:5432 \
  postgres:14
```

### Project Setup

```bash
# Clone/create project
cargo new --bin slime_server
cd slime_server

# Add dependencies (see Phase 1)
# Edit Cargo.toml

# Create database
createdb slime_online2

# Set DATABASE_URL
export DATABASE_URL=postgres://postgres:password@localhost/slime_online2

# Run migrations
sqlx migrate run

# Build
cargo build --release

# Run
./target/release/slime_server
```

### Configuration

Create `config.toml`:
```toml
[server]
host = "0.0.0.0"
port = 5555

[database]
url = "postgres://postgres:password@localhost/slime_online2"
max_connections = 50

[game]
motd = "Welcome to Slime Online 2 Private Server!"
version = "0.106"

[security]
max_connections_per_ip = 3
max_login_attempts = 5
bcrypt_cost = 12
```

## Debugging Tips

### Enable Debug Logging

```rust
// In main.rs
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .with_target(false)
    .with_thread_ids(true)
    .with_line_number(true)
    .init();
```

### Packet Capture

```bash
# Capture client traffic
tcpdump -i lo -w slime.pcap port 5555

# View in Wireshark
wireshark slime.pcap
```

### Database Queries

```bash
# Connect to DB
psql slime_online2

# Check active players
SELECT id, username, x, y, room_id FROM characters
WHERE id IN (SELECT character_id FROM sessions WHERE is_active = true);

# Check account with most points
SELECT username, points FROM characters ORDER BY points DESC LIMIT 10;
```

## Common Issues

### Client Won't Connect

1. Check server is listening: `netstat -tlnp | grep 5555`
2. Check firewall: `sudo ufw allow 5555`
3. Verify encryption keys match exactly
4. Check client version string is "0.106"

### Authentication Fails

1. Verify bcrypt is working: run unit test
2. Check database connection: `psql slime_online2`
3. Verify password hashing during registration
4. Check logs for error messages

### Movement Desync

1. Verify movement validation logic
2. Check broadcast is sending to all room players
3. Verify position updates are atomic
4. Check for race conditions in room player list

### Database Too Slow

1. Add indexes on frequently queried columns
2. Use connection pooling (sqlx pool)
3. Optimize queries (use EXPLAIN ANALYZE)
4. Consider read replicas for scaling

## Production Deployment

### Systemd Service

Create `/etc/systemd/system/slime-server.service`:
```ini
[Unit]
Description=Slime Online 2 Server
After=network.target postgresql.service

[Service]
Type=simple
User=slime
WorkingDirectory=/opt/slime-server
Environment=DATABASE_URL=postgres://slime:***@localhost/slime_online2
ExecStart=/opt/slime-server/slime_server
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable slime-server
sudo systemctl start slime-server
sudo systemctl status slime-server
```

### Monitoring

```bash
# Prometheus metrics endpoint
GET http://localhost:9090/metrics

# Example metrics:
slime_active_players 42
slime_messages_per_second 523
slime_database_query_duration_seconds{query="get_player"} 0.005
```

### Backups

```bash
# Daily backup script
#!/bin/bash
DATE=$(date +%Y%m%d)
pg_dump -Fc slime_online2 > /backups/slime_$DATE.dump

# Keep last 30 days
find /backups -name "slime_*.dump" -mtime +30 -delete
```

### Restore

```bash
pg_restore -d slime_online2 /backups/slime_20240101.dump
```

## Support

For questions during implementation:
1. Refer to specific documentation files in `docs/`
2. Check client decompiled code for exact behavior
3. Test with real v0.106 client
4. Validate against message catalog

---

**Remember:** This is a legacy protocol emulator. Focus on compatibility first, then security (server-side only), then features.

Good luck with your implementation! 🎮
