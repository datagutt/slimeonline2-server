# Server-Side Validation

## Overview

Since the client cannot be modified and uses weak encryption, **ALL validation must be server-side**. The server must never trust client input and must validate every message thoroughly.

## Validation Layers

### Layer 1: Connection-Level Validation
- TCP connection limits per IP
- Rate limiting on connection attempts
- Ban list checking (IP, MAC, account)
- Message size limits

### Layer 2: Protocol-Level Validation
- Message type is valid (1-141, excluding reserved)
- Message structure is correct
- Required fields are present
- Data types match specification

### Layer 3: Game-Logic Validation
- Player state is valid for action
- Position/movement is physically possible
- Inventory transactions are valid
- Currency amounts don't overflow
- Permissions are sufficient

## Authentication Validation

### Registration

```rust
async fn validate_registration(
    req: &RegisterRequest,
    ip: &IpAddr,
    db: &PgPool,
) -> Result<(), ValidationError> {
    // 1. Version check
    if req.version != "0.106" {
        return Err(ValidationError::VersionMismatch);
    }
    
    // 2. Username validation
    if req.username.len() < 3 || req.username.len() > 20 {
        return Err(ValidationError::InvalidUsernameLength);
    }
    
    if !req.username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(ValidationError::InvalidUsernameChars);
    }
    
    // 3. Password validation
    if req.password.len() < 6 || req.password.len() > 50 {
        return Err(ValidationError::InvalidPasswordLength);
    }
    
    // 4. Check if username exists (case-insensitive)
    let exists = sqlx::query!(
        "SELECT 1 FROM accounts WHERE LOWER(username) = LOWER($1)",
        req.username
    )
    .fetch_optional(db)
    .await?
    .is_some();
    
    if exists {
        return Err(ValidationError::UsernameExists);
    }
    
    // 5. Check IP ban
    if is_ip_banned(ip, db).await? {
        return Err(ValidationError::IpBanned);
    }
    
    // 6. Check MAC ban
    if is_mac_banned(&req.mac_address, db).await? {
        return Err(ValidationError::MacBanned);
    }
    
    // 7. Rate limit: max 3 accounts per IP per day
    let count = sqlx::query!(
        "SELECT COUNT(*) FROM accounts 
         WHERE created_at > NOW() - INTERVAL '24 hours'
         AND last_login_ip = $1",
        ip.to_string()
    )
    .fetch_one(db)
    .await?
    .count
    .unwrap_or(0);
    
    if count >= 3 {
        return Err(ValidationError::TooManyAccountsPerIp);
    }
    
    Ok(())
}
```

### Login

```rust
async fn validate_login(
    req: &LoginRequest,
    ip: &IpAddr,
    db: &PgPool,
    active_sessions: &DashMap<u32, Session>,
) -> Result<Account, ValidationError> {
    // 1. Version check
    if req.version != "0.106" {
        return Err(ValidationError::VersionMismatch);
    }
    
    // 2. Check IP ban FIRST (before database query)
    if is_ip_banned(ip, db).await? {
        return Err(ValidationError::IpBanned);
    }
    
    // 3. Fetch account
    let account = sqlx::query_as!(
        Account,
        "SELECT * FROM accounts WHERE username = $1",
        req.username
    )
    .fetch_optional(db)
    .await?
    .ok_or(ValidationError::AccountNotFound)?;
    
    // 4. Check account ban
    if account.is_banned {
        return Err(ValidationError::AccountBanned);
    }
    
    // 5. Verify password (use bcrypt)
    if !bcrypt::verify(&req.password, &account.password_hash)? {
        // Log failed attempt
        log_failed_login(&account.id, ip).await;
        return Err(ValidationError::WrongPassword);
    }
    
    // 6. Check if already logged in
    if active_sessions.iter().any(|s| s.account_id == account.id) {
        // Kick old session
        kick_existing_session(account.id, active_sessions).await;
    }
    
    // 7. Check MAC ban
    if is_mac_banned(&req.mac_address, db).await? {
        return Err(ValidationError::MacBanned);
    }
    
    // 8. Rate limit: max 5 login attempts per IP per minute
    if exceeded_login_rate_limit(ip).await {
        return Err(ValidationError::TooManyAttempts);
    }
    
    Ok(account)
}
```

