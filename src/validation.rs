//! Input validation module for server-side validation
//!
//! Validates client inputs to prevent:
//! - Out-of-bounds values
//! - Invalid game state manipulation
//! - Exploits (speed hacks, item duplication, etc.)
//!
//! Note: SQL injection is NOT a concern here because SQLx uses parameterized queries.
//!
//! ## Usage
//!
//! Validation functions take config-derived limits as parameters. Create a `Validator`
//! from `GameConfig` for convenient access to all validation methods with proper limits.

use crate::config::{ClanLimitsConfig, GameConfig, LimitsConfig};

/// Validation result with detailed error message
#[derive(Debug)]
pub struct ValidationError {
    pub field: &'static str,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    /// Normal invalid input (typo, mistake)
    Low,
    /// Suspicious input (possible exploit attempt)
    Medium,
    /// Definite exploit attempt
    High,
}

impl ValidationError {
    pub fn new(field: &'static str, message: impl Into<String>, severity: Severity) -> Self {
        Self {
            field,
            message: message.into(),
            severity,
        }
    }
}

pub type ValidationResult<T> = Result<T, ValidationError>;

// =============================================================================
// Validator - wraps config and provides validation methods
// =============================================================================

/// Validator that uses game config for limits
///
/// Create from `GameConfig` and use for all validation needs:
/// ```ignore
/// let validator = Validator::new(&game_config);
/// validator.username("player123")?;
/// validator.chat_message("hello world")?;
/// ```
#[derive(Debug, Clone)]
pub struct Validator {
    pub limits: LimitsConfig,
    pub clan_limits: ClanLimitsConfig,
}

impl Validator {
    /// Create a new validator from game config
    pub fn new(config: &GameConfig) -> Self {
        Self {
            limits: config.game.limits.clone(),
            clan_limits: config.clans.limits.clone(),
        }
    }

    /// Validate username format
    pub fn username(&self, username: &str) -> ValidationResult<()> {
        validate_username(
            username,
            self.limits.min_username_length,
            self.limits.max_username_length,
        )
    }

    /// Validate password format
    pub fn password(&self, password: &str) -> ValidationResult<()> {
        validate_password(
            password,
            self.limits.min_password_length,
            self.limits.max_password_length,
        )
    }

    /// Validate chat message
    pub fn chat_message<'a>(&self, message: &'a str) -> ValidationResult<&'a str> {
        validate_chat_message(message, self.limits.max_chat_length)
    }

    /// Validate clan name
    pub fn clan_name(&self, name: &str) -> ValidationResult<()> {
        validate_clan_name(
            name,
            self.clan_limits.min_name_length,
            self.clan_limits.max_name_length,
        )
    }

    /// Validate point amounts
    pub fn points(&self, points: u32) -> ValidationResult<u32> {
        validate_points(points, self.limits.max_points)
    }

    /// Validate bank amount
    pub fn bank_amount(&self, amount: u32, current_balance: u32) -> ValidationResult<u32> {
        validate_bank_amount(amount, current_balance, self.limits.max_bank_balance)
    }

    /// Sanitize username - keep only safe characters
    pub fn sanitize_username(&self, input: &str) -> String {
        sanitize_username(input, self.limits.max_username_length)
    }

    /// Sanitize chat message
    pub fn sanitize_chat(&self, input: &str) -> String {
        sanitize_string(input, self.limits.max_chat_length)
    }
}

// =============================================================================
// String Validators (parameterized)
// =============================================================================

/// Validate username format with configurable limits
pub fn validate_username(
    username: &str,
    min_length: usize,
    max_length: usize,
) -> ValidationResult<()> {
    if username.len() < min_length {
        return Err(ValidationError::new(
            "username",
            format!("Username too short (min {} chars)", min_length),
            Severity::Low,
        ));
    }

    if username.len() > max_length {
        return Err(ValidationError::new(
            "username",
            format!("Username too long (max {} chars)", max_length),
            Severity::Medium,
        ));
    }

    // Only allow alphanumeric, underscore, and dash
    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(ValidationError::new(
            "username",
            "Username contains invalid characters",
            Severity::Medium,
        ));
    }

    Ok(())
}

