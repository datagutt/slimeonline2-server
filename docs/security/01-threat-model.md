# Threat Model

**See:** [`02-server-validation.md`](02-server-validation.md) for comprehensive validation examples.

## Threat Categories

### 1. Network Threats
- **Man-in-the-Middle:** RC4 with public keys - assume all traffic can be intercepted
- **DDoS:** Rate limiting required (see [`02-server-validation.md`](02-server-validation.md))
- **Replay Attacks:** Session tokens and timestamps

### 2. Client Tampering
- **Modified Client:** Assume client is untrusted, validate everything server-side
- **Packet Injection:** Validate all message structures
- **Memory Editing:** Server-authoritative game state

### 3. Exploitation
- **SQL Injection:** Use parameterized queries (sqlx)
- **Buffer Overflow:** Rust memory safety
- **Integer Overflow:** Checked arithmetic for currency
- **Path Traversal:** Validate all file paths

### 4. Game Exploits
- **Duplication:** Transaction-based item handling
- **Teleportation:** Position validation
- **Speed Hacks:** Movement rate limiting
- **Currency Overflow:** i32 bounds checking

## Mitigation Strategies

All mitigations are documented in [`02-server-validation.md`](02-server-validation.md).

See also: [`../protocol/01-connection.md`](../protocol/01-connection.md) for encryption details.
