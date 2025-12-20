//! Message structures for Slime Online 2 protocol
//!
//! This module defines typed message structures that can be parsed from
//! and serialized to the binary protocol format.

use super::{MessageReader, MessageWriter, MessageType};
use super::reader::ReadResult;
use crate::constants::{
    Direction, LOGIN_SUCCESS,
    PROTOCOL_VERSION, MIN_USERNAME_LENGTH, MAX_USERNAME_LENGTH,
    MIN_PASSWORD_LENGTH, MAX_PASSWORD_LENGTH,
};

// =============================================================================
// LOGIN / REGISTER MESSAGES
// =============================================================================

/// Client login request (MSG_LOGIN = 10)
#[derive(Debug, Clone)]
pub struct LoginRequest {
    pub version: String,
    pub username: String,
    pub password: String,
    pub mac_address: String,
}

impl LoginRequest {
    /// Parse a login request from the message buffer.
    /// Assumes the message type (u16) has already been read.
    pub fn parse(reader: &mut MessageReader) -> ReadResult<Self> {
        Ok(Self {
            version: reader.read_string()?,
            username: reader.read_string()?,
            password: reader.read_string()?,
            mac_address: reader.read_string()?,
        })
    }

    /// Validate the login request fields.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version != PROTOCOL_VERSION {
            return Err("Invalid client version");
        }
        if self.username.len() < MIN_USERNAME_LENGTH || self.username.len() > MAX_USERNAME_LENGTH {
            return Err("Invalid username length");
        }
        if self.password.is_empty() || self.password.len() > MAX_PASSWORD_LENGTH {
            return Err("Invalid password length");
        }
        // Validate username contains only allowed characters
        if !self.username.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err("Invalid characters in username");
        }
        Ok(())
    }
}

/// Successful login response data
#[derive(Debug, Clone)]
pub struct LoginSuccessData {
    pub player_id: u16,
    pub server_time: u32,
    pub motd: String,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub username: String,
    pub spawn_x: u16,
    pub spawn_y: u16,
    pub spawn_room: u16,
    pub body_id: u16,
    pub acs1_id: u16,
    pub acs2_id: u16,
    pub points: u32,
    pub has_signature: bool,
    pub quest_id: u16,
    pub quest_step: u8,
    pub trees_planted: u16,
    pub objects_built: u16,
    pub emotes: [u8; 5],
    pub outfits: [u16; 9],
    pub accessories: [u16; 9],
    pub items: [u16; 9],
    pub tools: [u8; 9],
}

impl LoginSuccessData {
    /// Serialize to binary format for sending to client.
    pub fn write(&self, writer: &mut MessageWriter) {
        writer
            .write_u16(MessageType::Login.id())
            .write_u8(LOGIN_SUCCESS)
            .write_u16(self.player_id)
            .write_u32(self.server_time)
            .write_string(&self.motd)
            .write_u8(self.day)
            .write_u8(self.hour)
            .write_u8(self.minute)
            .write_string(&self.username)
            .write_u16(self.spawn_x)
            .write_u16(self.spawn_y)
            .write_u16(self.spawn_room)
            .write_u16(self.body_id)
            .write_u16(self.acs1_id)
            .write_u16(self.acs2_id)
            .write_u32(self.points)
            .write_u8(if self.has_signature { 1 } else { 0 })
            .write_u16(self.quest_id)
            .write_u8(self.quest_step)
            .write_u16(self.trees_planted)
            .write_u16(self.objects_built);

        // Write emotes (5 slots)
        for emote in &self.emotes {
            writer.write_u8(*emote);
        }

        // Write outfits (9 slots)
        for outfit in &self.outfits {
            writer.write_u16(*outfit);
        }

        // Write accessories (9 slots)
        for accessory in &self.accessories {
            writer.write_u16(*accessory);
        }

        // Write items (9 slots)
        for item in &self.items {
            writer.write_u16(*item);
        }

        // Write tools (9 slots)
        for tool in &self.tools {
            writer.write_u8(*tool);
        }
    }
}

/// Write a login failure response.
pub fn write_login_failure(writer: &mut MessageWriter, error_code: u8) {
    writer.write_u16(MessageType::Login.id()).write_u8(error_code);
}

/// Client registration request (MSG_REGISTER = 7)
/// Note: Unlike login, registration does NOT include a version string.
#[derive(Debug, Clone)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub mac_address: String,
}

impl RegisterRequest {
    /// Parse a register request from the message buffer.
    /// Assumes the message type (u16) has already been read.
    /// Format: username + password + mac_address (no version string)
    pub fn parse(reader: &mut MessageReader) -> ReadResult<Self> {
        Ok(Self {
            username: reader.read_string()?,
            password: reader.read_string()?,
            mac_address: reader.read_string()?,
        })
    }

