# Protocol: Authentication Flow

## Overview

Authentication is the first interaction between client and server. The client must successfully authenticate before any other game operations are allowed.

## Connection Sequence

```
1. Client establishes TCP connection to server port 5555
2. Client immediately sends MSG_LOGIN or MSG_REGISTER
3. Server validates and responds
4. If successful, client receives full account data
5. Client transitions to authenticated state
```

## MSG_REGISTER (7) - Account Registration

### Client Request

```rust
struct RegisterRequest {
    msg_type: u16,      // 7
    version: String,    // "0.106"
    username: String,   // 3-20 characters
    password: String,   // Plaintext
    mac_address: String // Hardware ID
}
```

**Binary Layout:**
```
[u16: 7][string: "0.106"][string: username][string: password][string: mac]
```

### Server Validation

```rust
async fn validate_register(req: RegisterRequest, ip: IpAddr) -> Result<()> {
    // 1. Version check
    if req.version != "0.106" {
        return Err(ValidationError::VersionMismatch);
    }
    
    // 2. Username validation
    if req.username.len() < 3 || req.username.len() > 20 {
        return Err(ValidationError::InvalidUsername);
    }
    
    if !req.username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(ValidationError::InvalidUsernameChars);
    }
    
    // 3. Check if username exists (case-insensitive)
    if username_exists(&req.username).await? {
        return Err(ValidationError::UsernameExists);
    }
    
    // 4. Check IP ban
    if is_ip_banned(&ip).await? {
        return Err(ValidationError::IpBanned);
    }
    
    // 5. Check MAC ban
    if is_mac_banned(&req.mac_address).await? {
        return Err(ValidationError::MacBanned);
    }
    
    // 6. Rate limit: max 3 accounts per IP per day
    if get_accounts_created_today(&ip).await? >= 3 {
        return Err(ValidationError::TooManyAccounts);
    }
    
    Ok(())
}
```

### Server Response

```rust
struct RegisterResponse {
    msg_type: u16,  // 7
    result: u8,     // See result codes below
}

// Result codes:
const REGISTER_SUCCESS: u8 = 1;
const REGISTER_EXISTS: u8 = 2;
const REGISTER_IP_BANNED: u8 = 3;
const REGISTER_MAC_BANNED: u8 = 4;
```

**Success Flow:**
```rust
async fn handle_register_success(username: &str, password: &str, mac: &str) -> Result<()> {
    // 1. Hash password with bcrypt (cost 12)
    let hash = bcrypt::hash(password, 12)?;
    
    // 2. Insert into database
    sqlx::query!(
        "INSERT INTO accounts (username, password_hash, mac_address, created_at)
         VALUES ($1, $2, $3, NOW())",
        username, hash, mac
    )
    .execute(&pool)
    .await?;
    
    // 3. Create default character
    let account_id = get_account_id(username).await?;
    create_default_character(account_id, username).await?;
    
    // 4. Send success response
    let mut writer = MessageWriter::new();
    writer.write_u16(MSG_REGISTER);
    writer.write_u8(REGISTER_SUCCESS);
    send_encrypted(socket, writer.into_bytes()).await?;
    
    Ok(())
}
```

### Default Character Creation

```rust
async fn create_default_character(account_id: u32, username: &str) -> Result<u16> {
    let character_id = sqlx::query!(
        "INSERT INTO characters (
            account_id, username, x, y, room_id, 
            body_id, points, created_at
        ) VALUES ($1, $2, 160, 120, 1, 1, 0, NOW())
        RETURNING id",
        account_id as i32,
        username
    )
    .fetch_one(&pool)
    .await?
    .id as u16;
    
    // Create empty inventory
    sqlx::query!(
        "INSERT INTO inventories (character_id) VALUES ($1)",
        character_id as i32
    )
    .execute(&pool)
    .await?;
    
    Ok(character_id)
}
```

## MSG_LOGIN (10) - Account Login

### Client Request

```rust
struct LoginRequest {
    msg_type: u16,      // 10
    version: String,    // "0.106"
    username: String,
    password: String,   // Plaintext
    mac_address: String
}
```

### Server Validation