/// Validate password format with configurable limits
pub fn validate_password(
    password: &str,
    min_length: usize,
    max_length: usize,
) -> ValidationResult<()> {
    if password.len() < min_length {
        return Err(ValidationError::new(
            "password",
            format!("Password too short (min {} chars)", min_length),
            Severity::Low,
        ));
    }

    if password.len() > max_length {
        return Err(ValidationError::new(
            "password",
            format!("Password too long (max {} chars)", max_length),
            Severity::Medium,
        ));
    }

    Ok(())
}

/// Validate chat message with configurable max length
pub fn validate_chat_message(message: &str, max_length: usize) -> ValidationResult<&str> {
    if message.is_empty() {
        return Err(ValidationError::new(
            "message",
            "Empty chat message",
            Severity::Low,
        ));
    }

    if message.len() > max_length {
        return Err(ValidationError::new(
            "message",
            "Chat message too long",
            Severity::Medium,
        ));
    }

    // Strip control characters except newline
    if message
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\r')
    {
        return Err(ValidationError::new(
            "message",
            "Message contains control characters",
            Severity::Medium,
        ));
    }

    Ok(message)
}

/// Validate clan name with configurable limits
pub fn validate_clan_name(
    name: &str,
    min_length: usize,
    max_length: usize,
) -> ValidationResult<()> {
    if name.len() < min_length {
        return Err(ValidationError::new(
            "clan_name",
            format!("Clan name too short (min {} chars)", min_length),
            Severity::Low,
        ));
    }

    if name.len() > max_length {
        return Err(ValidationError::new(
            "clan_name",
            format!("Clan name too long (max {} chars)", max_length),
            Severity::Medium,
        ));
    }

    Ok(())
}

// =============================================================================
// Numeric Validators (parameterized)
// =============================================================================

/// Validate position coordinates
/// Uses hardcoded limits as these are engine constraints, not configurable
pub fn validate_position(x: u16, y: u16) -> ValidationResult<(u16, u16)> {
    // Room dimensions vary, but a reasonable maximum is ~10000 pixels
    // Based on client code: coordinates are u16 (0-65535)
    // But typical room is ~2000x1000 max
    const MAX_REASONABLE_X: u16 = 10000;
    const MAX_REASONABLE_Y: u16 = 5000;

    if x > MAX_REASONABLE_X {
        return Err(ValidationError::new(
            "x",
            format!("X coordinate out of bounds: {}", x),
            Severity::High,
        ));
    }

    if y > MAX_REASONABLE_Y {
        return Err(ValidationError::new(
            "y",
            format!("Y coordinate out of bounds: {}", y),
            Severity::High,
        ));
    }

    Ok((x, y))
}

/// Validate room ID
/// Uses hardcoded limit as this is an engine constraint
pub fn validate_room_id(room_id: u16) -> ValidationResult<u16> {
    const MAX_ROOM_ID: u16 = 1000;

    if room_id > MAX_ROOM_ID {
        return Err(ValidationError::new(
            "room_id",
            format!("Invalid room ID: {}", room_id),
            Severity::High,
        ));
    }

    Ok(room_id)
}

/// Validate inventory slot (1-based, items are slots 1-9)
/// Uses hardcoded limit as inventory size is engine constraint
pub fn validate_item_slot(slot: u8) -> ValidationResult<u8> {
    const ITEM_SLOTS: u8 = 9;

    if slot < 1 || slot > ITEM_SLOTS {
        return Err(ValidationError::new(
            "slot",
            format!("Invalid item slot: {} (must be 1-{})", slot, ITEM_SLOTS),
            Severity::Medium,
        ));
    }
    Ok(slot)
}