    /// Validate the registration request fields.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.username.len() < MIN_USERNAME_LENGTH || self.username.len() > MAX_USERNAME_LENGTH {
            return Err("Invalid username length");
        }
        if self.password.len() < MIN_PASSWORD_LENGTH || self.password.len() > MAX_PASSWORD_LENGTH {
            return Err("Invalid password length");
        }
        // Validate username contains only allowed characters
        if !self.username.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err("Invalid characters in username");
        }
        Ok(())
    }
}

/// Write a registration response.
pub fn write_register_response(writer: &mut MessageWriter, result_code: u8) {
    writer.write_u16(MessageType::Register.id()).write_u8(result_code);
}

// =============================================================================
// PLAYER MESSAGES
// =============================================================================

/// New player notification (MSG_NEW_PLAYER = 1)
#[derive(Debug, Clone)]
pub struct NewPlayerInfo {
    pub x: u16,
    pub y: u16,
    pub player_id: u16,
    pub room_id: u16,
    pub username: String,
    pub body_id: u16,
    pub acs1_id: u16,
    pub acs2_id: u16,
    // For case 2, additional movement state
    pub ileft: u8,
    pub iright: u8,
    pub iup: u8,
    pub idown: u8,
    pub iup_press: u8,
}

impl NewPlayerInfo {
    /// Write case 1 (new player joined server, needs response)
    pub fn write_case1(&self, writer: &mut MessageWriter) {
        writer
            .write_u16(MessageType::NewPlayer.id())
            .write_u8(1)
            .write_u16(self.x)
            .write_u16(self.y)
            .write_u16(self.player_id)
            .write_u16(self.room_id)
            .write_string(&self.username)
            .write_u16(self.body_id)
            .write_u16(self.acs1_id)
            .write_u16(self.acs2_id);
    }

    /// Write case 2 (existing player info, room change)
    pub fn write_case2(&self, writer: &mut MessageWriter) {
        writer
            .write_u16(MessageType::NewPlayer.id())
            .write_u8(2)
            .write_u16(self.x)
            .write_u16(self.y)
            .write_u16(self.player_id)
            .write_u8(self.ileft)
            .write_u8(self.iright)
            .write_u8(self.iup)
            .write_u8(self.idown)
            .write_u8(self.iup_press)
            .write_u16(self.room_id)
            .write_string(&self.username)
            .write_u16(self.body_id)
            .write_u16(self.acs1_id)
            .write_u16(self.acs2_id);
    }
}

/// Player left notification (MSG_LOGOUT = 6)
pub fn write_player_left(writer: &mut MessageWriter, player_id: u16) {
    writer.write_u16(MessageType::Logout.id()).write_u16(player_id);
}

// =============================================================================
// MOVEMENT MESSAGES
// =============================================================================

/// Movement update from client (MSG_MOVE_PLAYER = 2)
#[derive(Debug, Clone)]
pub struct MovementUpdate {
    pub direction: u8,
    pub x: Option<u16>,
    pub y: Option<u16>,
}

impl MovementUpdate {
    /// Parse a movement update from the message buffer.
    /// Assumes the message type (u16) has already been read.
    pub fn parse(reader: &mut MessageReader) -> ReadResult<Self> {
        let direction = reader.read_u8()?;
        
        // Determine if x/y coordinates are included based on direction
        let (x, y) = if let Some(dir) = Direction::from_u8(direction) {
            match dir {
                // Directions that include x and y
                Direction::StartLeftGround | Direction::StartRightGround |
                Direction::StopLeftGround | Direction::StopRightGround |
                Direction::Landing => {
                    (Some(reader.read_u16()?), Some(reader.read_u16()?))
                }
                // Directions that include only x
                Direction::Jump => {
                    (Some(reader.read_u16()?), None)
                }
                // Directions with no coordinates
                _ => (None, None),
            }
        } else {
            // Unknown direction, no coordinates
            (None, None)
        };

        Ok(Self { direction, x, y })
    }

    /// Write a movement broadcast to other players.
    pub fn write_broadcast(&self, writer: &mut MessageWriter, player_id: u16) {
        writer
            .write_u16(MessageType::MovePlayer.id())
            .write_u16(player_id)
            .write_u8(self.direction);
        
        if let Some(x) = self.x {
            writer.write_u16(x);
        }
        if let Some(y) = self.y {
            writer.write_u16(y);
        }
    }
}

// =============================================================================
// CHAT MESSAGES
// =============================================================================

/// Chat message from client (MSG_CHAT = 17)
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub message: String,
}

impl ChatMessage {
    /// Parse a chat message from the buffer.
    pub fn parse(reader: &mut MessageReader) -> ReadResult<Self> {
        Ok(Self {
            message: reader.read_string()?,
        })
    }