```rust
async fn validate_login(req: LoginRequest, ip: IpAddr) -> Result<Account> {
    // 1. Version check
    if req.version != "0.106" {
        return Err(ValidationError::VersionMismatch);
    }
    
    // 2. Check IP ban FIRST (before DB query)
    if is_ip_banned(&ip).await? {
        return Err(ValidationError::IpBanned);
    }
    
    // 3. Fetch account
    let account = sqlx::query_as!(
        Account,
        "SELECT * FROM accounts WHERE username = $1",
        req.username
    )
    .fetch_optional(&pool)
    .await?
    .ok_or(ValidationError::AccountNotFound)?;
    
    // 4. Check account ban
    if account.is_banned {
        return Err(ValidationError::AccountBanned);
    }
    
    // 5. Verify password with bcrypt
    if !bcrypt::verify(&req.password, &account.password_hash)? {
        log_failed_login(account.id, &ip).await;
        increment_failed_attempts(&ip).await;
        return Err(ValidationError::WrongPassword);
    }
    
    // 6. Check if already logged in
    if is_session_active(account.id).await? {
        // Kick old session
        kick_session(account.id).await?;
    }
    
    // 7. Check MAC ban
    if is_mac_banned(&req.mac_address).await? {
        return Err(ValidationError::MacBanned);
    }
    
    // 8. Rate limit: max 5 login attempts per IP per minute
    if get_login_attempts_last_minute(&ip).await? > 5 {
        return Err(ValidationError::TooManyAttempts);
    }
    
    Ok(account)
}
```

### Server Response (Success)

```rust
struct LoginSuccess {
    msg_type: u16,      // 10
    case: u8,           // 1 = success
    player_id: u16,
    server_time: u32,   // Unix timestamp
    motd: String,       // Message of the day
    day: u8,            // 1-7 (Sun-Sat)
    hour: u8,           // 0-23
    minute: u8,         // 0-59
    username: String,   // Echo back
    spawn_x: u16,
    spawn_y: u16,
    spawn_room: u16,
    body_id: u16,
    acs1_id: u16,
    acs2_id: u16,
    points: u32,
    signature: u8,      // 0/1
    quest_id: u16,
    quest_step: u8,
    trees_planted: u16,
    objects_built: u16,
    emotes: [u8; 5],    // 5 emote slots
    outfits: [u16; 9],  // 9 outfit slots
    accessories: [u16; 9], // 9 accessory slots
    items: [u16; 9],    // 9 item slots
    tools: [u8; 9],     // 9 tool slots
}
```

**Implementation:**
```rust
async fn send_login_success(socket: &mut TcpStream, account_id: u32) -> Result<()> {
    // 1. Load character data
    let character = load_character(account_id).await?;
    let inventory = load_inventory(character.id).await?;
    
    // 2. Create session
    let session_id = Uuid::new_v4();
    create_session(session_id, character.id, socket.peer_addr()?).await?;
    
    // 3. Build response
    let mut writer = MessageWriter::new();
    writer.write_u16(MSG_LOGIN);
    writer.write_u8(1); // Success case
    writer.write_u16(character.id);
    writer.write_u32(get_server_time());
    writer.write_string(&get_motd());
    
    // Game time
    let (day, hour, minute) = get_game_time();
    writer.write_u8(day);
    writer.write_u8(hour);
    writer.write_u8(minute);
    
    // Character data
    writer.write_string(&character.username);
    writer.write_u16(character.x);
    writer.write_u16(character.y);
    writer.write_u16(character.room_id);
    writer.write_u16(character.body_id);
    writer.write_u16(character.acs1_id);
    writer.write_u16(character.acs2_id);
    writer.write_u32(character.points);
    writer.write_u8(character.has_signature as u8);
    writer.write_u16(character.quest_id);
    writer.write_u8(character.quest_step);
    writer.write_u16(character.trees_planted);
    writer.write_u16(character.objects_built);
    
    // Inventory arrays
    for i in 0..5 {
        writer.write_u8(inventory.emotes[i]);
    }
    for i in 0..9 {
        writer.write_u16(inventory.outfits[i]);
    }
    for i in 0..9 {
        writer.write_u16(inventory.accessories[i]);
    }
    for i in 0..9 {
        writer.write_u16(inventory.items[i]);
    }
    for i in 0..9 {
        writer.write_u8(inventory.tools[i]);
    }
    
    // 4. Send encrypted
    send_encrypted(socket, writer.into_bytes()).await?;
    
    // 5. Update last login
    update_last_login(account_id).await?;
    
    Ok(())
}
```

### Server Response (Failure)

