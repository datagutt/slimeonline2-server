//! Anti-cheat type definitions
//!
//! Defines hack types tracked by the system and response actions.

use std::fmt;

/// Types of hacks tracked by the anti-cheat system
/// Based on GML hack_alert.gml and various case handlers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HackType {
    /// Attempted to spend more points than available (bank, shop)
    InsufficientFunds,
    /// Invalid inventory slot number
    InvalidSlot,
    /// Tried to use item from empty slot
    ItemNotInSlot,
    /// Tried to begin already-cleared quest
    QuestAlreadyCleared,
    /// Tried to clear quest not in progress
    QuestNotActive,
    /// Invalid building placement spot
    InvalidBuildSpot,
    /// Building spot already occupied
    SpotNotFree,
    /// Tried to buy out-of-stock item (should be blocked client-side)
    ShopOutOfStock,
    /// Missing required items/points for clan creation
    ClanMissingRequirements,
    /// Invalid race checkpoint sequence
    RaceSequenceViolation,
    /// Teleported impossibly far (movement hack)
    PositionTeleport,
    /// Moving faster than possible (speed hack)
    SpeedHack,
    /// Invalid item ID or manipulation attempt
    InvalidItem,
    /// Sending malformed or invalid messages
    ProtocolViolation,
    /// Generic suspicious activity
    SuspiciousActivity,
}

impl fmt::Display for HackType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientFunds => write!(f, "InsufficientFunds"),
            Self::InvalidSlot => write!(f, "InvalidSlot"),
            Self::ItemNotInSlot => write!(f, "ItemNotInSlot"),
            Self::QuestAlreadyCleared => write!(f, "QuestAlreadyCleared"),
            Self::QuestNotActive => write!(f, "QuestNotActive"),
            Self::InvalidBuildSpot => write!(f, "InvalidBuildSpot"),
            Self::SpotNotFree => write!(f, "SpotNotFree"),
            Self::ShopOutOfStock => write!(f, "ShopOutOfStock"),
            Self::ClanMissingRequirements => write!(f, "ClanMissingRequirements"),
            Self::RaceSequenceViolation => write!(f, "RaceSequenceViolation"),
            Self::PositionTeleport => write!(f, "PositionTeleport"),
            Self::SpeedHack => write!(f, "SpeedHack"),
            Self::InvalidItem => write!(f, "InvalidItem"),
            Self::ProtocolViolation => write!(f, "ProtocolViolation"),
            Self::SuspiciousActivity => write!(f, "SuspiciousActivity"),
        }
    }
}

impl HackType {
    /// Get a human-readable description of the hack type
    pub fn description(&self) -> &'static str {
        match self {
            Self::InsufficientFunds => "Attempted transaction with insufficient funds",
            Self::InvalidSlot => "Referenced invalid inventory slot",
            Self::ItemNotInSlot => "Attempted to use item from empty slot",
            Self::QuestAlreadyCleared => "Attempted to start already-completed quest",
            Self::QuestNotActive => "Attempted to complete inactive quest",
            Self::InvalidBuildSpot => "Attempted to build at invalid location",
            Self::SpotNotFree => "Attempted to build on occupied spot",
            Self::ShopOutOfStock => "Attempted to buy out-of-stock item",
            Self::ClanMissingRequirements => "Attempted clan creation without requirements",
            Self::RaceSequenceViolation => "Invalid race checkpoint sequence",
            Self::PositionTeleport => "Impossible position change detected",
            Self::SpeedHack => "Movement speed exceeds maximum",
            Self::InvalidItem => "Invalid or manipulated item data",
            Self::ProtocolViolation => "Malformed or invalid message",
            Self::SuspiciousActivity => "Generic suspicious behavior",
        }
    }

    /// Get the severity level (1-5, higher = more severe)
    pub fn severity(&self) -> u8 {
        match self {
            // Low severity - could be client bugs or lag
            Self::InvalidSlot | Self::ItemNotInSlot => 1,
            // Medium severity - likely manipulation
            Self::InsufficientFunds
            | Self::QuestAlreadyCleared
            | Self::QuestNotActive
            | Self::ShopOutOfStock => 2,
            // Higher severity - definite cheating
            Self::InvalidBuildSpot
            | Self::SpotNotFree
            | Self::ClanMissingRequirements
            | Self::RaceSequenceViolation => 3,
            // High severity - active hacking
            Self::PositionTeleport | Self::SpeedHack | Self::InvalidItem => 4,
            // Critical - protocol manipulation
            Self::ProtocolViolation | Self::SuspiciousActivity => 5,
        }
    }
}

/// Response action to take after a hack is detected
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HackResponse {
    /// Hack logged, no further action
    Logged,
    /// Warning issued to player, includes current/max hack count
    Warning { hack_count: u32, max_hacks: u32 },
    /// Player should be kicked
    Kick { reason: String },
    /// Player should be banned (account + IP/MAC)
    Ban { reason: String },
}

impl HackResponse {
    /// Check if this response requires disconnection
    pub fn should_disconnect(&self) -> bool {
        matches!(self, Self::Kick { .. } | Self::Ban { .. })
    }

    /// Check if this response is a ban
    pub fn is_ban(&self) -> bool {
        matches!(self, Self::Ban { .. })
    }
}

/// Cheat detection result from movement validation
#[derive(Debug, Clone)]
pub enum CheatResult {
    /// No cheating detected
    Clean,
    /// Suspicious but not definitive
    Suspicious { reason: String, severity: u8 },
    /// Definite cheat detected
    Cheating { reason: String },
}

impl CheatResult {
    pub fn is_clean(&self) -> bool {
        matches!(self, CheatResult::Clean)
    }

    pub fn is_cheating(&self) -> bool {
        matches!(self, CheatResult::Cheating { .. })
    }
}
