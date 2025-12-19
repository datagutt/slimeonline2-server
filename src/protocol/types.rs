//! Message type definitions and parsing utilities
//!
//! This module provides a generic, reusable system for defining and parsing
//! binary message types with self-documenting field definitions.

use std::fmt;
use crate::protocol::MessageReader;

// =============================================================================
// MESSAGE TYPE ENUM
// =============================================================================

/// All message types in the Slime Online 2 protocol.
/// 
/// Each variant includes the message ID and can be converted to/from u16.
/// The Display trait provides human-readable names for logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum MessageType {
    // Authentication
    Register = 7,
    Login = 10,
    
    // Player Management
    NewPlayer = 1,
    MovePlayer = 2,
    PlayerExist = 3,
    SendId = 4,
    ChangeRoom = 5,
    Logout = 6,
    SendStatus = 8,
    PlayerLeave = 11,
    
    // Actions & Appearance
    Action = 12,
    ChangeOutfit = 13,
    Warp = 14,
    Position = 15,
    Create = 16,
    ChangeAccessory1 = 25,
    ChangeAccessory2 = 26,
    
    // Communication
    Chat = 17,
    Emote = 23,
    EmoteDice = 93,
    PlayerTyping = 133,
    
    // Points & Collection
    Point = 18,
    
    // Server Utility
    Ping = 9,
    Save = 19,
    Time = 21,
    MusicChange = 22,
    ServerClose = 24,
    CanMoveTrue = 42,
    PlayerStop = 43,
    RequestStatus = 44,
    TimeUpdate = 74,
    PingReq = 117,
    ServerTime = 118,
    ServerTimeReset = 119,
    
    // Items & Inventory
    RoomShopInfo = 27,
    ShopBuy = 28,
    ShopBuyFail = 29,
    ShopStock = 30,
    UseItem = 31,
    CollectibleInfo = 32,
    CollectibleTakeSelf = 33,
    CollectibleTaken = 34,
    OneTimeInfo = 35,
    OneTimeDisappear = 36,
    OneTimeGet = 37,
    DiscardItem = 39,
    DiscardedItemTake = 40,
    GetItem = 41,
    BankProcess = 45,
    ModAction = 46,
    Mailbox = 47,
    ItemMapSet = 48,
    ItemMapSetDestroy = 49,
    ReturnItem = 50,
    SignTxtRequest = 52,
    SellReqPrices = 53,
    Sell = 54,
    PointsDec = 55,
    StorageReq = 56,
    StoragePages = 57,
    StorageMove = 58,
    
    // Planting
    PlantSpotFree = 63,
    PlantSpotUsed = 64,
    PlantDie = 65,
    PlantGrow = 66,
    PlantAddPinwheel = 67,
    PlantAddFairy = 68,
    PlantGetFruit = 69,
    PlantHasFruit = 70,
    
    // Misc
    GetTopPoints = 73,
    GetSomething = 75,
    GetWarpInfo = 76,
    WarpCenterUseSlot = 77,
    MailSend = 78,
    MailpaperReq = 79,
    MailReceiverCheck = 80,
    ToolEquip = 81,
    ToolUnequip = 82,
    
    // Quests
    QuestBegin = 83,
    QuestClear = 84,
    QuestStepInc = 85,
    QuestCancel = 86,
    QuestNpcReq = 87,
    QuestVarCheck = 88,
    QuestVarInc = 89,
    QuestVarSet = 90,
    QuestStatusReq = 91,
    QuestReward = 92,
    TreePlantedInc = 94,
    
    // Music & Misc
    MusicChangerList = 95,
    MusicChangerSet = 96,
    PlayerThrow = 97,
    
    // Cannon
    CannonEnter = 98,
    CannonMove = 99,
    CannonSetPower = 100,
    CannonShoot = 101,
    
    // Building
    BuildSpotFree = 103,
    BuildSpotUsed = 104,
    BuildSpotBecomeFree = 105,
    ObjectsBuiltInc = 106,
    
    // Upgrader
    UpgraderGet = 108,
    UpgraderPoints = 109,
    UpgraderInvest = 110,
    UpgraderAppear = 111,
    UnlockableExists = 112,
    BuyGum = 113,
    BuySoda = 114,
    SwitchSet = 116,
    
    // Racing
    RaceInfo = 120,
    RaceStart = 121,
    RaceCheckpoint = 122,
    RaceEnd = 123,
    MoveGetOn = 124,
    MoveGetOff = 125,
    
    // Clan
    ClanCreate = 126,
    ClanDissolve = 127,
    ClanInvite = 128,
    ClanLeave = 129,
    ClanInfo = 130,
    ClanAdmin = 131,
    CollectibleEvolve = 132,
    
    // BBS
    BbsRequestCategories = 134,
    BbsRequestGui = 135,
    BbsRequestMaxPages = 136,
    BbsRequestMessages = 137,
    BbsRequestMessageContent = 138,
    BbsReportMessage = 139,
    BbsRequestPost = 140,
    BbsPost = 141,
    
    /// Unknown message type
    Unknown(u16),
}