```rust
struct LoginFailure {
    msg_type: u16,  // 10
    case: u8,       // Error code
}

const LOGIN_SUCCESS: u8 = 1;
const LOGIN_NO_ACCOUNT: u8 = 2;
const LOGIN_WRONG_PASSWORD: u8 = 3;
const LOGIN_ALREADY_LOGGED_IN: u8 = 4;
const LOGIN_VERSION_MISMATCH: u8 = 5;
const LOGIN_ACCOUNT_BANNED: u8 = 6;
const LOGIN_IP_BANNED_1: u8 = 7;
const LOGIN_IP_BANNED_2: u8 = 8;
```

## Session Management

### Creating Sessions

```rust
async fn create_session(
    session_id: Uuid,
    character_id: u16,
    addr: SocketAddr,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO sessions (id, character_id, ip_address, connected_at, last_activity, is_active)
         VALUES ($1, $2, $3, NOW(), NOW(), true)",
        session_id.to_string(),
        character_id as i32,
        addr.ip().to_string()
    )
    .execute(&pool)
    .await?;
    
    Ok(())
}
```

### Checking Active Sessions

```rust
async fn is_session_active(character_id: u16) -> Result<bool> {
    let count = sqlx::query!(
        "SELECT COUNT(*) as count FROM sessions 
         WHERE character_id = $1 AND is_active = true",
        character_id as i32
    )
    .fetch_one(&pool)
    .await?
    .count
    .unwrap_or(0);
    
    Ok(count > 0)
}
```

### Kicking Old Sessions

```rust
async fn kick_session(character_id: u16) -> Result<()> {
    // 1. Mark session as inactive
    sqlx::query!(
        "UPDATE sessions SET is_active = false 
         WHERE character_id = $1 AND is_active = true",
        character_id as i32
    )
    .execute(&pool)
    .await?;
    
    // 2. Send disconnect to old client (if still connected)
    if let Some(connection) = get_active_connection(character_id) {
        send_server_close(&connection, "Logged in from another location").await?;
        connection.close().await?;
    }
    
    Ok(())
}
```

## Password Security

### Hashing on Registration

```rust
use bcrypt::{hash, DEFAULT_COST};

const BCRYPT_COST: u32 = 12; // Higher = more secure, slower

async fn hash_password(password: &str) -> Result<String> {
    // Run bcrypt in blocking thread (CPU-intensive)
    let password = password.to_string();
    tokio::task::spawn_blocking(move || {
        hash(&password, BCRYPT_COST)
    })
    .await?
    .map_err(|e| anyhow!("Password hashing failed: {}", e))
}
```

### Verification on Login

```rust
use bcrypt::verify;

async fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let password = password.to_string();
    let hash = hash.to_string();
    
    tokio::task::spawn_blocking(move || {
        verify(&password, &hash)
    })
    .await?
    .map_err(|e| anyhow!("Password verification failed: {}", e))
}
```

## Failed Login Tracking

### Tracking Attempts

```rust
use std::sync::Arc;
use dashmap::DashMap;

pub struct FailedLoginTracker {
    attempts: Arc<DashMap<IpAddr, Vec<Instant>>>,
}

impl FailedLoginTracker {
    pub fn record_attempt(&self, ip: IpAddr) {
        let mut entry = self.attempts.entry(ip).or_insert(Vec::new());
        entry.push(Instant::now());
        
        // Keep only last 10 minutes
        entry.retain(|&t| t.elapsed() < Duration::from_secs(600));
    }
    
    pub fn get_recent_attempts(&self, ip: &IpAddr) -> usize {
        self.attempts
            .get(ip)
            .map(|entry| {
                entry.iter()
                    .filter(|&&t| t.elapsed() < Duration::from_secs(60))
                    .count()
            })
            .unwrap_or(0)
    }
    
    pub fn should_rate_limit(&self, ip: &IpAddr) -> bool {
        self.get_recent_attempts(ip) > 5
    }
}
```

### Automatic Banning

```rust
async fn check_and_ban_brute_force(ip: IpAddr) -> Result<()> {
    let attempts = get_failed_login_attempts_last_hour(&ip).await?;
    
    if attempts > 20 {
        // Auto-ban for 1 hour
        ban_ip(
            &ip,
            "Automatic ban: too many failed login attempts",
            Some(Duration::from_secs(3600))
        ).await?;
        
        warn!("Auto-banned IP {} for brute force (20+ attempts)", ip);
    }
    
    Ok(())
}
```

## Client-Side Behavior

