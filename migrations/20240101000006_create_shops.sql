-- Create shop_items table for dynamic shop inventory
-- Each shop item is defined by room, slot position, category, item_id, price, and stock
CREATE TABLE IF NOT EXISTS shop_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id INTEGER NOT NULL,           -- Room where the shop is located
    slot_id INTEGER NOT NULL,           -- Shop slot position (1-based, matches obj_shop_call_item.shop_id)
    category INTEGER NOT NULL,          -- 1=Outfit, 2=Item, 3=Accessory, 4=Tool
    item_id INTEGER NOT NULL,           -- ID of the item/outfit/accessory/tool
    price INTEGER NOT NULL,             -- Price in points
    stock INTEGER NOT NULL DEFAULT 1,   -- 0=sold out, 1=available (can be per-player in future)
    is_limited INTEGER NOT NULL DEFAULT 0, -- If 1, stock decrements on purchase (one-time items)
    
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    UNIQUE(room_id, slot_id)            -- Only one item per slot per room
);

-- Index for fast room lookup
CREATE INDEX IF NOT EXISTS idx_shop_items_room ON shop_items(room_id);

-- =============================================================================
-- SEED DATA
-- =============================================================================
-- Shop items are determined by the server. The client only provides shop slot
-- positions in each room via obj_shop_call_item objects with a shop_id.
-- 
-- Item IDs from db_items.gml (items/database.rs)
-- Outfit IDs from db_outfits.gml (1=Green Slime, 2=Holed Slime, etc.)
-- Accessory IDs from db_acs.gml (1=AFK Sign, 2=Fefnir Helmet, etc.)
-- =============================================================================

-- Room 44: New City Outfits (5 slots) - Beginner outfits
-- Outfits: 1=Green Slime, 2=Holed Slime, 3=Carnage Slime, 4=Shadow Slime, 5=Glowing Slime, 6=Headcrab, 7=Fire, 8=Shark, 9=Sprinkled
INSERT OR IGNORE INTO shop_items (room_id, slot_id, category, item_id, price, stock) VALUES
    (44, 1, 1, 2, 200, 1),    -- Holed Slime, 200 points
    (44, 2, 1, 3, 350, 1),    -- Carnage Slime, 350 points
    (44, 3, 1, 4, 500, 1),    -- Shadow Slime, 500 points
    (44, 4, 1, 5, 750, 1),    -- Glowing Slime, 750 points
    (44, 5, 1, 9, 1000, 1);   -- Sprinkled Slime, 1000 points

-- Room 45: New City Accessories (5 slots) - Beginner accessories
-- Accessories: 1=AFK Sign, 2=Fefnir Helmet, 3=Angel Stuff, 4=Dinosaur Stuff, 5=Fluffy ears, 6=Egg-Shell, 7=Bag
INSERT OR IGNORE INTO shop_items (room_id, slot_id, category, item_id, price, stock) VALUES
    (45, 1, 3, 1, 100, 1),    -- AFK Sign, 100 points
    (45, 2, 3, 3, 300, 1),    -- Angel Stuff, 300 points
    (45, 3, 3, 5, 400, 1),    -- Fluffy ears, 400 points
    (45, 4, 3, 6, 500, 1),    -- Egg-Shell, 500 points
    (45, 5, 3, 7, 250, 1);    -- Bag, 250 points

-- Room 46: New City Items (6 slots) - Basic consumables
-- Items: 1=Warp-Wing, 2=Smokebomb, 3=Applebomb, 4=Bubbles, 9=Simple Seed, 27=Red Gum, 37=Speed Soda, 38=Jump Soda
INSERT OR IGNORE INTO shop_items (room_id, slot_id, category, item_id, price, stock) VALUES
    (46, 1, 2, 1, 100, 1),    -- Warp-Wing, 100 points
    (46, 2, 2, 2, 50, 1),     -- Smokebomb, 50 points
    (46, 3, 2, 4, 75, 1),     -- Bubbles, 75 points
    (46, 4, 2, 9, 200, 1),    -- Simple Seed, 200 points
    (46, 5, 2, 27, 25, 1),    -- Red Gum, 25 points
    (46, 6, 2, 37, 150, 1);   -- Speed Soda, 150 points

-- Room 123: Old City Outfits (5 slots) - Advanced outfits
-- Outfits: 10=Wireframe, 11=Albino, 12=Elephant, 15=Gear, 16=8bit, 20=Homer
INSERT OR IGNORE INTO shop_items (room_id, slot_id, category, item_id, price, stock) VALUES
    (123, 1, 1, 10, 2000, 1),  -- Wireframe Slime, 2000 points
    (123, 2, 1, 11, 2500, 1),  -- Albino Slime, 2500 points
    (123, 3, 1, 15, 3500, 1),  -- Gear Slime, 3500 points
    (123, 4, 1, 16, 4000, 1),  -- 8bit Slime, 4000 points
    (123, 5, 1, 20, 5000, 1);  -- Homer Slime, 5000 points

-- Room 124: Old City Accessories (5 slots) - Advanced accessories
-- Accessories: 9=Gasmask, 10=Snow Hat, 11=Predator Mask, 13=Headphones, 18=Knight Helmet
INSERT OR IGNORE INTO shop_items (room_id, slot_id, category, item_id, price, stock) VALUES
    (124, 1, 3, 9, 1500, 1),   -- Gasmask, 1500 points
    (124, 2, 3, 10, 2000, 1),  -- Snow Hat, 2000 points
    (124, 3, 3, 11, 3000, 1),  -- Predator Mask, 3000 points
    (124, 4, 3, 13, 2500, 1),  -- Headphones, 2500 points
    (124, 5, 3, 18, 4000, 1);  -- Knight Helmet, 4000 points

-- Room 125: Old City Items (5 slots) - Advanced consumables
-- Items: 3=Applebomb, 8=Chicken Mine, 10=Fairy, 11=Blue Pinwheel, 38=Jump Soda, 26=Weak Cannon Kit
INSERT OR IGNORE INTO shop_items (room_id, slot_id, category, item_id, price, stock) VALUES
    (125, 1, 2, 3, 100, 1),    -- Applebomb, 100 points
    (125, 2, 2, 8, 500, 1),    -- Chicken Mine, 500 points
    (125, 3, 2, 10, 300, 1),   -- Fairy, 300 points
    (125, 4, 2, 11, 400, 1),   -- Blue Pinwheel, 400 points
    (125, 5, 2, 38, 200, 1);   -- Jump Soda, 200 points