impl MessageType {
    /// Convert from a u16 message ID to MessageType
    pub fn from_id(id: u16) -> Self {
        match id {
            1 => Self::NewPlayer,
            2 => Self::MovePlayer,
            3 => Self::PlayerExist,
            4 => Self::SendId,
            5 => Self::ChangeRoom,
            6 => Self::Logout,
            7 => Self::Register,
            8 => Self::SendStatus,
            9 => Self::Ping,
            10 => Self::Login,
            11 => Self::PlayerLeave,
            12 => Self::Action,
            13 => Self::ChangeOutfit,
            14 => Self::Warp,
            15 => Self::Position,
            16 => Self::Create,
            17 => Self::Chat,
            18 => Self::Point,
            19 => Self::Save,
            21 => Self::Time,
            22 => Self::MusicChange,
            23 => Self::Emote,
            24 => Self::ServerClose,
            25 => Self::ChangeAccessory1,
            26 => Self::ChangeAccessory2,
            27 => Self::RoomShopInfo,
            28 => Self::ShopBuy,
            29 => Self::ShopBuyFail,
            30 => Self::ShopStock,
            31 => Self::UseItem,
            32 => Self::CollectibleInfo,
            33 => Self::CollectibleTakeSelf,
            34 => Self::CollectibleTaken,
            35 => Self::OneTimeInfo,
            36 => Self::OneTimeDisappear,
            37 => Self::OneTimeGet,
            39 => Self::DiscardItem,
            40 => Self::DiscardedItemTake,
            41 => Self::GetItem,
            42 => Self::CanMoveTrue,
            43 => Self::PlayerStop,
            44 => Self::RequestStatus,
            45 => Self::BankProcess,
            46 => Self::ModAction,
            47 => Self::Mailbox,
            48 => Self::ItemMapSet,
            49 => Self::ItemMapSetDestroy,
            50 => Self::ReturnItem,
            52 => Self::SignTxtRequest,
            53 => Self::SellReqPrices,
            54 => Self::Sell,
            55 => Self::PointsDec,
            56 => Self::StorageReq,
            57 => Self::StoragePages,
            58 => Self::StorageMove,
            63 => Self::PlantSpotFree,
            64 => Self::PlantSpotUsed,
            65 => Self::PlantDie,
            66 => Self::PlantGrow,
            67 => Self::PlantAddPinwheel,
            68 => Self::PlantAddFairy,
            69 => Self::PlantGetFruit,
            70 => Self::PlantHasFruit,
            73 => Self::GetTopPoints,
            74 => Self::TimeUpdate,
            75 => Self::GetSomething,
            76 => Self::GetWarpInfo,
            77 => Self::WarpCenterUseSlot,
            78 => Self::MailSend,
            79 => Self::MailpaperReq,
            80 => Self::MailReceiverCheck,
            81 => Self::ToolEquip,
            82 => Self::ToolUnequip,
            83 => Self::QuestBegin,
            84 => Self::QuestClear,
            85 => Self::QuestStepInc,
            86 => Self::QuestCancel,
            87 => Self::QuestNpcReq,
            88 => Self::QuestVarCheck,
            89 => Self::QuestVarInc,
            90 => Self::QuestVarSet,
            91 => Self::QuestStatusReq,
            92 => Self::QuestReward,
            93 => Self::EmoteDice,
            94 => Self::TreePlantedInc,
            95 => Self::MusicChangerList,
            96 => Self::MusicChangerSet,
            97 => Self::PlayerThrow,
            98 => Self::CannonEnter,
            99 => Self::CannonMove,
            100 => Self::CannonSetPower,
            101 => Self::CannonShoot,
            103 => Self::BuildSpotFree,
            104 => Self::BuildSpotUsed,
            105 => Self::BuildSpotBecomeFree,
            106 => Self::ObjectsBuiltInc,
            108 => Self::UpgraderGet,
            109 => Self::UpgraderPoints,
            110 => Self::UpgraderInvest,
            111 => Self::UpgraderAppear,
            112 => Self::UnlockableExists,
            113 => Self::BuyGum,
            114 => Self::BuySoda,
            116 => Self::SwitchSet,
            117 => Self::PingReq,
            118 => Self::ServerTime,
            119 => Self::ServerTimeReset,
            120 => Self::RaceInfo,
            121 => Self::RaceStart,
            122 => Self::RaceCheckpoint,
            123 => Self::RaceEnd,
            124 => Self::MoveGetOn,
            125 => Self::MoveGetOff,
            126 => Self::ClanCreate,
            127 => Self::ClanDissolve,
            128 => Self::ClanInvite,
            129 => Self::ClanLeave,
            130 => Self::ClanInfo,
            131 => Self::ClanAdmin,
            132 => Self::CollectibleEvolve,
            133 => Self::PlayerTyping,
            134 => Self::BbsRequestCategories,
            135 => Self::BbsRequestGui,
            136 => Self::BbsRequestMaxPages,
            137 => Self::BbsRequestMessages,
            138 => Self::BbsRequestMessageContent,
            139 => Self::BbsReportMessage,
            140 => Self::BbsRequestPost,
            141 => Self::BbsPost,
            other => Self::Unknown(other),
        }
    }
    