    /// Write a chat broadcast to room.
    pub fn write_broadcast(writer: &mut MessageWriter, player_id: u16, message: &str) {
        writer
            .write_u16(MessageType::Chat.id())
            .write_u16(player_id)
            .write_string(message);
    }
}

// =============================================================================
// UTILITY MESSAGES
// =============================================================================

/// Write a ping response (MSG_PING = 9)
pub fn write_ping(writer: &mut MessageWriter) {
    writer.write_u16(MessageType::Ping.id());
}

/// Write a server close message (MSG_SERVER_CLOSE = 24)
#[allow(dead_code)]
pub fn write_server_close(writer: &mut MessageWriter) {
    writer.write_u16(MessageType::ServerClose.id());
}

/// Write a player stop message (MSG_PLAYER_STOP = 43)
#[allow(dead_code)]
pub fn write_player_stop(writer: &mut MessageWriter) {
    writer.write_u16(MessageType::PlayerStop.id());
}

/// Write a can-move message (MSG_CANMOVE_TRUE = 42)
#[allow(dead_code)]
pub fn write_canmove_true(writer: &mut MessageWriter) {
    writer.write_u16(MessageType::CanMoveTrue.id());
}

// =============================================================================
// WARP MESSAGES
// =============================================================================

/// Write a warp message (MSG_WARP = 14)
#[allow(dead_code)]
pub fn write_warp(writer: &mut MessageWriter, x: u16, y: u16, room_id: u16) {
    writer
        .write_u16(MessageType::Warp.id())
        .write_u16(x)
        .write_u16(y)
        .write_u16(room_id);
}

// =============================================================================
// MESSAGE TYPE HELPER
// =============================================================================

/// Get the message type from raw bytes (without consuming).
#[allow(dead_code)]
pub fn peek_message_type(data: &[u8]) -> Option<u16> {
    if data.len() < 2 {
        return None;
    }
    Some(u16::from_le_bytes([data[0], data[1]]))
}

/// Check if a message type is valid.
#[allow(dead_code)]
pub fn is_valid_message_type(msg_type: u16) -> bool {
    // Valid message types are 1-141, excluding reserved ones
    matches!(msg_type, 1..=141) && !matches!(msg_type, 3 | 4 | 8 | 20)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_request_parse() {
        let mut data = Vec::new();
        data.extend_from_slice(b"0.106\x00");
        data.extend_from_slice(b"TestUser\x00");
        data.extend_from_slice(b"password123\x00");
        data.extend_from_slice(b"00-11-22-33-44-55\x00");

        let mut reader = MessageReader::new(&data);
        let login = LoginRequest::parse(&mut reader).unwrap();

        assert_eq!(login.version, "0.106");
        assert_eq!(login.username, "TestUser");
        assert_eq!(login.password, "password123");
        assert_eq!(login.mac_address, "00-11-22-33-44-55");
    }

    #[test]
    fn test_login_request_validate() {
        let valid = LoginRequest {
            version: "0.106".to_string(),
            username: "Player123".to_string(),
            password: "secret123".to_string(),
            mac_address: "00-11-22-33-44-55".to_string(),
        };
        assert!(valid.validate().is_ok());

        let invalid_version = LoginRequest {
            version: "1.0".to_string(),
            ..valid.clone()
        };
        assert!(invalid_version.validate().is_err());

        let short_username = LoginRequest {
            username: "ab".to_string(),
            ..valid.clone()
        };
        assert!(short_username.validate().is_err());
    }

    #[test]
    fn test_login_success_write() {
        let data = LoginSuccessData {
            player_id: 42,
            server_time: 1234567890,
            motd: "Welcome!".to_string(),
            day: 1,
            hour: 12,
            minute: 30,
            username: "Player1".to_string(),
            spawn_x: 160,
            spawn_y: 120,
            spawn_room: 1,
            body_id: 1,
            acs1_id: 0,
            acs2_id: 0,
            points: 1000,
            has_signature: false,
            quest_id: 0,
            quest_step: 0,
            trees_planted: 0,
            objects_built: 0,
            emotes: [0; 5],
            outfits: [0; 9],
            accessories: [0; 9],
            items: [0; 9],
            tools: [0; 9],
        };

        let mut writer = MessageWriter::new();
        data.write(&mut writer);

        let bytes = writer.into_bytes();
        
        // Check message type (Login = 10)
        assert_eq!(bytes[0..2], [10, 0]);
        // Check success case
        assert_eq!(bytes[2], 1);
        // Check player ID
        assert_eq!(bytes[3..5], [42, 0]);
    }

    #[test]
    fn test_peek_message_type() {
        let data = [10u8, 0, 1, 2, 3];
        assert_eq!(peek_message_type(&data), Some(10));

        let short = [10u8];
        assert_eq!(peek_message_type(&short), None);
    }
}