## Movement Validation

### Position Validation

```rust
async fn validate_movement(
    player: &Player,
    direction: u8,
    x: Option<u16>,
    y: Option<u16>,
    game_state: &GameState,
) -> Result<(), ValidationError> {
    // 1. Check if player can move
    if !player.can_move {
        return Err(ValidationError::CannotMove);
    }
    
    // 2. Validate direction code
    if direction == 0 || direction > 13 {
        return Err(ValidationError::InvalidDirection);
    }
    
    // 3. Validate position if provided
    if let (Some(new_x), Some(new_y)) = (x, y) {
        // Get room bounds
        let room = game_state.rooms.get(&player.room_id)
            .ok_or(ValidationError::RoomNotFound)?;
        
        // Check bounds
        if new_x > room.width || new_y > room.height {
            return Err(ValidationError::OutOfBounds);
        }
        
        // Check for teleporting (sudden position change)
        let distance = calculate_distance(
            (player.x, player.y),
            (new_x, new_y)
        );
        
        // Max movement per tick: ~2 pixels (at max speed)
        if distance > 50 {  // Allow some lag tolerance
            // Log potential teleport hack
            warn!(
                player_id = player.id,
                old_pos = ?(player.x, player.y),
                new_pos = ?(new_x, new_y),
                distance = distance,
                "Suspicious movement detected"
            );
            return Err(ValidationError::TeleportDetected);
        }
        
        // Check physics constraints
        match direction {
            DIR_JUMP => {
                // Can't jump if already in air
                if player.is_airborne {
                    return Err(ValidationError::InvalidPhysics);
                }
            }
            DIR_DUCK => {
                // Can only duck on ground
                if player.is_airborne {
                    return Err(ValidationError::InvalidPhysics);
                }
            }
            _ => {}
        }
    }
    
    // 4. Rate limit: max 60 movement updates/second
    if !check_movement_rate_limit(player.id).await {
        return Err(ValidationError::MovementTooFast);
    }
    
    Ok(())
}
```

## Item Validation

### Item Usage

```rust
async fn validate_item_use(
    player: &Player,
    slot: u8,
    game_state: &GameState,
) -> Result<Item, ValidationError> {
    // 1. Validate slot
    if slot < 1 || slot > 9 {
        return Err(ValidationError::InvalidSlot);
    }
    
    // 2. Get item from inventory
    let item_id = player.inventory.items[(slot - 1) as usize];
    
    if item_id == 0 {
        return Err(ValidationError::EmptySlot);
    }
    
    // 3. Get item definition
    let item = get_item_definition(item_id)
        .ok_or(ValidationError::InvalidItemId)?;
    
    // 4. Check if item is usable
    if !item.is_usable {
        return Err(ValidationError::ItemNotUsable);
    }
    
    // 5. Check item-specific requirements
    match item_id {
        ITEM_SIMPLE_SEED | ITEM_BLUE_SEED => {
            // Must be at a plant spot
            if !player.at_plant_spot {
                return Err(ValidationError::NoPlantSpot);
            }
        }
        ITEM_WEAK_CANNON_KIT => {
            // Must be at a build spot
            if !player.at_build_spot {
                return Err(ValidationError::NoBuildSpot);
            }
        }
        ITEM_WARP_WING => {
            // Must not be in combat/special state
            if player.in_cannon || !player.can_warp {
                return Err(ValidationError::CannotWarpNow);
            }
        }
        _ => {}
    }
    
    // 6. Cooldown check
    if let Some(last_use) = player.item_cooldowns.get(&item_id) {
        if last_use.elapsed() < item.cooldown {
            return Err(ValidationError::ItemOnCooldown);
        }
    }
    
    Ok(item)
}
```

### Shop Purchase