    /// Get the numeric ID for this message type
    pub fn id(&self) -> u16 {
        match self {
            Self::Unknown(id) => *id,
            Self::NewPlayer => 1,
            Self::MovePlayer => 2,
            Self::PlayerExist => 3,
            Self::SendId => 4,
            Self::ChangeRoom => 5,
            Self::Logout => 6,
            Self::Register => 7,
            Self::SendStatus => 8,
            Self::Ping => 9,
            Self::Login => 10,
            Self::PlayerLeave => 11,
            Self::Action => 12,
            Self::ChangeOutfit => 13,
            Self::Warp => 14,
            Self::Position => 15,
            Self::Create => 16,
            Self::Chat => 17,
            Self::Point => 18,
            Self::Save => 19,
            Self::Time => 21,
            Self::MusicChange => 22,
            Self::Emote => 23,
            Self::ServerClose => 24,
            Self::ChangeAccessory1 => 25,
            Self::ChangeAccessory2 => 26,
            Self::RoomShopInfo => 27,
            Self::ShopBuy => 28,
            Self::ShopBuyFail => 29,
            Self::ShopStock => 30,
            Self::UseItem => 31,
            Self::CollectibleInfo => 32,
            Self::CollectibleTakeSelf => 33,
            Self::CollectibleTaken => 34,
            Self::OneTimeInfo => 35,
            Self::OneTimeDisappear => 36,
            Self::OneTimeGet => 37,
            Self::DiscardItem => 39,
            Self::DiscardedItemTake => 40,
            Self::GetItem => 41,
            Self::CanMoveTrue => 42,
            Self::PlayerStop => 43,
            Self::RequestStatus => 44,
            Self::BankProcess => 45,
            Self::ModAction => 46,
            Self::Mailbox => 47,
            Self::ItemMapSet => 48,
            Self::ItemMapSetDestroy => 49,
            Self::ReturnItem => 50,
            Self::SignTxtRequest => 52,
            Self::SellReqPrices => 53,
            Self::Sell => 54,
            Self::PointsDec => 55,
            Self::StorageReq => 56,
            Self::StoragePages => 57,
            Self::StorageMove => 58,
            Self::PlantSpotFree => 63,
            Self::PlantSpotUsed => 64,
            Self::PlantDie => 65,
            Self::PlantGrow => 66,
            Self::PlantAddPinwheel => 67,
            Self::PlantAddFairy => 68,
            Self::PlantGetFruit => 69,
            Self::PlantHasFruit => 70,
            Self::GetTopPoints => 73,
            Self::TimeUpdate => 74,
            Self::GetSomething => 75,
            Self::GetWarpInfo => 76,
            Self::WarpCenterUseSlot => 77,
            Self::MailSend => 78,
            Self::MailpaperReq => 79,
            Self::MailReceiverCheck => 80,
            Self::ToolEquip => 81,
            Self::ToolUnequip => 82,
            Self::QuestBegin => 83,
            Self::QuestClear => 84,
            Self::QuestStepInc => 85,
            Self::QuestCancel => 86,
            Self::QuestNpcReq => 87,
            Self::QuestVarCheck => 88,
            Self::QuestVarInc => 89,
            Self::QuestVarSet => 90,
            Self::QuestStatusReq => 91,
            Self::QuestReward => 92,
            Self::EmoteDice => 93,
            Self::TreePlantedInc => 94,
            Self::MusicChangerList => 95,
            Self::MusicChangerSet => 96,
            Self::PlayerThrow => 97,
            Self::CannonEnter => 98,
            Self::CannonMove => 99,
            Self::CannonSetPower => 100,
            Self::CannonShoot => 101,
            Self::BuildSpotFree => 103,
            Self::BuildSpotUsed => 104,
            Self::BuildSpotBecomeFree => 105,
            Self::ObjectsBuiltInc => 106,
            Self::UpgraderGet => 108,
            Self::UpgraderPoints => 109,
            Self::UpgraderInvest => 110,
            Self::UpgraderAppear => 111,
            Self::UnlockableExists => 112,
            Self::BuyGum => 113,
            Self::BuySoda => 114,
            Self::SwitchSet => 116,
            Self::PingReq => 117,
            Self::ServerTime => 118,
            Self::ServerTimeReset => 119,
            Self::RaceInfo => 120,
            Self::RaceStart => 121,
            Self::RaceCheckpoint => 122,
            Self::RaceEnd => 123,
            Self::MoveGetOn => 124,
            Self::MoveGetOff => 125,
            Self::ClanCreate => 126,
            Self::ClanDissolve => 127,
            Self::ClanInvite => 128,
            Self::ClanLeave => 129,
            Self::ClanInfo => 130,
            Self::ClanAdmin => 131,
            Self::CollectibleEvolve => 132,
            Self::PlayerTyping => 133,
            Self::BbsRequestCategories => 134,
            Self::BbsRequestGui => 135,
            Self::BbsRequestMaxPages => 136,
            Self::BbsRequestMessages => 137,
            Self::BbsRequestMessageContent => 138,
            Self::BbsReportMessage => 139,
            Self::BbsRequestPost => 140,
            Self::BbsPost => 141,
        }
    }
    
