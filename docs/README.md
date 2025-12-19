# Slime Online 2 - Private Server Documentation

## Overview

This documentation provides complete specifications for implementing a **legacy-compatible** private server for Slime Online 2 (v0.106) in Rust. The server must maintain 100% compatibility with the existing GameMaker client without any client-side modifications.

**⚠️ IMPORTANT: Legacy Client Compatibility**

- The client uses **RC4 encryption with hardcoded keys** - this cannot be changed
- The client expects specific TCP protocol behavior - must be replicated exactly
- All security improvements must be **server-side only**
- No TLS, no protocol changes, no client modifications

## Documentation Structure

```
docs/
├── README.md                          # This file
├── protocol/                          # Network protocol specifications
│   ├── 01-connection.md              # TCP connection & encryption
│   ├── 02-message-format.md          # Binary message format
│   ├── 03-authentication.md          # Login & registration
│   ├── 04-message-catalog.md         # All 141 message types
│   ├── 05-movement-protocol.md       # Player movement sync
│   └── 06-timing-and-sync.md         # Ping, time, keepalive
├── architecture/                      # Server architecture
│   ├── 01-overview.md                # High-level architecture
│   ├── 02-connection-manager.md      # TCP connection handling
│   ├── 03-world-manager.md           # Game world state
│   ├── 04-player-manager.md          # Player session management
│   ├── 05-room-system.md             # Room/map instances
│   └── 06-event-system.md            # Event-driven architecture
├── database/                          # Database schemas
│   ├── 01-schema-overview.md         # Database structure
│   ├── 02-accounts.md                # Account/authentication tables
│   ├── 03-characters.md              # Character/player data
│   ├── 04-items-inventory.md         # Items & inventory
│   ├── 05-clans.md                   # Clan system
│   ├── 06-mail-bbs.md                # Mail & bulletin board
│   └── 07-world-state.md             # World persistence
├── game-systems/                      # Game mechanics
│   ├── 01-movement-physics.md        # Movement validation
│   ├── 02-item-system.md             # Item mechanics
│   ├── 03-quest-system.md            # Quest implementation
│   ├── 04-shop-economy.md            # Shops & economy
│   ├── 05-clan-system.md             # Clan features
│   ├── 06-planting-system.md         # Tree farming
│   ├── 07-collectibles.md            # Collectible items
│   ├── 08-mail-system.md             # Mail delivery
│   ├── 09-bbs-system.md              # Bulletin board
│   └── 10-special-features.md        # Cannons, racing, etc
├── security/                          # Security considerations
│   ├── 01-threat-model.md            # Security threats
│   ├── 02-server-validation.md       # Server-side validation
│   ├── 03-anti-cheat.md              # Cheat detection
│   ├── 04-rate-limiting.md           # DDoS protection
│   └── 05-ban-system.md              # Account/IP banning
└── reference/                         # Reference data
    ├── 01-constants.md               # All game constants
    ├── 02-item-database.md           # Item definitions
    ├── 03-room-database.md           # Room/map data
    ├── 04-npc-database.md            # NPC definitions
    └── 05-error-codes.md             # Error handling
```

## Implementation Philosophy

### 1. **Full Compatibility**
- Must work with unmodified v0.106 client
- Replicate exact binary protocol
- Match expected server responses
- Handle all 141 message types

### 2. **Server-Side Security**
- Validate ALL client input (never trust client)
- Rate limit all operations
- Server-authoritative for game state
- Secure password storage (bcrypt)
- Session management with timeouts
- IP/account banning system

### 3. **No Shortcuts**
- Implement ALL features completely
- No TODO placeholders in production
- Comprehensive error handling
- Full database persistence
- Proper concurrency management

### 4. **Production Quality**
- Structured logging
- Metrics and monitoring
- Configuration management
- Database migrations
- Graceful shutdown
- Crash recovery

## Technology Stack

### Required Rust Crates

**Networking:**
- `tokio` - Async runtime
- `tokio::net::TcpListener` - TCP server
- `bytes` - Efficient byte buffer handling

**Encryption:**
- `rc4` - Legacy RC4 cipher (required for client compat)

**Database:**
- `sqlx` - Async SQL with compile-time checked queries
- `sqlx::postgres` - PostgreSQL driver (recommended)

**Serialization:**
- Custom binary protocol (matches client format)

**Security:**
- `bcrypt` - Password hashing
- `argon2` - Alternative password hashing
- `uuid` - Session tokens

**Utilities:**
- `tracing` - Structured logging
- `serde` - Configuration serialization
- `chrono` - Date/time handling
- `dashmap` - Concurrent hashmap

## Quick Start Guide

### 1. Read Core Protocol Documentation
Start with these files in order:
1. `protocol/01-connection.md` - Understand TCP/encryption
2. `protocol/02-message-format.md` - Binary message structure
3. `protocol/03-authentication.md` - Login flow
4. `protocol/04-message-catalog.md` - All message types

### 2. Understand Architecture
1. `architecture/01-overview.md` - System design
2. `architecture/02-connection-manager.md` - Connection handling
3. `architecture/03-world-manager.md` - Game world state

### 3. Database Setup
1. `database/01-schema-overview.md` - Database structure
2. Set up PostgreSQL with provided schemas
3. Initialize with reference data

