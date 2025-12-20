//! Item database definitions from db_items.gml
//!
//! All item data is sourced from the client's db_items.gml file.

/// Item type for special handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
    WarpWing,       // 1 - Teleport to save point
    Smokebomb,      // 2 - Visual effect
    Applebomb,      // 3 - Visual effect
    Bubbles,        // 4 - Visual effect
    Slimebag50,     // 5 - Gives 50 points
    Slimebag200,    // 6 - Gives 200 points
    Slimebag500,    // 7 - Gives 500 points
    ChickenMine,    // 8 - Place a mine
    SimpleSeed,     // 9 - Plant a tree
    Fairy,          // 10 - Use on tree (+10% fruit chance)
    BluePinwheel,   // 11 - Use on tree (-20% grow time)
    RedPinwheel,    // 12 - Use on tree (-35% grow time)
    GlowPinwheel,   // 13 - Use on tree (-50% grow time)
    Soundmaker,     // 14-19 - Play sound
    Material,       // 20-25, 39-50, 57-61 - Collectible/crafting materials
    WeakCannonKit,  // 26 - Build a weak cannon
    Gum,            // 27-32 - Place gum on ground
    LuckyCoin,      // 33 - Gambling item
    Soda,           // 34-36 - Decorative sodas
    SpeedSoda,      // 37 - Temporary speed boost
    JumpSoda,       // 38 - Temporary jump boost
    BlueSeed,       // 24 - Plant a special tree
    ProofStone,     // 51-56 - Proof of conquest items
    Generic,        // Other items
}

/// Item information
#[derive(Debug, Clone)]
pub struct ItemInfo {
    pub id: u16,
    pub name: &'static str,
    pub item_type: ItemType,
}