```rust
async fn validate_shop_purchase(
    player: &Player,
    shop_id: u16,
    category: u8,
    item_id: u16,
    db: &PgPool,
) -> Result<(Item, u16), ValidationError> {
    // 1. Get shop data
    let shop = get_shop_definition(shop_id)
        .ok_or(ValidationError::ShopNotFound)?;
    
    // 2. Check if player is at shop
    if player.room_id != shop.room_id {
        return Err(ValidationError::NotAtShop);
    }
    
    let shop_pos = (shop.x, shop.y);
    let player_pos = (player.x, player.y);
    let distance = calculate_distance(shop_pos, player_pos);
    
    if distance > 100 {  // Must be within 100 pixels
        return Err(ValidationError::TooFarFromShop);
    }
    
    // 3. Check if item is sold by this shop
    let shop_item = shop.items.iter()
        .find(|i| i.category == category && i.item_id == item_id)
        .ok_or(ValidationError::ItemNotSoldHere)?;
    
    // 4. Check stock
    if shop_item.stock > 0 {
        // Query current stock from database
        let current_stock = sqlx::query!(
            "SELECT stock FROM shop_stock 
             WHERE shop_id = $1 AND item_id = $2",
            shop_id as i32,
            item_id as i32
        )
        .fetch_one(db)
        .await?
        .stock as u16;
        
        if current_stock == 0 {
            return Err(ValidationError::OutOfStock);
        }
    }
    
    // 5. Check if player has enough points
    if player.points < shop_item.price {
        return Err(ValidationError::InsufficientFunds);
    }
    
    // 6. Check if inventory has space
    let has_space = match category {
        1 => player.inventory.has_outfit_space(),
        2 => player.inventory.has_item_space(),
        3 => player.inventory.has_accessory_space(),
        4 => player.inventory.has_tool_space(),
        _ => return Err(ValidationError::InvalidCategory),
    };
    
    if !has_space {
        return Err(ValidationError::InventoryFull);
    }
    
    // 7. Anti-spam: max 10 purchases per minute
    if !check_purchase_rate_limit(player.id).await {
        return Err(ValidationError::TooManyPurchases);
    }
    
    Ok((shop_item.item, shop_item.price))
}
```

## Currency Validation

### Point Transactions

```rust
async fn validate_points_transaction(
    player_id: u16,
    amount: i32,  // Can be negative for spending
    reason: &str,
    db: &PgPool,
) -> Result<u32, ValidationError> {
    // Get current points with row lock (FOR UPDATE)
    let current = sqlx::query!(
        "SELECT points FROM characters 
         WHERE id = $1 FOR UPDATE",
        player_id as i32
    )
    .fetch_one(db)
    .await?
    .points as i64;
    
    let new_balance = current + amount as i64;
    
    // Check for overflow
    if new_balance < 0 {
        return Err(ValidationError::InsufficientPoints);
    }
    
    if new_balance > MAX_POINTS as i64 {
        return Err(ValidationError::PointsOverflow);
    }
    
    // Log transaction
    sqlx::query!(
        "INSERT INTO point_transactions 
         (player_id, amount, balance_after, reason, created_at)
         VALUES ($1, $2, $3, $4, NOW())",
        player_id as i32,
        amount,
        new_balance as i32,
        reason
    )
    .execute(db)
    .await?;
    
    Ok(new_balance as u32)
}
```

## Clan Validation

### Clan Creation

```rust
async fn validate_clan_creation(
    player: &Player,
    clan_name: &str,
    db: &PgPool,
) -> Result<(), ValidationError> {
    // 1. Check if player already in clan
    if player.clan_id.is_some() {
        return Err(ValidationError::AlreadyInClan);
    }
    
    // 2. Validate clan name
    if clan_name.len() < 3 || clan_name.len() > 20 {
        return Err(ValidationError::InvalidClanNameLength);
    }
    
    if !clan_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == ' ') {
        return Err(ValidationError::InvalidClanNameChars);
    }
    
    // 3. Check if name exists (case-insensitive)
    let exists = sqlx::query!(
        "SELECT 1 FROM clans WHERE LOWER(name) = LOWER($1)",
        clan_name
    )
    .fetch_optional(db)
    .await?
    .is_some();
    
    if exists {
        return Err(ValidationError::ClanNameExists);
    }
    
    // 4. Check player has required items
    if !player.inventory.has_item(ITEM_PROOF_NATURE) {
        return Err(ValidationError::MissingProofNature);
    }
    
    if !player.inventory.has_item(ITEM_PROOF_EARTH) {
        return Err(ValidationError::MissingProofEarth);
    }
    
    // 5. Check player has enough points
    if player.points < CLAN_CREATION_COST {
        return Err(ValidationError::InsufficientFunds);
    }
    
    // 6. Rate limit: max 1 clan creation per player per month
    let recent = sqlx::query!(
        "SELECT 1 FROM clan_audit_log
         WHERE player_id = $1 
         AND action = 'create'
         AND created_at > NOW() - INTERVAL '30 days'",
        player.id as i32
    )
    .fetch_optional(db)
    .await?
    .is_some();
    
    if recent {
        return Err(ValidationError::ClanCreationCooldown);
    }
    
    Ok(())
}
```