    /// Get a short category name for this message type
    pub fn category(&self) -> &'static str {
        match self {
            Self::Register | Self::Login => "Auth",
            Self::NewPlayer | Self::MovePlayer | Self::PlayerExist | Self::SendId |
            Self::ChangeRoom | Self::Logout | Self::SendStatus | Self::PlayerLeave => "Player",
            Self::Action | Self::ChangeOutfit | Self::ChangeAccessory1 | Self::ChangeAccessory2 => "Appearance",
            Self::Warp | Self::Position | Self::Create => "Warp",
            Self::Chat | Self::Emote | Self::EmoteDice | Self::PlayerTyping => "Chat",
            Self::Point | Self::PointsDec | Self::GetTopPoints => "Points",
            Self::Ping | Self::PingReq | Self::Save | Self::Time | Self::TimeUpdate |
            Self::ServerTime | Self::ServerTimeReset | Self::MusicChange |
            Self::ServerClose | Self::CanMoveTrue | Self::PlayerStop | Self::RequestStatus => "Server",
            Self::RoomShopInfo | Self::ShopBuy | Self::ShopBuyFail | Self::ShopStock |
            Self::SellReqPrices | Self::Sell => "Shop",
            Self::UseItem | Self::GetItem | Self::DiscardItem | Self::DiscardedItemTake |
            Self::ReturnItem | Self::ItemMapSet | Self::ItemMapSetDestroy => "Item",
            Self::CollectibleInfo | Self::CollectibleTakeSelf | Self::CollectibleTaken |
            Self::CollectibleEvolve => "Collectible",
            Self::OneTimeInfo | Self::OneTimeDisappear | Self::OneTimeGet => "OneTime",
            Self::BankProcess | Self::StorageReq | Self::StoragePages | Self::StorageMove => "Bank",
            Self::Mailbox | Self::MailSend | Self::MailpaperReq | Self::MailReceiverCheck => "Mail",
            Self::QuestBegin | Self::QuestClear | Self::QuestStepInc | Self::QuestCancel |
            Self::QuestNpcReq | Self::QuestVarCheck | Self::QuestVarInc | Self::QuestVarSet |
            Self::QuestStatusReq | Self::QuestReward => "Quest",
            Self::PlantSpotFree | Self::PlantSpotUsed | Self::PlantDie | Self::PlantGrow |
            Self::PlantAddPinwheel | Self::PlantAddFairy | Self::PlantGetFruit |
            Self::PlantHasFruit | Self::TreePlantedInc => "Plant",
            Self::BuildSpotFree | Self::BuildSpotUsed | Self::BuildSpotBecomeFree |
            Self::ObjectsBuiltInc => "Build",
            Self::CannonEnter | Self::CannonMove | Self::CannonSetPower | Self::CannonShoot => "Cannon",
            Self::RaceInfo | Self::RaceStart | Self::RaceCheckpoint | Self::RaceEnd |
            Self::MoveGetOn | Self::MoveGetOff => "Race",
            Self::ClanCreate | Self::ClanDissolve | Self::ClanInvite | Self::ClanLeave |
            Self::ClanInfo | Self::ClanAdmin => "Clan",
            Self::BbsRequestCategories | Self::BbsRequestGui | Self::BbsRequestMaxPages |
            Self::BbsRequestMessages | Self::BbsRequestMessageContent | Self::BbsReportMessage |
            Self::BbsRequestPost | Self::BbsPost => "BBS",
            Self::UpgraderGet | Self::UpgraderPoints | Self::UpgraderInvest |
            Self::UpgraderAppear | Self::UnlockableExists => "Upgrader",
            Self::MusicChangerList | Self::MusicChangerSet | Self::PlayerThrow |
            Self::SignTxtRequest | Self::BuyGum | Self::BuySoda | Self::SwitchSet |
            Self::ModAction | Self::ToolEquip | Self::ToolUnequip | Self::GetSomething |
            Self::GetWarpInfo | Self::WarpCenterUseSlot => "Misc",
            Self::Unknown(_) => "Unknown",
        }
    }
    
    /// Check if this is a high-frequency message that should not be logged
    pub fn is_high_frequency(&self) -> bool {
        matches!(self, Self::Ping | Self::MovePlayer)
    }
}