/// Validate outfit slot (1-based)
/// Uses hardcoded limit as inventory size is engine constraint
pub fn validate_outfit_slot(slot: u8) -> ValidationResult<u8> {
    const OUTFIT_SLOTS: u8 = 9;

    if slot < 1 || slot > OUTFIT_SLOTS {
        return Err(ValidationError::new(
            "slot",
            format!("Invalid outfit slot: {} (must be 1-{})", slot, OUTFIT_SLOTS),
            Severity::Medium,
        ));
    }
    Ok(slot)
}

/// Validate accessory slot (1-based)
/// Uses hardcoded limit as inventory size is engine constraint
pub fn validate_accessory_slot(slot: u8) -> ValidationResult<u8> {
    const ACCESSORY_SLOTS: u8 = 9;

    if slot < 1 || slot > ACCESSORY_SLOTS {
        return Err(ValidationError::new(
            "slot",
            format!(
                "Invalid accessory slot: {} (must be 1-{})",
                slot, ACCESSORY_SLOTS
            ),
            Severity::Medium,
        ));
    }
    Ok(slot)
}

/// Validate tool slot (1-based)
/// Uses hardcoded limit as inventory size is engine constraint
pub fn validate_tool_slot(slot: u8) -> ValidationResult<u8> {
    const TOOL_SLOTS: u8 = 9;

    if slot < 1 || slot > TOOL_SLOTS {
        return Err(ValidationError::new(
            "slot",
            format!("Invalid tool slot: {} (must be 1-{})", slot, TOOL_SLOTS),
            Severity::Medium,
        ));
    }
    Ok(slot)
}

/// Validate emote slot (0-based in array)
/// Uses hardcoded limit as emote slots are engine constraint
pub fn validate_emote_slot(slot: u8) -> ValidationResult<u8> {
    const EMOTE_SLOTS: u8 = 5;

    if slot >= EMOTE_SLOTS {
        return Err(ValidationError::new(
            "slot",
            format!("Invalid emote slot: {} (must be 0-{})", slot, EMOTE_SLOTS - 1),
            Severity::Medium,
        ));
    }
    Ok(slot)
}

/// Validate point amounts with configurable max
pub fn validate_points(points: u32, max_points: u32) -> ValidationResult<u32> {
    if points > max_points {
        return Err(ValidationError::new(
            "points",
            format!("Points exceed maximum: {} > {}", points, max_points),
            Severity::High,
        ));
    }
    Ok(points)
}

/// Validate bank transfer/deposit/withdraw amount with configurable max
pub fn validate_bank_amount(
    amount: u32,
    current_balance: u32,
    max_bank_balance: u32,
) -> ValidationResult<u32> {
    if amount == 0 {
        return Err(ValidationError::new(
            "amount",
            "Amount must be greater than 0",
            Severity::Low,
        ));
    }

    if amount > current_balance {
        return Err(ValidationError::new(
            "amount",
            format!("Insufficient balance: {} > {}", amount, current_balance),
            Severity::Medium,
        ));
    }

    if amount > max_bank_balance {
        return Err(ValidationError::new(
            "amount",
            "Amount exceeds maximum bank balance",
            Severity::High,
        ));
    }

    Ok(amount)
}

/// Validate item ID (based on db_items.gml from client)
/// Uses hardcoded limit as this is determined by client data
pub fn validate_item_id(item_id: u16) -> ValidationResult<u16> {
    // Items 1-61 are defined in the client
    // 0 = empty slot
    const MAX_ITEM_ID: u16 = 61;

    if item_id > MAX_ITEM_ID {
        return Err(ValidationError::new(
            "item_id",
            format!("Invalid item ID: {}", item_id),
            Severity::High,
        ));
    }

    Ok(item_id)
}

/// Validate direction byte for movement
/// Uses hardcoded limit as this is determined by client protocol
pub fn validate_direction(direction: u8) -> ValidationResult<u8> {
    // Based on case_msg_move_player.gml:
    // 1-13 are valid direction codes
    if !(1..=13).contains(&direction) {
        return Err(ValidationError::new(
            "direction",
            format!("Invalid direction: {}", direction),
            Severity::Medium,
        ));
    }

    Ok(direction)
}

// =============================================================================
// Complex Validators
// =============================================================================

