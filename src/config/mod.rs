//! Configuration module for Slime Online 2 server
//!
//! Loads and provides access to game configuration from TOML files.
//! Config files are the single source of truth for static game data like
//! prices, spawn points, collectible locations, etc.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file {path}: {source}")]
    IoError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse config file {path}: {source}")]
    ParseError {
        path: String,
        #[source]
        source: toml::de::Error,
    },
}

/// Complete game configuration loaded from all TOML files
#[derive(Debug, Clone)]
pub struct GameConfig {
    pub server: ServerConfig,
    pub game: GameRulesConfig,
    pub prices: PriceConfig,
    pub shops: ShopsConfig,
    pub collectibles: CollectiblesConfig,
    pub plants: PlantsConfig,
    pub clans: ClansConfig,
    pub upgrader: UpgraderConfig,
}

// =============================================================================
// server.toml
// =============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub server: ServerSettingsConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub admin: AdminConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerSettingsConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_database_path")]
    pub database_path: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default = "default_motd")]
    pub motd: String,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default)]
    pub max_connections_per_ip: usize,
    #[serde(default)]
    pub auto_save_position: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkConfig {
    #[serde(default = "default_max_message_size")]
    pub max_message_size: usize,
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_secs: u64,
    #[serde(default = "default_ping_interval")]
    pub ping_interval_secs: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            max_message_size: default_max_message_size(),
            connection_timeout_secs: default_connection_timeout(),
            ping_interval_secs: default_ping_interval(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_admin_port")]
    pub port: u16,
    #[serde(default = "default_admin_host")]
    pub host: String,
    #[serde(default)]
    pub api_key: String,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_admin_port(),
            host: default_admin_host(),
            api_key: String::new(),
        }
    }
}

fn default_admin_port() -> u16 { 8080 }
fn default_admin_host() -> String { "127.0.0.1".to_string() }

fn default_port() -> u16 { 5555 }
fn default_database_path() -> String { "slime_online2.db".to_string() }
fn default_max_connections() -> usize { 500 }
fn default_version() -> String { "0.106".to_string() }
fn default_motd() -> String { "Welcome to Slime Online 2 Private Server!".to_string() }
fn default_host() -> String { "0.0.0.0".to_string() }
fn default_log_level() -> String { "info".to_string() }
fn default_max_message_size() -> usize { 8192 }
fn default_connection_timeout() -> u64 { 300 }
fn default_ping_interval() -> u64 { 30 }