impl fmt::Display for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::NewPlayer => "NewPlayer",
            Self::MovePlayer => "MovePlayer",
            Self::PlayerExist => "PlayerExist",
            Self::SendId => "SendId",
            Self::ChangeRoom => "ChangeRoom",
            Self::Logout => "Logout",
            Self::Register => "Register",
            Self::SendStatus => "SendStatus",
            Self::Ping => "Ping",
            Self::Login => "Login",
            Self::PlayerLeave => "PlayerLeave",
            Self::Action => "Action",
            Self::ChangeOutfit => "ChangeOutfit",
            Self::Warp => "Warp",
            Self::Position => "Position",
            Self::Create => "Create",
            Self::Chat => "Chat",
            Self::Point => "Point",
            Self::Save => "Save",
            Self::Time => "Time",
            Self::MusicChange => "MusicChange",
            Self::Emote => "Emote",
            Self::ServerClose => "ServerClose",
            Self::ChangeAccessory1 => "ChangeAccessory1",
            Self::ChangeAccessory2 => "ChangeAccessory2",
            Self::RoomShopInfo => "RoomShopInfo",
            Self::ShopBuy => "ShopBuy",
            Self::ShopBuyFail => "ShopBuyFail",
            Self::ShopStock => "ShopStock",
            Self::UseItem => "UseItem",
            Self::CollectibleInfo => "CollectibleInfo",
            Self::CollectibleTakeSelf => "CollectibleTakeSelf",
            Self::CollectibleTaken => "CollectibleTaken",
            Self::OneTimeInfo => "OneTimeInfo",
            Self::OneTimeDisappear => "OneTimeDisappear",
            Self::OneTimeGet => "OneTimeGet",
            Self::DiscardItem => "DiscardItem",
            Self::DiscardedItemTake => "DiscardedItemTake",
            Self::GetItem => "GetItem",
            Self::CanMoveTrue => "CanMoveTrue",
            Self::PlayerStop => "PlayerStop",
            Self::RequestStatus => "RequestStatus",
            Self::BankProcess => "BankProcess",
            Self::ModAction => "ModAction",
            Self::Mailbox => "Mailbox",
            Self::ItemMapSet => "ItemMapSet",
            Self::ItemMapSetDestroy => "ItemMapSetDestroy",
            Self::ReturnItem => "ReturnItem",
            Self::SignTxtRequest => "SignTxtRequest",
            Self::SellReqPrices => "SellReqPrices",
            Self::Sell => "Sell",
            Self::PointsDec => "PointsDec",
            Self::StorageReq => "StorageReq",
            Self::StoragePages => "StoragePages",
            Self::StorageMove => "StorageMove",
            Self::PlantSpotFree => "PlantSpotFree",
            Self::PlantSpotUsed => "PlantSpotUsed",
            Self::PlantDie => "PlantDie",
            Self::PlantGrow => "PlantGrow",
            Self::PlantAddPinwheel => "PlantAddPinwheel",
            Self::PlantAddFairy => "PlantAddFairy",
            Self::PlantGetFruit => "PlantGetFruit",
            Self::PlantHasFruit => "PlantHasFruit",
            Self::GetTopPoints => "GetTopPoints",
            Self::TimeUpdate => "TimeUpdate",
            Self::GetSomething => "GetSomething",
            Self::GetWarpInfo => "GetWarpInfo",
            Self::WarpCenterUseSlot => "WarpCenterUseSlot",
            Self::MailSend => "MailSend",
            Self::MailpaperReq => "MailpaperReq",
            Self::MailReceiverCheck => "MailReceiverCheck",
            Self::ToolEquip => "ToolEquip",
            Self::ToolUnequip => "ToolUnequip",
            Self::QuestBegin => "QuestBegin",
            Self::QuestClear => "QuestClear",
            Self::QuestStepInc => "QuestStepInc",
            Self::QuestCancel => "QuestCancel",
            Self::QuestNpcReq => "QuestNpcReq",
            Self::QuestVarCheck => "QuestVarCheck",
            Self::QuestVarInc => "QuestVarInc",
            Self::QuestVarSet => "QuestVarSet",
            Self::QuestStatusReq => "QuestStatusReq",
            Self::QuestReward => "QuestReward",
            Self::EmoteDice => "EmoteDice",
            Self::TreePlantedInc => "TreePlantedInc",
            Self::MusicChangerList => "MusicChangerList",
            Self::MusicChangerSet => "MusicChangerSet",
            Self::PlayerThrow => "PlayerThrow",
            Self::CannonEnter => "CannonEnter",
            Self::CannonMove => "CannonMove",
            Self::CannonSetPower => "CannonSetPower",
            Self::CannonShoot => "CannonShoot",
            Self::BuildSpotFree => "BuildSpotFree",
            Self::BuildSpotUsed => "BuildSpotUsed",
            Self::BuildSpotBecomeFree => "BuildSpotBecomeFree",
            Self::ObjectsBuiltInc => "ObjectsBuiltInc",
            Self::UpgraderGet => "UpgraderGet",
            Self::UpgraderPoints => "UpgraderPoints",
            Self::UpgraderInvest => "UpgraderInvest",
            Self::UpgraderAppear => "UpgraderAppear",
            Self::UnlockableExists => "UnlockableExists",
            Self::BuyGum => "BuyGum",
            Self::BuySoda => "BuySoda",
            Self::SwitchSet => "SwitchSet",
            Self::PingReq => "PingReq",
            Self::ServerTime => "ServerTime",
            Self::ServerTimeReset => "ServerTimeReset",
            Self::RaceInfo => "RaceInfo",
            Self::RaceStart => "RaceStart",
            Self::RaceCheckpoint => "RaceCheckpoint",
            Self::RaceEnd => "RaceEnd",
            Self::MoveGetOn => "MoveGetOn",
            Self::MoveGetOff => "MoveGetOff",
            Self::ClanCreate => "ClanCreate",
            Self::ClanDissolve => "ClanDissolve",
            Self::ClanInvite => "ClanInvite",
            Self::ClanLeave => "ClanLeave",
            Self::ClanInfo => "ClanInfo",
            Self::ClanAdmin => "ClanAdmin",
            Self::CollectibleEvolve => "CollectibleEvolve",
            Self::PlayerTyping => "PlayerTyping",
            Self::BbsRequestCategories => "BbsRequestCategories",
            Self::BbsRequestGui => "BbsRequestGui",
            Self::BbsRequestMaxPages => "BbsRequestMaxPages",
            Self::BbsRequestMessages => "BbsRequestMessages",
            Self::BbsRequestMessageContent => "BbsRequestMessageContent",
            Self::BbsReportMessage => "BbsReportMessage",
            Self::BbsRequestPost => "BbsRequestPost",
            Self::BbsPost => "BbsPost",
            Self::Unknown(id) => return write!(f, "Unknown({})", id),
        };
        write!(f, "{}", name)
    }
}