### Login Flow (Client Perspective)

```gml
// From login_controller Create Event

// 1. Connect to server
global.server = tcpconnect(global.server_ip, global.tcp_port, true);

// 2. Build and send login message
clearbuffer()
writeushort(MSG_LOGIN)
writestring(GameVersion)          // "0.106"
writestring(obj_inputfield_name.value)
writestring(obj_inputfield_pass.value)
writestring(getmacaddress())
send_message()

// 3. Wait for response in step event
```

### Response Handling (Client)

```gml
// From login_controller User Event 10

messageid = readushort()  // Should be 10 (MSG_LOGIN)
_case = readbyte()

switch(_case) {
    case 1: // Success
        get_account()  // Read all account data
        instance_create(x, y, obj_controller)
        global.firstLogin = true
        break;
    
    case 2: // Account doesn't exist
        obj_scrolltxt.value = "The given account does not exist"
        instance_destroy()
        break;
    
    case 3: // Wrong password
        obj_scrolltxt.value = "The given password did not match"
        instance_destroy()
        break;
    
    case 4: // Already logged in
        obj_scrolltxt.value = "Somebody is already logged in with that account"
        instance_destroy()
        break;
    
    case 5: // Version mismatch
        obj_scrolltxt.value = "Your Version is not compatible with the Server"
        instance_destroy()
        break;
    
    case 6: // Account banned
        obj_scrolltxt.value = "The given account is banned on this Server"
        instance_destroy()
        break;
    
    case 7: // IP banned (code 1)
    case 8: // IP banned (code 2)
        obj_scrolltxt.value = "You are banned by IP on this server"
        instance_destroy()
        break;
}
```

## Testing Authentication

### Unit Tests

```rust
#[tokio::test]
async fn test_register_success() {
    let db = setup_test_db().await;
    
    let req = RegisterRequest {
        msg_type: 7,
        version: "0.106".to_string(),
        username: "testuser".to_string(),
        password: "password123".to_string(),
        mac_address: "00:11:22:33:44:55".to_string(),
    };
    
    let result = handle_register(req, "127.0.0.1".parse().unwrap(), &db).await;
    assert!(result.is_ok());
    
    // Verify account created
    let account = get_account("testuser", &db).await.unwrap();
    assert_eq!(account.username, "testuser");
    
    // Verify password hashed
    assert!(bcrypt::verify("password123", &account.password_hash).unwrap());
}

#[tokio::test]
async fn test_login_wrong_password() {
    let db = setup_test_db().await;
    
    // Create account
    create_test_account("testuser", "password123", &db).await;
    
    // Try login with wrong password
    let req = LoginRequest {
        msg_type: 10,
        version: "0.106".to_string(),
        username: "testuser".to_string(),
        password: "wrongpassword".to_string(),
        mac_address: "00:11:22:33:44:55".to_string(),
    };
    
    let result = handle_login(req, "127.0.0.1".parse().unwrap(), &db).await;
    assert!(matches!(result, Err(ValidationError::WrongPassword)));
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_full_auth_flow() {
    let server = start_test_server().await;
    let mut client = TcpStream::connect("127.0.0.1:5555").await.unwrap();
    
    // Register
    let register_msg = build_register_message("testuser", "pass123", "00:11:22:33:44:55");
    send_encrypted(&mut client, register_msg).await.unwrap();
    
    let response = read_encrypted(&mut client).await.unwrap();
    let mut reader = MessageReader::new(&response);
    assert_eq!(reader.read_u16().unwrap(), MSG_REGISTER);
    assert_eq!(reader.read_u8().unwrap(), REGISTER_SUCCESS);
    
    // Disconnect
    drop(client);
    
    // Login
    let mut client = TcpStream::connect("127.0.0.1:5555").await.unwrap();
    let login_msg = build_login_message("testuser", "pass123", "00:11:22:33:44:55");
    send_encrypted(&mut client, login_msg).await.unwrap();
    
    let response = read_encrypted(&mut client).await.unwrap();
    let mut reader = MessageReader::new(&response);
    assert_eq!(reader.read_u16().unwrap(), MSG_LOGIN);
    assert_eq!(reader.read_u8().unwrap(), LOGIN_SUCCESS);
    
    // Should receive full account data
    let player_id = reader.read_u16().unwrap();
    assert!(player_id > 0);
}
```

---

**Next:** See `protocol/04-message-catalog.md` for other message types.