// =============================================================================
// game.toml
// =============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct GameRulesConfig {
    pub limits: LimitsConfig,
    pub defaults: DefaultsConfig,
    pub welcome_mail: WelcomeMailConfig,
    pub bbs: BbsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LimitsConfig {
    pub max_username_length: usize,
    pub min_username_length: usize,
    pub max_password_length: usize,
    pub min_password_length: usize,
    pub max_chat_length: usize,
    pub max_points: u32,
    pub max_bank_balance: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DefaultsConfig {
    pub spawn_x: u16,
    pub spawn_y: u16,
    pub spawn_room: u16,
    pub outfit: u16,
    #[serde(default)]
    pub accessory1: u16,
    #[serde(default)]
    pub accessory2: u16,
    pub signature: u8,
    pub signature_bg: u8,
    pub emotes: [u8; 5],
    pub starting_items: [u16; 9],
    #[serde(default)]
    pub starting_outfits: [u16; 9],
    #[serde(default)]
    pub starting_accessories: [u16; 9],
    #[serde(default)]
    pub starting_tools: [u8; 9],
    #[serde(default)]
    pub unlocked_mail_paper: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WelcomeMailConfig {
    pub sender: String,
    pub text: String,
    pub points: u16,
    pub paper: u8,
    pub font: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BbsConfig {
    pub categories: Vec<String>,
}

// =============================================================================
// prices.toml
// =============================================================================

/// Rules section within prices.toml
#[derive(Debug, Clone, Deserialize, Default)]
struct PriceRulesRaw {
    #[serde(default)]
    discardable: Vec<u16>,
}

/// Raw config structure that matches the TOML file (string keys)
#[derive(Debug, Clone, Deserialize)]
struct PriceConfigRaw {
    items: HashMap<String, ItemPriceEntry>,
    outfits: HashMap<String, u32>,
    accessories: HashMap<String, u32>,
    tools: HashMap<String, ToolPriceEntry>,
    mail_paper: HashMap<String, u16>,
    #[serde(default)]
    rules: PriceRulesRaw,
}

/// Processed config with numeric keys for fast lookups
#[derive(Debug, Clone)]
pub struct PriceConfig {
    pub items: HashMap<u16, ItemPriceEntry>,
    pub outfits: HashMap<u16, u32>,
    pub accessories: HashMap<u16, u32>,
    pub tools: HashMap<u8, ToolPriceEntry>,
    pub mail_paper: HashMap<u8, u16>,
    pub discardable: Vec<u16>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemPriceEntry {
    pub name: String,
    pub price: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolPriceEntry {
    pub name: String,
    pub price: u32,
}

impl From<PriceConfigRaw> for PriceConfig {
    fn from(raw: PriceConfigRaw) -> Self {
        Self {
            items: raw.items.into_iter()
                .filter_map(|(k, v)| k.parse::<u16>().ok().map(|id| (id, v)))
                .collect(),
            outfits: raw.outfits.into_iter()
                .filter_map(|(k, v)| k.parse::<u16>().ok().map(|id| (id, v)))
                .collect(),
            accessories: raw.accessories.into_iter()
                .filter_map(|(k, v)| k.parse::<u16>().ok().map(|id| (id, v)))
                .collect(),
            tools: raw.tools.into_iter()
                .filter_map(|(k, v)| k.parse::<u8>().ok().map(|id| (id, v)))
                .collect(),
            mail_paper: raw.mail_paper.into_iter()
                .filter_map(|(k, v)| k.parse::<u8>().ok().map(|id| (id, v)))
                .collect(),
            discardable: raw.rules.discardable,
        }
    }
}

impl PriceConfig {
    /// Get the buy price for an item
    pub fn get_item_price(&self, item_id: u16) -> Option<u32> {
        self.items.get(&item_id).map(|e| e.price)
    }

    /// Get the sell price for an item (buy_price / 3, rounded down)
    pub fn get_item_sell_price(&self, item_id: u16) -> Option<u32> {
        self.items.get(&item_id).map(|e| e.price / 3)
    }

    /// Get the buy price for an outfit
    pub fn get_outfit_price(&self, outfit_id: u16) -> Option<u32> {
        self.outfits.get(&outfit_id).copied()
    }

    /// Get the sell price for an outfit
    pub fn get_outfit_sell_price(&self, outfit_id: u16) -> Option<u32> {
        self.outfits.get(&outfit_id).map(|p| p / 3)
    }

    /// Get the buy price for an accessory
    pub fn get_accessory_price(&self, accessory_id: u16) -> Option<u32> {
        self.accessories.get(&accessory_id).copied()
    }

    /// Get the sell price for an accessory
    pub fn get_accessory_sell_price(&self, accessory_id: u16) -> Option<u32> {
        self.accessories.get(&accessory_id).map(|p| p / 3)
    }

    /// Check if an item can be discarded (dropped on ground)
    pub fn is_discardable(&self, item_id: u16) -> bool {
        self.discardable.contains(&item_id)
    }
}

// =============================================================================
// shops.toml
// =============================================================================

/// Raw shops config as parsed from TOML (nested room tables)
#[derive(Debug, Clone, Deserialize)]
struct ShopsConfigRaw {
    #[serde(default)]
    room: HashMap<String, RoomShopConfig>,
}

/// Processed shops config with numeric room IDs
#[derive(Debug, Clone)]
pub struct ShopsConfig {
    pub rooms: HashMap<u16, RoomShopConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoomShopConfig {
    pub slots: Vec<ShopSlotConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShopSlotConfig {
    pub cat: u8,      // 1=outfit, 2=item, 3=accessory, 4=tool
    pub item: u16,    // item/outfit/accessory/tool ID
    pub stock: u16,   // 0 = unlimited
    #[serde(default = "default_true")]
    pub avail: bool,  // whether slot is visible/purchasable
}

fn default_true() -> bool {
    true
}

impl From<ShopsConfigRaw> for ShopsConfig {
    fn from(raw: ShopsConfigRaw) -> Self {
        Self {
            rooms: raw.room.into_iter()
                .filter_map(|(k, v)| k.parse::<u16>().ok().map(|id| (id, v)))
                .collect(),
        }
    }
}

impl ShopsConfig {
    /// Get shop config for a room by room ID
    pub fn get_room(&self, room_id: u16) -> Option<&RoomShopConfig> {
        self.rooms.get(&room_id)
    }
}

// =============================================================================
// collectibles.toml
// =============================================================================

/// Raw collectibles config as parsed from TOML
#[derive(Debug, Clone, Deserialize)]
struct CollectiblesConfigRaw {
    #[serde(default)]
    evolving: HashMap<String, EvolvingConfig>,
    #[serde(default)]
    room: HashMap<String, RoomCollectiblesConfig>,
}

/// Processed collectibles config with numeric IDs
#[derive(Debug, Clone)]
pub struct CollectiblesConfig {
    pub evolving: HashMap<u16, EvolvingConfig>,
    pub rooms: HashMap<u16, RoomCollectiblesConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EvolvingConfig {
    pub to: u16,
    pub minutes: u32,
    #[serde(default)]
    pub variance: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoomCollectiblesConfig {
    pub spawns: Vec<CollectibleSpawnConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CollectibleSpawnConfig {
    pub id: u8,
    pub item: u16,
    pub x: u16,
    pub y: u16,
    pub respawn: u32,    // base respawn time in minutes
    #[serde(default)]
    pub variance: u32,   // random additional minutes (0 to variance)
    #[serde(default)]
    pub start_hour: Option<u8>,  // optional time restriction
    #[serde(default)]
    pub end_hour: Option<u8>,
}

impl From<CollectiblesConfigRaw> for CollectiblesConfig {
    fn from(raw: CollectiblesConfigRaw) -> Self {
        Self {
            evolving: raw.evolving.into_iter()
                .filter_map(|(k, v)| k.parse::<u16>().ok().map(|id| (id, v)))
                .collect(),
            rooms: raw.room.into_iter()
                .filter_map(|(k, v)| k.parse::<u16>().ok().map(|id| (id, v)))
                .collect(),
        }
    }
}

impl CollectiblesConfig {
    /// Get collectible spawns for a room by room ID
    pub fn get_room(&self, room_id: u16) -> Option<&RoomCollectiblesConfig> {
        self.rooms.get(&room_id)
    }

    /// Check if an item can evolve
    pub fn get_evolution(&self, item_id: u16) -> Option<&EvolvingConfig> {
        self.evolving.get(&item_id)
    }
}

// =============================================================================
// plants.toml
// =============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct PlantsConfig {
    pub bonuses: PlantBonusesConfig,
    #[serde(default)]
    pub seeds: HashMap<String, SeedConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlantBonusesConfig {
    pub fairy_chance_bonus: u8,
    pub max_fairies: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeedConfig {
    pub name: String,
    pub stages: [u32; 6],   // minutes for each growth stage
    pub fruits: [u16; 5],   // possible fruit item IDs
    pub chance: u8,         // base % chance for fruit
}

impl PlantsConfig {
    /// Get seed config by seed item ID
    pub fn get_seed(&self, seed_id: u16) -> Option<&SeedConfig> {
        let key = format!("seeds.{}", seed_id);
        self.seeds.get(&key)
    }
}

// =============================================================================
// upgrader.toml
// =============================================================================

/// Raw upgrader config as parsed from TOML
#[derive(Debug, Clone, Deserialize)]
struct UpgraderConfigRaw {
    #[serde(default)]
    towns: Vec<TownUpgraderConfigRaw>,
}

#[derive(Debug, Clone, Deserialize)]
struct TownUpgraderConfigRaw {
    town_id: u16,
    #[serde(default)]
    warp_center_room: u16,
    #[serde(default)]
    upgrades: Vec<UpgradeSlotConfig>,
}

/// Processed upgrader config with easy lookups
#[derive(Debug, Clone, Default)]
pub struct UpgraderConfig {
    pub towns: HashMap<u16, TownUpgraderConfig>,
}

#[derive(Debug, Clone)]
pub struct TownUpgraderConfig {
    pub town_id: u16,
    pub warp_center_room: u16,
    /// Upgrades indexed by (category, slot)
    pub upgrades: HashMap<(String, u8), UpgradeSlotConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpgradeSlotConfig {
    pub category: String,
    pub slot: u8,
    pub name: String,
    pub need: u32,
    #[serde(default)]
    pub unlocked: bool,
    #[serde(default)]
    pub option: u8,
    #[serde(default)]
    pub other1: i32,
    #[serde(default)]
    pub other2: i32,
    #[serde(default)]
    pub other3: i32,
    #[serde(default)]
    pub other4: i32,
    #[serde(default)]
    pub other5: i32,
    // For warp center upgrades
    #[serde(default)]
    pub warp_slot: u8,
    #[serde(default)]
    pub warp_category: u8,
    // For unlock chaining
    #[serde(default)]
    pub unlock_chain: Vec<UnlockChainEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UnlockChainEntry {
    pub category: String,
    pub slot: u8,
}

impl From<UpgraderConfigRaw> for UpgraderConfig {
    fn from(raw: UpgraderConfigRaw) -> Self {
        let towns = raw.towns.into_iter().map(|town| {
            let upgrades = town.upgrades.into_iter()
                .map(|u| ((u.category.clone(), u.slot), u))
                .collect();
            (town.town_id, TownUpgraderConfig {
                town_id: town.town_id,
                warp_center_room: town.warp_center_room,
                upgrades,
            })
        }).collect();
        Self { towns }
    }
}

impl UpgraderConfig {
    /// Get town upgrader config by town room ID
    pub fn get_town(&self, town_id: u16) -> Option<&TownUpgraderConfig> {
        self.towns.get(&town_id)
    }

    /// Get a specific upgrade slot
    pub fn get_upgrade(&self, town_id: u16, category: &str, slot: u8) -> Option<&UpgradeSlotConfig> {
        self.towns.get(&town_id)
            .and_then(|t| t.upgrades.get(&(category.to_string(), slot)))
    }

    /// Get all upgrades for a town and category
    pub fn get_category_upgrades(&self, town_id: u16, category: &str) -> Vec<&UpgradeSlotConfig> {
        self.towns.get(&town_id)
            .map(|t| {
                t.upgrades.iter()
                    .filter(|((cat, _), _)| cat == category)
                    .map(|(_, u)| u)
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl TownUpgraderConfig {
    /// Get upgrades for a specific page (4 per page, 1-based slots)
    pub fn get_page_upgrades(&self, category: &str, page: u8) -> Vec<Option<&UpgradeSlotConfig>> {
        let start_slot = (page * 4) + 1;
        (0..4).map(|i| {
            let slot = start_slot + i;
            self.upgrades.get(&(category.to_string(), slot))
        }).collect()
    }

    /// Check if there are more slots after the given page
    pub fn has_more_slots(&self, category: &str, page: u8) -> bool {
        let next_slot = (page * 4) + 5;
        self.upgrades.contains_key(&(category.to_string(), next_slot))
    }
}

// =============================================================================
// clans.toml
// =============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct ClansConfig {
    pub creation: ClanCreationConfig,
    pub limits: ClanLimitsConfig,
    #[serde(default)]
    pub defaults: ClanDefaultsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClanCreationConfig {
    pub cost: u32,
    pub required_items: Vec<u16>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClanLimitsConfig {
    pub min_name_length: usize,
    pub max_name_length: usize,
    pub initial_member_slots: u8,
    pub max_member_slots: u8,
    #[serde(default)]
    pub base_unlock_cost: u32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ClanDefaultsConfig {
    #[serde(default)]
    pub inner_color: ColorConfig,
    #[serde(default)]
    pub outer_color: ColorConfig,
    #[serde(default = "default_news")]
    pub news: String,
    #[serde(default = "default_info")]
    pub info: String,
    #[serde(default = "default_true")]
    pub show_leader: bool,
}

fn default_news() -> String {
    "No news".to_string()
}

fn default_info() -> String {
    "A new clan".to_string()
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ColorConfig {
    #[serde(default)]
    pub r: u8,
    #[serde(default)]
    pub g: u8,
    #[serde(default)]
    pub b: u8,
}

// =============================================================================
// Config Loading
// =============================================================================

impl GameConfig {
    /// Load all configuration files from the given directory
    pub fn load(config_dir: &str) -> Result<Self, ConfigError> {
        let dir = Path::new(config_dir);

        let server = load_toml::<ServerConfig>(&dir.join("server.toml"))?;
        let game = load_toml::<GameRulesConfig>(&dir.join("game.toml"))?;
        let prices_raw = load_toml::<PriceConfigRaw>(&dir.join("prices.toml"))?;
        let prices: PriceConfig = prices_raw.into();
        let shops_raw = load_toml::<ShopsConfigRaw>(&dir.join("shops.toml"))?;
        let shops: ShopsConfig = shops_raw.into();
        let collectibles_raw = load_toml::<CollectiblesConfigRaw>(&dir.join("collectibles.toml"))?;
        let collectibles: CollectiblesConfig = collectibles_raw.into();
        let plants = load_toml::<PlantsConfig>(&dir.join("plants.toml"))?;
        let clans = load_toml::<ClansConfig>(&dir.join("clans.toml"))?;
        let upgrader_raw = load_toml::<UpgraderConfigRaw>(&dir.join("upgrader.toml"))?;
        let upgrader: UpgraderConfig = upgrader_raw.into();

        Ok(Self {
            server,
            game,
            prices,
            shops,
            collectibles,
            plants,
            clans,
            upgrader,
        })
    }
}

fn load_toml<T>(path: &Path) -> Result<T, ConfigError>
where
    T: for<'de> Deserialize<'de>,
{
    let path_str = path.display().to_string();
    let content = std::fs::read_to_string(path).map_err(|e| ConfigError::IoError {
        path: path_str.clone(),
        source: e,
    })?;

    toml::from_str(&content).map_err(|e| ConfigError::ParseError {
        path: path_str,
        source: e,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        // This test requires config files to exist
        let config = GameConfig::load("config");
        assert!(config.is_ok(), "Failed to load config: {:?}", config.err());

        let config = config.unwrap();
        
        // Test game config
        assert_eq!(config.game.defaults.spawn_room, 32);
        assert_eq!(config.game.defaults.spawn_x, 385);
        assert_eq!(config.game.defaults.spawn_y, 71);
        
        // Test BBS categories count
        assert_eq!(config.game.bbs.categories.len(), 6);
        
        // Test prices
        assert!(config.prices.get_item_price(1).is_some());
        
        // Test collectibles
        assert!(config.collectibles.get_room(100).is_some() || config.collectibles.get_room(33).is_some());
    }
}