### 4. Implement Core Systems
Follow implementation order:
1. TCP server with RC4 encryption
2. Message parsing/serialization
3. Authentication system
4. Player session management
5. Room/world system
6. Individual game features

### 5. Security Hardening
1. `security/02-server-validation.md` - Implement all validations
2. `security/04-rate-limiting.md` - Add rate limits
3. `security/03-anti-cheat.md` - Deploy cheat detection

## Key Design Constraints

### Protocol Constraints (MUST NOT CHANGE)
- ✅ RC4 encryption with keys `t54gz65u74njb6zg6` (decrypt) and `retrtz7jmijb5467n47` (encrypt)
- ✅ Binary message format: `[u16 msg_type][...payload]`
- ✅ Null-terminated strings
- ✅ Little-endian byte order
- ✅ TCP only (port configurable, typically 5555)

### Server Freedoms (CAN CHANGE)
- ✅ Database schema design
- ✅ Internal state management
- ✅ Server-side validation logic
- ✅ Rate limiting parameters
- ✅ Ban/moderation systems
- ✅ Logging and monitoring
- ✅ Configuration management

## Development Workflow

### Phase 1: Foundation (Weeks 1-2)
- [ ] TCP server with RC4 encryption
- [ ] Message parser/serializer
- [ ] Database schema and migrations
- [ ] Basic authentication (login/register)
- [ ] Session management

### Phase 2: Core Gameplay (Weeks 3-5)
- [ ] Player spawning and persistence
- [ ] Movement synchronization
- [ ] Room/map system
- [ ] Basic item system
- [ ] Chat system

### Phase 3: Game Systems (Weeks 6-10)
- [ ] Shop and economy
- [ ] Quest system
- [ ] Clan system
- [ ] Planting/farming
- [ ] Mail system
- [ ] Bulletin board
- [ ] Collectibles

### Phase 4: Advanced Features (Weeks 11-12)
- [ ] Racing system
- [ ] Cannon system
- [ ] Building system
- [ ] Special items and effects

### Phase 5: Polish & Security (Weeks 13-14)
- [ ] Comprehensive input validation
- [ ] Rate limiting
- [ ] Anti-cheat detection
- [ ] Performance optimization
- [ ] Load testing
- [ ] Documentation completion

## Testing Strategy

### Unit Tests
- Message parsing/serialization
- RC4 encryption/decryption
- Game logic (movement, items, etc)
- Database queries

### Integration Tests
- Full login/logout flow
- Movement synchronization
- Item transactions
- Quest progression
- Clan operations

### Load Tests
- 100 concurrent players
- 1000 messages/second
- Database performance
- Memory leak detection

### Compatibility Tests
- Test with actual v0.106 client
- Verify all 141 message types
- Test edge cases (disconnects, timeouts)
- Cross-platform compatibility

## Performance Targets

- **Latency:** < 50ms message processing
- **Throughput:** 10,000 messages/second
- **Concurrent Players:** 500+ per server instance
- **Memory:** < 2GB for 500 players
- **CPU:** < 50% on 4-core server
- **Database:** < 10ms query time (p95)

## Monitoring & Operations

### Metrics to Track
- Active player count
- Messages processed/second
- Authentication success/failure rate
- Average ping time
- Database query performance
- Error rates by message type
- Ban/kick events

### Log Levels
- **ERROR:** Server errors, crashes, critical failures
- **WARN:** Suspicious activity, failed validations, rate limits
- **INFO:** Player connections, major events, state changes
- **DEBUG:** Message traces, detailed state
- **TRACE:** Full binary protocol dumps (dev only)

## Security Checklist

Before deploying to production:

- [ ] All user input is validated server-side
- [ ] Passwords are hashed with bcrypt (cost 12+)
- [ ] SQL injection prevention (parameterized queries)
- [ ] Rate limiting on all endpoints
- [ ] Session timeout enforcement
- [ ] IP ban system functional
- [ ] Account ban system functional
- [ ] Movement validation prevents teleporting
- [ ] Item duplication checks in place
- [ ] Point/currency validation prevents overflow
- [ ] Database backups configured
- [ ] Error messages don't leak internal state
- [ ] Admin commands require authentication
- [ ] Logging configured for security events

## Support & Maintenance

### Configuration Files
All runtime configuration in `config.toml`:
- Server IP/port
- Database connection string
- Rate limit parameters
- Feature flags
- Debug options

### Database Migrations
Use `sqlx migrate` for schema changes:
```bash
sqlx migrate add description_of_change
# Edit generated migration file
sqlx migrate run
```

### Backup Strategy
- Daily full database backup
- Transaction log backups every 15 minutes
- Retain backups for 30 days
- Test restore procedure monthly

## Getting Help

When implementing features, always refer to:
1. Protocol specification in `protocol/04-message-catalog.md`
2. Client behavior analysis in original documentation
3. Database schema in `database/` folder
4. Security guidelines in `security/` folder

## Version History

- **v1.0.0** - Initial documentation for v0.106 client compatibility
- Target: 100% feature completeness
- No planned protocol changes (legacy compatibility)

## Next Steps

1. Read `protocol/01-connection.md` to understand TCP/encryption
2. Review `architecture/01-overview.md` for system design
3. Set up development environment with Rust toolchain
4. Initialize database with schemas from `database/`
5. Begin implementation with connection manager

---

**Remember:** This is a legacy protocol emulation project. The client cannot be changed. All improvements must be server-side only.