// =============================================================================
// FIELD VALUE TYPE - for generic field storage
// =============================================================================

/// A typed value that can be read from a message payload.
#[derive(Debug, Clone)]
pub enum FieldValue {
    U8(u8),
    U16(u16),
    U32(u32),
    I8(i8),
    I16(i16),
    I32(i32),
    F32(f32),
    Bool(bool),
    String(String),
    Bytes(Vec<u8>),
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::U8(v) => write!(f, "{}", v),
            Self::U16(v) => write!(f, "{}", v),
            Self::U32(v) => write!(f, "{}", v),
            Self::I8(v) => write!(f, "{}", v),
            Self::I16(v) => write!(f, "{}", v),
            Self::I32(v) => write!(f, "{}", v),
            Self::F32(v) => write!(f, "{:.2}", v),
            Self::Bool(v) => write!(f, "{}", v),
            Self::String(v) => write!(f, "\"{}\"", v),
            Self::Bytes(v) => write!(f, "[{} bytes]", v.len()),
        }
    }
}

impl FieldValue {
    pub fn as_u8(&self) -> Option<u8> {
        match self { Self::U8(v) => Some(*v), _ => None }
    }
    pub fn as_u16(&self) -> Option<u16> {
        match self { Self::U16(v) => Some(*v), _ => None }
    }
    pub fn as_u32(&self) -> Option<u32> {
        match self { Self::U32(v) => Some(*v), _ => None }
    }
    pub fn as_i16(&self) -> Option<i16> {
        match self { Self::I16(v) => Some(*v), _ => None }
    }
    pub fn as_string(&self) -> Option<&str> {
        match self { Self::String(v) => Some(v.as_str()), _ => None }
    }
}

// =============================================================================
// FIELD DEFINITION - describes how to read a field
// =============================================================================

/// Defines how to read a field from a binary message.
#[derive(Debug, Clone)]
pub struct FieldDef {
    /// Human-readable name for this field
    pub name: &'static str,
    /// The type of data to read
    pub field_type: FieldType,
}

/// The type of a field, determining how to read it from binary data.
#[derive(Debug, Clone, Copy)]
pub enum FieldType {
    U8,
    U16,
    U32,
    I8,
    I16,
    I32,
    F32,
    Bool,
    /// A length-prefixed string (2-byte length prefix)
    String,
    /// Fixed-size byte array
    Bytes(usize),
}

impl FieldDef {
    pub const fn new(name: &'static str, field_type: FieldType) -> Self {
        Self { name, field_type }
    }
    