## Chat Validation

```rust
async fn validate_chat_message(
    player: &Player,
    message: &str,
) -> Result<String, ValidationError> {
    // 1. Length check
    if message.is_empty() {
        return Err(ValidationError::EmptyMessage);
    }
    
    if message.len() > MAX_CHAT_LENGTH {
        return Err(ValidationError::MessageTooLong);
    }
    
    // 2. Rate limit: 1 message per 2 seconds
    if !check_chat_rate_limit(player.id).await {
        return Err(ValidationError::ChatTooFast);
    }
    
    // 3. Spam detection
    if is_spam(player.id, message).await {
        return Err(ValidationError::SpamDetected);
    }
    
    // 4. Profanity filter (optional, configurable)
    let filtered = filter_profanity(message);
    
    // 5. Check for command injection attempts
    if message.starts_with('/') && !player.is_moderator {
        return Err(ValidationError::UnauthorizedCommand);
    }
    
    Ok(filtered)
}
```

## Validation Error Handling

```rust
#[derive(Debug)]
pub enum ValidationError {
    // Auth
    VersionMismatch,
    InvalidUsernameLength,
    InvalidUsernameChars,
    UsernameExists,
    InvalidPasswordLength,
    AccountNotFound,
    WrongPassword,
    AccountBanned,
    IpBanned,
    MacBanned,
    TooManyAttempts,
    
    // Movement
    CannotMove,
    InvalidDirection,
    OutOfBounds,
    TeleportDetected,
    InvalidPhysics,
    MovementTooFast,
    
    // Items
    InvalidSlot,
    EmptySlot,
    InvalidItemId,
    ItemNotUsable,
    NoPlantSpot,
    NoBuildSpot,
    CannotWarpNow,
    ItemOnCooldown,
    
    // Shop
    ShopNotFound,
    NotAtShop,
    TooFarFromShop,
    ItemNotSoldHere,
    OutOfStock,
    InsufficientFunds,
    InvalidCategory,
    InventoryFull,
    TooManyPurchases,
    
    // Currency
    InsufficientPoints,
    PointsOverflow,
    
    // Clan
    AlreadyInClan,
    InvalidClanNameLength,
    InvalidClanNameChars,
    ClanNameExists,
    MissingProofNature,
    MissingProofEarth,
    ClanCreationCooldown,
    
    // Chat
    EmptyMessage,
    MessageTooLong,
    ChatTooFast,
    SpamDetected,
    UnauthorizedCommand,
    
    // General
    RoomNotFound,
    PlayerNotFound,
    DatabaseError(sqlx::Error),
}

impl ValidationError {
    pub fn to_client_message(&self) -> Option<Vec<u8>> {
        // Some errors should send response to client
        // Others should just disconnect
        match self {
            Self::VersionMismatch => {
                // Send MSG_LOGIN with case 5
                Some(build_login_response(LOGIN_VERSION_MISMATCH))
            }
            Self::WrongPassword => {
                Some(build_login_response(LOGIN_WRONG_PASSWORD))
            }
            // ... etc
            _ => None, // Just disconnect
        }
    }
    
    pub fn should_disconnect(&self) -> bool {
        matches!(self,
            Self::TeleportDetected |
            Self::VersionMismatch |
            Self::IpBanned |
            Self::MacBanned |
            Self::TooManyAttempts
        )
    }
    
    pub fn should_log_security_event(&self) -> bool {
        matches!(self,
            Self::TeleportDetected |
            Self::InvalidPhysics |
            Self::UnauthorizedCommand
        )
    }
}
```

---

**Critical:** Implement ALL these validations. Never trust client input. The client can be modified to send any data.
