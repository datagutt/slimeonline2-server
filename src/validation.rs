//! Input validation module for server-side validation
//!
//! Validates client inputs to prevent:
//! - Out-of-bounds values
//! - Invalid game state manipulation
//! - Exploits (speed hacks, item duplication, etc.)
//!
//! Note: SQL injection is NOT a concern here because SQLx uses parameterized queries.

use crate::constants::*;

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
// String Validators
// =============================================================================

/// Validate username format
pub fn validate_username(username: &str) -> ValidationResult<()> {
    if username.len() < MIN_USERNAME_LENGTH {
        return Err(ValidationError::new(
            "username",
            format!("Username too short (min {} chars)", MIN_USERNAME_LENGTH),
            Severity::Low,
        ));
    }

    if username.len() > MAX_USERNAME_LENGTH {
        return Err(ValidationError::new(
            "username",
            format!("Username too long (max {} chars)", MAX_USERNAME_LENGTH),
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

/// Validate password format
pub fn validate_password(password: &str) -> ValidationResult<()> {
    if password.len() < MIN_PASSWORD_LENGTH {
        return Err(ValidationError::new(
            "password",
            format!("Password too short (min {} chars)", MIN_PASSWORD_LENGTH),
            Severity::Low,
        ));
    }

    if password.len() > MAX_PASSWORD_LENGTH {
        return Err(ValidationError::new(
            "password",
            format!("Password too long (max {} chars)", MAX_PASSWORD_LENGTH),
            Severity::Medium,
        ));
    }

    Ok(())
}

/// Validate chat message
pub fn validate_chat_message(message: &str) -> ValidationResult<&str> {
    if message.is_empty() {
        return Err(ValidationError::new(
            "message",
            "Empty chat message",
            Severity::Low,
        ));
    }

    if message.len() > MAX_CHAT_LENGTH {
        return Err(ValidationError::new(
            "message",
            "Chat message too long",
            Severity::Medium,
        ));
    }

    // Strip control characters except newline
    // Note: We return the original message but validation passes if OK
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

/// Validate clan name
pub fn validate_clan_name(name: &str) -> ValidationResult<()> {
    if name.len() < MIN_CLAN_NAME_LENGTH {
        return Err(ValidationError::new(
            "clan_name",
            format!("Clan name too short (min {} chars)", MIN_CLAN_NAME_LENGTH),
            Severity::Low,
        ));
    }

    if name.len() > MAX_CLAN_NAME_LENGTH {
        return Err(ValidationError::new(
            "clan_name",
            format!("Clan name too long (max {} chars)", MAX_CLAN_NAME_LENGTH),
            Severity::Medium,
        ));
    }

    Ok(())
}

// =============================================================================
// Numeric Validators
// =============================================================================

/// Validate position coordinates
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
pub fn validate_room_id(room_id: u16) -> ValidationResult<u16> {
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
pub fn validate_item_slot(slot: u8) -> ValidationResult<u8> {
    if slot < 1 || slot > ITEM_SLOTS as u8 {
        return Err(ValidationError::new(
            "slot",
            format!("Invalid item slot: {} (must be 1-{})", slot, ITEM_SLOTS),
            Severity::Medium,
        ));
    }
    Ok(slot)
}

/// Validate outfit slot (1-based)
pub fn validate_outfit_slot(slot: u8) -> ValidationResult<u8> {
    if slot < 1 || slot > OUTFIT_SLOTS as u8 {
        return Err(ValidationError::new(
            "slot",
            format!("Invalid outfit slot: {} (must be 1-{})", slot, OUTFIT_SLOTS),
            Severity::Medium,
        ));
    }
    Ok(slot)
}

/// Validate accessory slot (1-based)
pub fn validate_accessory_slot(slot: u8) -> ValidationResult<u8> {
    if slot < 1 || slot > ACCESSORY_SLOTS as u8 {
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
pub fn validate_tool_slot(slot: u8) -> ValidationResult<u8> {
    if slot < 1 || slot > TOOL_SLOTS as u8 {
        return Err(ValidationError::new(
            "slot",
            format!("Invalid tool slot: {} (must be 1-{})", slot, TOOL_SLOTS),
            Severity::Medium,
        ));
    }
    Ok(slot)
}

/// Validate emote slot (0-based in array, but we validate count)
pub fn validate_emote_slot(slot: u8) -> ValidationResult<u8> {
    if slot >= EMOTE_SLOTS as u8 {
        return Err(ValidationError::new(
            "slot",
            format!(
                "Invalid emote slot: {} (must be 0-{})",
                slot,
                EMOTE_SLOTS - 1
            ),
            Severity::Medium,
        ));
    }
    Ok(slot)
}

/// Validate point amounts
pub fn validate_points(points: u32) -> ValidationResult<u32> {
    if points > MAX_POINTS {
        return Err(ValidationError::new(
            "points",
            format!("Points exceed maximum: {} > {}", points, MAX_POINTS),
            Severity::High,
        ));
    }
    Ok(points)
}

/// Validate bank transfer/deposit/withdraw amount
pub fn validate_bank_amount(amount: u32, current_balance: u32) -> ValidationResult<u32> {
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
            Severity::Medium, // Could be exploit attempt
        ));
    }

    if amount > MAX_BANK_BALANCE {
        return Err(ValidationError::new(
            "amount",
            "Amount exceeds maximum bank balance",
            Severity::High,
        ));
    }

    Ok(amount)
}

/// Validate item ID (based on db_items.gml from client)
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

/// Validate mail content
pub fn validate_mail(subject: &str, body: &str) -> ValidationResult<()> {
    if subject.is_empty() {
        return Err(ValidationError::new(
            "subject",
            "Mail subject cannot be empty",
            Severity::Low,
        ));
    }

    if subject.len() > MAX_MAIL_SUBJECT {
        return Err(ValidationError::new(
            "subject",
            "Mail subject too long",
            Severity::Medium,
        ));
    }

    if body.len() > MAX_MAIL_BODY {
        return Err(ValidationError::new(
            "body",
            "Mail body too long",
            Severity::Medium,
        ));
    }

    Ok(())
}

/// Validate BBS post
pub fn validate_bbs_post(title: &str, content: &str) -> ValidationResult<()> {
    if title.is_empty() {
        return Err(ValidationError::new(
            "title",
            "BBS title cannot be empty",
            Severity::Low,
        ));
    }

    if title.len() > MAX_BBS_TITLE {
        return Err(ValidationError::new(
            "title",
            "BBS title too long",
            Severity::Medium,
        ));
    }

    if content.len() > MAX_BBS_CONTENT {
        return Err(ValidationError::new(
            "content",
            "BBS content too long",
            Severity::Medium,
        ));
    }

    Ok(())
}

/// Validate MAC address format
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
pub fn sanitize_username(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(MAX_USERNAME_LENGTH)
        .collect()
}

/// Sanitize chat message
pub fn sanitize_chat(input: &str) -> String {
    sanitize_string(input, MAX_CHAT_LENGTH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_username_validation() {
        assert!(validate_username("validuser").is_ok());
        assert!(validate_username("user_123").is_ok());
        assert!(validate_username("user-name").is_ok());
        assert!(validate_username("ab").is_err()); // too short
        assert!(validate_username(&"a".repeat(50)).is_err()); // too long
        assert!(validate_username("user name").is_err()); // spaces not allowed
        assert!(validate_username("user@name").is_err()); // special chars not allowed
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
        assert!(validate_bank_amount(100, 1000).is_ok());
        assert!(validate_bank_amount(0, 1000).is_err()); // zero not allowed
        assert!(validate_bank_amount(2000, 1000).is_err()); // exceeds balance
    }
}