    /// Read this field from a MessageReader
    pub fn read(&self, reader: &mut MessageReader) -> Option<FieldValue> {
        match self.field_type {
            FieldType::U8 => reader.read_u8().ok().map(FieldValue::U8),
            FieldType::U16 => reader.read_u16().ok().map(FieldValue::U16),
            FieldType::U32 => reader.read_u32().ok().map(FieldValue::U32),
            FieldType::I8 => reader.read_u8().ok().map(|v| FieldValue::I8(v as i8)),
            FieldType::I16 => reader.read_u16().ok().map(|v| FieldValue::I16(v as i16)),
            FieldType::I32 => reader.read_u32().ok().map(|v| FieldValue::I32(v as i32)),
            FieldType::F32 => reader.read_u32().ok().map(|v| FieldValue::F32(f32::from_bits(v))),
            FieldType::Bool => reader.read_u8().ok().map(|v| FieldValue::Bool(v != 0)),
            FieldType::String => reader.read_string().ok().map(FieldValue::String),
            FieldType::Bytes(len) => {
                let mut bytes = vec![0u8; len];
                for i in 0..len {
                    bytes[i] = reader.read_u8().ok()?;
                }
                Some(FieldValue::Bytes(bytes))
            }
        }
    }
}

// =============================================================================
// PARSED MESSAGE - a message with named, typed fields
// =============================================================================

/// A parsed message with named fields.
#[derive(Debug)]
pub struct ParsedMessage {
    pub msg_type: MessageType,
    pub fields: Vec<(&'static str, FieldValue)>,
}

impl ParsedMessage {
    pub fn new(msg_type: MessageType) -> Self {
        Self {
            msg_type,
            fields: Vec::new(),
        }
    }
    
    pub fn with_field(mut self, name: &'static str, value: FieldValue) -> Self {
        self.fields.push((name, value));
        self
    }
    
    pub fn get(&self, name: &str) -> Option<&FieldValue> {
        self.fields.iter().find(|(n, _)| *n == name).map(|(_, v)| v)
    }
    
    pub fn get_u8(&self, name: &str) -> Option<u8> {
        self.get(name).and_then(|v| v.as_u8())
    }
    
    pub fn get_u16(&self, name: &str) -> Option<u16> {
        self.get(name).and_then(|v| v.as_u16())
    }
    
    pub fn get_u32(&self, name: &str) -> Option<u32> {
        self.get(name).and_then(|v| v.as_u32())
    }
    
    pub fn get_i16(&self, name: &str) -> Option<i16> {
        self.get(name).and_then(|v| v.as_i16())
    }
    
    pub fn get_string(&self, name: &str) -> Option<&str> {
        self.get(name).and_then(|v| v.as_string())
    }
}

impl fmt::Display for ParsedMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.msg_type.category(), self.msg_type)?;
        if !self.fields.is_empty() {
            write!(f, " {{")?;
            for (i, (name, value)) in self.fields.iter().enumerate() {
                if i > 0 {
                    write!(f, ",")?;
                }
                write!(f, " {}: {}", name, value)?;
            }
            write!(f, " }}")?;
        }
        Ok(())
    }
}

// =============================================================================
// MESSAGE SCHEMA - defines the structure of a message
// =============================================================================

/// Defines the structure of a message type for parsing.
pub struct MessageSchema {
    pub msg_type: MessageType,
    pub fields: &'static [FieldDef],
}

impl MessageSchema {
    pub const fn new(msg_type: MessageType, fields: &'static [FieldDef]) -> Self {
        Self { msg_type, fields }
    }
    
    /// Parse a message payload using this schema
    pub fn parse(&self, payload: &[u8]) -> ParsedMessage {
        let mut reader = MessageReader::new(payload);
        let mut msg = ParsedMessage::new(self.msg_type);
        
        for field_def in self.fields {
            if let Some(value) = field_def.read(&mut reader) {
                msg.fields.push((field_def.name, value));
            } else {
                break; // Stop on first read failure
            }
        }
        
        msg
    }
}

// =============================================================================
// PREDEFINED MESSAGE SCHEMAS
// =============================================================================

/// Schema for MSG_LOGIN (10) - Client → Server
pub static LOGIN_SCHEMA: MessageSchema = MessageSchema::new(
    MessageType::Login,
    &[
        FieldDef::new("version", FieldType::String),
        FieldDef::new("username", FieldType::String),
        FieldDef::new("password", FieldType::String),
        FieldDef::new("mac_address", FieldType::String),
    ],
);

/// Schema for MSG_REGISTER (7) - Client → Server
pub static REGISTER_SCHEMA: MessageSchema = MessageSchema::new(
    MessageType::Register,
    &[
        FieldDef::new("username", FieldType::String),
        FieldDef::new("password", FieldType::String),
        FieldDef::new("mac_address", FieldType::String),
    ],
);

/// Schema for MSG_MOVE_PLAYER (2) - Client → Server
pub static MOVE_PLAYER_SCHEMA: MessageSchema = MessageSchema::new(
    MessageType::MovePlayer,
    &[
        FieldDef::new("direction", FieldType::U8),
        FieldDef::new("x", FieldType::U16),
        FieldDef::new("y", FieldType::U16),
    ],
);

