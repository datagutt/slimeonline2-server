# Slime Online 2 - Private Server Implementation (Rust)

Complete specifications for building a legacy-compatible Slime Online 2 server.

## 📚 Documentation

**Start here:** [`DOCUMENTATION_INDEX.md`](./DOCUMENTATION_INDEX.md) - Full documentation overview

**Quick start:** [`docs/IMPLEMENTATION_GUIDE.md`](./docs/IMPLEMENTATION_GUIDE.md) - Phase-by-phase implementation plan

## 🎯 What's Included

✅ **Complete Network Protocol** - TCP/RC4 encryption, all 141 message types  
✅ **Full Database Schema** - PostgreSQL + SQLite for local dev  
✅ **Security Validation** - Comprehensive server-side input validation  
✅ **Architecture Design** - Multi-threaded async Rust with tokio  
✅ **Implementation Roadmap** - 16-week phase-by-phase plan  
✅ **Reference Data** - All constants, item IDs, error codes  

## 🚀 Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install SQLite (for local development)
sudo apt-get install sqlite3 libsqlite3-dev

# OR install PostgreSQL (for production)
sudo apt-get install postgresql postgresql-contrib
```

### Setup

```bash
# Create project
cargo new --bin slime_server
cd slime_server

# Copy documentation
cp -r /path/to/docs ./docs

# Add dependencies to Cargo.toml
[dependencies]
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "migrate"] }
rc4 = "0.1"
bcrypt = "0.15"
bytes = "1"
dashmap = "5"
tracing = "0.1"
tracing-subscriber = "0.3"
serde = { version = "1", features = ["derive"] }
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"

# Create database
sqlite3 slime_online2.db < docs/database/sqlite-schema.sql

# OR for PostgreSQL
createdb slime_online2
psql slime_online2 < docs/database/postgres-schema.sql

# Build
cargo build

# Run
cargo run
```

## 📖 Key Documentation Files

| File | Description | Size |
|------|-------------|------|
| [`docs/protocol/01-connection.md`](docs/protocol/01-connection.md) | TCP connection & RC4 encryption | 19 KB |
| [`docs/protocol/02-message-format.md`](docs/protocol/02-message-format.md) | Binary message format | 23 KB |
| [`docs/protocol/04-message-catalog.md`](docs/protocol/04-message-catalog.md) | All 141 message types | 18 KB |
| [`docs/architecture/01-overview.md`](docs/architecture/01-overview.md) | System architecture | 23 KB |
| [`docs/database/01-schema-overview.md`](docs/database/01-schema-overview.md) | Database schema | 21 KB |
| [`docs/database/08-sqlite-adaptation.md`](docs/database/08-sqlite-adaptation.md) | SQLite for local dev | 15 KB |
| [`docs/security/02-server-validation.md`](docs/security/02-server-validation.md) | Input validation | 36 KB |
| [`docs/reference/01-constants.md`](docs/reference/01-constants.md) | All game constants | 21 KB |

## 🔑 Critical Information

### Encryption Keys (Hardcoded in Client)

```rust
const CLIENT_ENCRYPT_KEY: &[u8] = b"retrtz7jmijb5467n47";
const CLIENT_DECRYPT_KEY: &[u8] = b"t54gz65u74njb6zg6";

// Server decrypts incoming messages with CLIENT_ENCRYPT_KEY
// Server encrypts outgoing messages with CLIENT_DECRYPT_KEY
```

### Client Version

```rust
const VERSION: &str = "0.106";
```

### Default Port

```rust
const PORT: u16 = 5555;
```

## 🛠️ Implementation Phases

1. **Foundation (Week 1-2)** - TCP server, RC4, authentication
2. **Core Gameplay (Week 3-5)** - Movement, chat, rooms
3. **Items & Economy (Week 6-8)** - Inventory, shops, banking
4. **Social Features (Week 9-11)** - Clans, mail, BBS
5. **Game Systems (Week 12-14)** - Quests, planting, special features
6. **Production (Week 15-16)** - Performance, monitoring, deployment

See [`docs/IMPLEMENTATION_GUIDE.md`](docs/IMPLEMENTATION_GUIDE.md) for detailed breakdown.

## 🗃️ Database Choice

**For Local Development:** SQLite
- ✅ No separate DB server needed
- ✅ Fast setup and testing
- ✅ Good for < 50 concurrent players
- 📖 See [`docs/database/08-sqlite-adaptation.md`](docs/database/08-sqlite-adaptation.md)

**For Production:** PostgreSQL
- ✅ Better concurrency (> 50 players)
- ✅ More robust under load
- ✅ Industry standard
- 📖 See [`docs/database/01-schema-overview.md`](docs/database/01-schema-overview.md)

## 🔒 Security Notes

The client uses **RC4 encryption with public keys** - assume all traffic can be intercepted. The server must:

- ✅ **Validate ALL input** - Never trust the client
- ✅ **Server-authoritative** - Server decides what's valid
- ✅ **Rate limiting** - Prevent spam/DDoS
- ✅ **Anomaly detection** - Log suspicious behavior
- ✅ **Quick banning** - Auto-ban on cheat detection

See [`docs/security/02-server-validation.md`](docs/security/02-server-validation.md) for comprehensive examples.

## 📋 Implementation Checklist

### Foundation ✅
- [ ] TCP server listening on port 5555
- [ ] RC4 encryption/decryption
- [ ] Binary message parser (MessageReader/Writer)
- [ ] Database setup (SQLite or PostgreSQL)
- [ ] MSG_LOGIN and MSG_REGISTER handlers
- [ ] bcrypt password hashing

### Core Features
- [ ] Movement synchronization (MSG_MOVE_PLAYER)
- [ ] Chat system (MSG_CHAT)
- [ ] Room management
- [ ] Item system
- [ ] Shop system
- [ ] Clan system
- [ ] Mail system
- [ ] Quest system

### Production Ready
- [ ] All 141 message types handled
- [ ] Comprehensive validation
- [ ] Rate limiting
- [ ] Anti-cheat detection
- [ ] Performance optimization
- [ ] Monitoring & metrics
- [ ] Automated backups

## 🧪 Testing

```bash
# Unit tests
cargo test

# With real client
1. Start server: cargo run
2. Configure client to connect to localhost:5555
3. Test login/registration
4. Test movement, chat, items
5. Test with multiple clients

# Load testing
# See docs/IMPLEMENTATION_GUIDE.md for load testing strategies
```

## 📊 Performance Targets

- **Latency:** < 50ms message processing
- **Throughput:** 10,000 messages/second
- **Concurrent Players:** 500+ per instance
- **Memory:** < 2GB for 500 players

## 🤝 Contributing

This is a private server emulator for the legacy v0.106 client. The client cannot be modified.

**Guidelines:**
- Maintain 100% protocol compatibility
- All security improvements must be server-side only
- Document all changes thoroughly
- Test with real v0.106 client

## 📄 License

This is a reverse-engineered protocol specification for educational and preservation purposes.

## 🆘 Support

For implementation questions:
1. Check [`DOCUMENTATION_INDEX.md`](DOCUMENTATION_INDEX.md)
2. Review specific documentation in `docs/`
3. Test behavior with decompiled client code
4. Validate with real v0.106 client

## 📝 Notes

- **Client Version:** 0.106 (fixed, cannot change)
- **Protocol:** Custom binary over TCP with RC4 encryption
- **Compatibility:** 100% with original client (no modifications)
- **Security:** Server-side validation only (client is untrusted)

---

**Ready to implement!** Start with [`docs/IMPLEMENTATION_GUIDE.md`](docs/IMPLEMENTATION_GUIDE.md) 🎮🦀