/// Get item information by ID (from db_items.gml)
pub fn get_item_info(item_id: u16) -> Option<ItemInfo> {
    match item_id {
        1 => Some(ItemInfo { id: 1, name: "Warp-Wing", item_type: ItemType::WarpWing }),
        2 => Some(ItemInfo { id: 2, name: "Smokebomb", item_type: ItemType::Smokebomb }),
        3 => Some(ItemInfo { id: 3, name: "Applebomb", item_type: ItemType::Applebomb }),
        4 => Some(ItemInfo { id: 4, name: "Bubbles", item_type: ItemType::Bubbles }),
        5 => Some(ItemInfo { id: 5, name: "Slimebag [50]", item_type: ItemType::Slimebag50 }),
        6 => Some(ItemInfo { id: 6, name: "Slimebag [200]", item_type: ItemType::Slimebag200 }),
        7 => Some(ItemInfo { id: 7, name: "Slimebag [500]", item_type: ItemType::Slimebag500 }),
        8 => Some(ItemInfo { id: 8, name: "Chicken Mine", item_type: ItemType::ChickenMine }),
        9 => Some(ItemInfo { id: 9, name: "Simple Seed", item_type: ItemType::SimpleSeed }),
        10 => Some(ItemInfo { id: 10, name: "Fairy", item_type: ItemType::Fairy }),
        11 => Some(ItemInfo { id: 11, name: "Blue Pinwheel", item_type: ItemType::BluePinwheel }),
        12 => Some(ItemInfo { id: 12, name: "Red Pinwheel", item_type: ItemType::RedPinwheel }),
        13 => Some(ItemInfo { id: 13, name: "Glow Pinwheel", item_type: ItemType::GlowPinwheel }),
        14 => Some(ItemInfo { id: 14, name: "Rockman Sound", item_type: ItemType::Soundmaker }),
        15 => Some(ItemInfo { id: 15, name: "Kirby Sound", item_type: ItemType::Soundmaker }),
        16 => Some(ItemInfo { id: 16, name: "Link Sound", item_type: ItemType::Soundmaker }),
        17 => Some(ItemInfo { id: 17, name: "Pipe Sound", item_type: ItemType::Soundmaker }),
        18 => Some(ItemInfo { id: 18, name: "DK Sound", item_type: ItemType::Soundmaker }),
        19 => Some(ItemInfo { id: 19, name: "Metroid Sound", item_type: ItemType::Soundmaker }),
        20 => Some(ItemInfo { id: 20, name: "Red Mushroom", item_type: ItemType::Material }),
        21 => Some(ItemInfo { id: 21, name: "Tailphire", item_type: ItemType::Material }),
        22 => Some(ItemInfo { id: 22, name: "Magmanis", item_type: ItemType::Material }),
        23 => Some(ItemInfo { id: 23, name: "Bright Drink", item_type: ItemType::Generic }),
        24 => Some(ItemInfo { id: 24, name: "Blue Seed", item_type: ItemType::BlueSeed }),
        25 => Some(ItemInfo { id: 25, name: "Juicy Bango", item_type: ItemType::Material }),
        26 => Some(ItemInfo { id: 26, name: "Weak Cannon Kit", item_type: ItemType::WeakCannonKit }),
        27 => Some(ItemInfo { id: 27, name: "Red Gum", item_type: ItemType::Gum }),
        28 => Some(ItemInfo { id: 28, name: "Orange Gum", item_type: ItemType::Gum }),
        29 => Some(ItemInfo { id: 29, name: "Green Gum", item_type: ItemType::Gum }),
        30 => Some(ItemInfo { id: 30, name: "Blue Gum", item_type: ItemType::Gum }),
        31 => Some(ItemInfo { id: 31, name: "Pink Gum", item_type: ItemType::Gum }),
        32 => Some(ItemInfo { id: 32, name: "White Gum", item_type: ItemType::Gum }),
        33 => Some(ItemInfo { id: 33, name: "Lucky Coin", item_type: ItemType::LuckyCoin }),
        34 => Some(ItemInfo { id: 34, name: "Bunny Soda", item_type: ItemType::Soda }),
        35 => Some(ItemInfo { id: 35, name: "Slime Soda", item_type: ItemType::Soda }),
        36 => Some(ItemInfo { id: 36, name: "Penguin Soda", item_type: ItemType::Soda }),
        37 => Some(ItemInfo { id: 37, name: "Speed Soda", item_type: ItemType::SpeedSoda }),
        38 => Some(ItemInfo { id: 38, name: "Jump Soda", item_type: ItemType::JumpSoda }),
        39 => Some(ItemInfo { id: 39, name: "Sleenmium", item_type: ItemType::Material }),
        40 => Some(ItemInfo { id: 40, name: "Sledmium", item_type: ItemType::Material }),
        41 => Some(ItemInfo { id: 41, name: "Sluemium", item_type: ItemType::Material }),
        42 => Some(ItemInfo { id: 42, name: "Slinkmium", item_type: ItemType::Material }),
        43 => Some(ItemInfo { id: 43, name: "Slelloymium", item_type: ItemType::Material }),
        44 => Some(ItemInfo { id: 44, name: "Slaymium", item_type: ItemType::Material }),
        45 => Some(ItemInfo { id: 45, name: "Slackmium", item_type: ItemType::Material }),
        46 => Some(ItemInfo { id: 46, name: "Screw", item_type: ItemType::Material }),
        47 => Some(ItemInfo { id: 47, name: "Rusty Screw", item_type: ItemType::Material }),
        48 => Some(ItemInfo { id: 48, name: "Bug Leg", item_type: ItemType::Material }),
        49 => Some(ItemInfo { id: 49, name: "Weird Coin", item_type: ItemType::Material }),
        50 => Some(ItemInfo { id: 50, name: "Firestone", item_type: ItemType::Material }),
        51 => Some(ItemInfo { id: 51, name: "Proof of Nature", item_type: ItemType::ProofStone }),
        52 => Some(ItemInfo { id: 52, name: "Proof of Earth", item_type: ItemType::ProofStone }),
        53 => Some(ItemInfo { id: 53, name: "Proof of Water", item_type: ItemType::ProofStone }),
        54 => Some(ItemInfo { id: 54, name: "Proof of Fire", item_type: ItemType::ProofStone }),
        55 => Some(ItemInfo { id: 55, name: "Proof of Stone", item_type: ItemType::ProofStone }),
        56 => Some(ItemInfo { id: 56, name: "Proof of Wind", item_type: ItemType::ProofStone }),
        57 => Some(ItemInfo { id: 57, name: "Blazing Bubble", item_type: ItemType::Material }),
        58 => Some(ItemInfo { id: 58, name: "Squishy Mushroom", item_type: ItemType::Material }),
        59 => Some(ItemInfo { id: 59, name: "Stinky Mushroom", item_type: ItemType::Material }),
        60 => Some(ItemInfo { id: 60, name: "Bell Twig", item_type: ItemType::Material }),
        61 => Some(ItemInfo { id: 61, name: "Irrlicht", item_type: ItemType::Material }),
        _ => None,
    }
}

/// Check if an item can be discarded (from item_discard_slot.gml)
/// Note: Bug Leg (48) is NOT in the discard list
pub fn can_discard_item(item_id: u16) -> bool {
    matches!(item_id, 1..=47 | 49..=61)
}