/// Schema for MSG_CHAT (17) - Client → Server
pub static CHAT_SCHEMA: MessageSchema = MessageSchema::new(
    MessageType::Chat,
    &[
        FieldDef::new("message", FieldType::String),
    ],
);

/// Schema for MSG_WARP (14) - Client → Server
pub static WARP_SCHEMA: MessageSchema = MessageSchema::new(
    MessageType::Warp,
    &[
        FieldDef::new("room_id", FieldType::U16),
        FieldDef::new("x", FieldType::U16),
        FieldDef::new("y", FieldType::U16),
    ],
);

/// Schema for MSG_EMOTE (23) - Client → Server
pub static EMOTE_SCHEMA: MessageSchema = MessageSchema::new(
    MessageType::Emote,
    &[
        FieldDef::new("emote_id", FieldType::U8),
    ],
);

/// Schema for MSG_ACTION (12) - Client → Server
pub static ACTION_SCHEMA: MessageSchema = MessageSchema::new(
    MessageType::Action,
    &[
        FieldDef::new("action_id", FieldType::U8),
    ],
);

/// Schema for MSG_CHANGE_OUTFIT (13) - Client → Server
pub static CHANGE_OUTFIT_SCHEMA: MessageSchema = MessageSchema::new(
    MessageType::ChangeOutfit,
    &[
        FieldDef::new("slot", FieldType::U8),
    ],
);

/// Schema for MSG_CHANGE_ACCESSORY1 (25) - Client → Server
pub static CHANGE_ACCESSORY1_SCHEMA: MessageSchema = MessageSchema::new(
    MessageType::ChangeAccessory1,
    &[
        FieldDef::new("slot", FieldType::U8),
    ],
);

/// Schema for MSG_CHANGE_ACCESSORY2 (26) - Client → Server
pub static CHANGE_ACCESSORY2_SCHEMA: MessageSchema = MessageSchema::new(
    MessageType::ChangeAccessory2,
    &[
        FieldDef::new("slot", FieldType::U8),
    ],
);

/// Schema for MSG_POINT (18) - Client → Server
pub static POINT_SCHEMA: MessageSchema = MessageSchema::new(
    MessageType::Point,
    &[
        FieldDef::new("point_index", FieldType::U8),
    ],
);

/// Schema for MSG_NEW_PLAYER response (1) - Client → Server
pub static NEW_PLAYER_RESPONSE_SCHEMA: MessageSchema = MessageSchema::new(
    MessageType::NewPlayer,
    &[
        FieldDef::new("player_id", FieldType::U16),
        FieldDef::new("x", FieldType::U16),
        FieldDef::new("y", FieldType::U16),
        FieldDef::new("body_id", FieldType::U16),
        FieldDef::new("acs1_id", FieldType::U16),
        FieldDef::new("acs2_id", FieldType::U16),
        FieldDef::new("username", FieldType::String),
        FieldDef::new("status", FieldType::String),
        FieldDef::new("clan_color1", FieldType::U32),
        FieldDef::new("clan_color2", FieldType::U32),
        FieldDef::new("clan_name", FieldType::String),
    ],
);

// =============================================================================
// HELPER FUNCTION TO GET SCHEMA BY MESSAGE TYPE
// =============================================================================

/// Get the schema for a message type (if defined)
pub fn get_schema(msg_type: MessageType) -> Option<&'static MessageSchema> {
    match msg_type {
        MessageType::Login => Some(&LOGIN_SCHEMA),
        MessageType::Register => Some(&REGISTER_SCHEMA),
        MessageType::MovePlayer => Some(&MOVE_PLAYER_SCHEMA),
        MessageType::Chat => Some(&CHAT_SCHEMA),
        MessageType::Warp => Some(&WARP_SCHEMA),
        MessageType::Emote => Some(&EMOTE_SCHEMA),
        MessageType::Action => Some(&ACTION_SCHEMA),
        MessageType::ChangeOutfit => Some(&CHANGE_OUTFIT_SCHEMA),
        MessageType::ChangeAccessory1 => Some(&CHANGE_ACCESSORY1_SCHEMA),
        MessageType::ChangeAccessory2 => Some(&CHANGE_ACCESSORY2_SCHEMA),
        MessageType::Point => Some(&POINT_SCHEMA),
        MessageType::NewPlayer => Some(&NEW_PLAYER_RESPONSE_SCHEMA),
        _ => None,
    }
}

/// Parse a message with its schema (if available), returning a formatted description
pub fn describe_message(msg_type_id: u16, payload: &[u8]) -> String {
    let msg_type = MessageType::from_id(msg_type_id);
    
    if let Some(schema) = get_schema(msg_type) {
        let parsed = schema.parse(payload);
        format!("{}", parsed)
    } else {
        format!("[{}] {} (payload: {} bytes)", msg_type.category(), msg_type, payload.len())
    }
}