/// Validate mail content with configurable limits
pub fn validate_mail(
    subject: &str,
    body: &str,
    max_subject: usize,
    max_body: usize,
) -> ValidationResult<()> {
    if subject.is_empty() {
        return Err(ValidationError::new(
            "subject",
            "Mail subject cannot be empty",
            Severity::Low,
        ));
    }

    if subject.len() > max_subject {
        return Err(ValidationError::new(
            "subject",
            "Mail subject too long",
            Severity::Medium,
        ));
    }

    if body.len() > max_body {
        return Err(ValidationError::new(
            "body",
            "Mail body too long",
            Severity::Medium,
        ));
    }

    Ok(())
}

/// Validate BBS post with configurable limits
pub fn validate_bbs_post(
    title: &str,
    content: &str,
    max_title: usize,
    max_content: usize,
) -> ValidationResult<()> {
    if title.is_empty() {
        return Err(ValidationError::new(
            "title",
            "BBS title cannot be empty",
            Severity::Low,
        ));
    }

    if title.len() > max_title {
        return Err(ValidationError::new(
            "title",
            "BBS title too long",
            Severity::Medium,
        ));
    }

    if content.len() > max_content {
        return Err(ValidationError::new(
            "content",
            "BBS content too long",
            Severity::Medium,
        ));
    }

    Ok(())
}

/// Validate MAC address format
/// Uses no config as MAC format is universal
pub fn validate_mac_address(mac: &str) -> ValidationResult<()> {
    // MAC address should be 12 hex characters (without separators) or 17 with separators
    if mac.is_empty() {
        return Err(ValidationError::new(
            "mac_address",
            "MAC address is empty",
            Severity::Low,
        ));
    }

    // Allow formats: AABBCCDDEEFF or AA:BB:CC:DD:EE:FF or AA-BB-CC-DD-EE-FF
    let clean: String = mac.chars().filter(|c| c.is_ascii_hexdigit()).collect();

    if clean.len() != 12 {
        return Err(ValidationError::new(
            "mac_address",
            "Invalid MAC address format",
            Severity::Medium,
        ));
    }

    Ok(())
}

// =============================================================================
// Sanitizers
// =============================================================================

/// Sanitize a string by removing dangerous characters
pub fn sanitize_string(input: &str, max_len: usize) -> String {
    input
        .chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .take(max_len)
        .collect()
}

/// Sanitize username - keep only safe characters
pub fn sanitize_username(input: &str, max_len: usize) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(max_len)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_username_validation() {
        // Using typical config values
        let min = 3;
        let max = 20;

        assert!(validate_username("validuser", min, max).is_ok());
        assert!(validate_username("user_123", min, max).is_ok());
        assert!(validate_username("user-name", min, max).is_ok());
        assert!(validate_username("ab", min, max).is_err()); // too short
        assert!(validate_username(&"a".repeat(50), min, max).is_err()); // too long
        assert!(validate_username("user name", min, max).is_err()); // spaces not allowed
        assert!(validate_username("user@name", min, max).is_err()); // special chars not allowed
    }

    #[test]
    fn test_position_validation() {
        assert!(validate_position(100, 100).is_ok());
        assert!(validate_position(2000, 500).is_ok());
        assert!(validate_position(50000, 100).is_err()); // x too large
        assert!(validate_position(100, 10000).is_err()); // y too large
    }

    #[test]
    fn test_item_slot_validation() {
        assert!(validate_item_slot(1).is_ok());
        assert!(validate_item_slot(9).is_ok());
        assert!(validate_item_slot(0).is_err());
        assert!(validate_item_slot(10).is_err());
    }

    #[test]
    fn test_bank_amount_validation() {
        let max_bank = 100_000_000;
        assert!(validate_bank_amount(100, 1000, max_bank).is_ok());
        assert!(validate_bank_amount(0, 1000, max_bank).is_err()); // zero not allowed
        assert!(validate_bank_amount(2000, 1000, max_bank).is_err()); // exceeds balance
    }
}
