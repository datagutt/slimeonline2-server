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
    pub game: GameRulesConfig,
    pub prices: PriceConfig,
    pub shops: ShopsConfig,
    pub collectibles: CollectiblesConfig,
    pub plants: PlantsConfig,
    pub clans: ClansConfig,
}

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

#[derive(Debug, Clone, Deserialize)]
pub struct PriceConfig {
    pub items: HashMap<u16, ItemPriceEntry>,
    pub outfits: HashMap<u16, u32>,
    pub accessories: HashMap<u16, u32>,
    pub tools: HashMap<u8, ToolPriceEntry>,
    pub mail_paper: HashMap<u8, u16>,
    #[serde(default)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct ShopsConfig {
    #[serde(flatten)]
    pub rooms: HashMap<String, RoomShopConfig>,
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

impl ShopsConfig {
    /// Get shop config for a room by room ID
    pub fn get_room(&self, room_id: u16) -> Option<&RoomShopConfig> {
        let key = format!("room.{}", room_id);
        self.rooms.get(&key)
    }
}

// =============================================================================
// collectibles.toml
// =============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct CollectiblesConfig {
    #[serde(default)]
    pub evolving: HashMap<u16, EvolvingConfig>,
    #[serde(flatten)]
    pub rooms: HashMap<String, RoomCollectiblesConfig>,
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

impl CollectiblesConfig {
    /// Get collectible spawns for a room by room ID
    pub fn get_room(&self, room_id: u16) -> Option<&RoomCollectiblesConfig> {
        let key = format!("room.{}", room_id);
        self.rooms.get(&key)
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

        let game = load_toml::<GameRulesConfig>(&dir.join("game.toml"))?;
        let prices = load_toml::<PriceConfig>(&dir.join("prices.toml"))?;
        let shops = load_toml::<ShopsConfig>(&dir.join("shops.toml"))?;
        let collectibles = load_toml::<CollectiblesConfig>(&dir.join("collectibles.toml"))?;
        let plants = load_toml::<PlantsConfig>(&dir.join("plants.toml"))?;
        let clans = load_toml::<ClansConfig>(&dir.join("clans.toml"))?;

        Ok(Self {
            game,
            prices,
            shops,
            collectibles,
            plants,
            clans,
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
