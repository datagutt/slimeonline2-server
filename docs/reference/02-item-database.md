# Item Database

**See:** [`01-constants.md`](01-constants.md) for item ID constants.

## Item Categories

Items are organized into categories, each with their own inventory slots and price files.

### Category IDs
- **1**: Outfits (body appearances)
- **2**: Items (consumables, materials, etc.)
- **3**: Accessories (worn items)
- **4**: Tools (equippable tools)

## Items (Category 2)

### Consumables & Usables (1-26)

| ID | Name | Price | Effect |
|----|------|-------|--------|
| 1 | Fly Wing | 10 | Teleport to spawn |
| 2 | Smoke Bomb | 30 | Visual effect |
| 3 | Apple Bomb | 15 | Throwable |
| 4 | Bubble Wand | 15 | Creates bubbles |
| 5 | Points Bag [50] | 50 | Gives 50 points |
| 6 | Points Bag [200] | 200 | Gives 200 points |
| 7 | Points Bag [500] | 500 | Gives 500 points |
| 8 | Chicken Mine | 10 | Placeable trap |
| 9 | Simple Seed | 150 | Plant a tree |
| 10 | Fairy | 250 | Add to planted tree (+10% fruit chance) |
| 11 | Blue Pinwheel | 100 | Add to planted tree |
| 12 | Red Pinwheel | 250 | Add to planted tree |
| 13 | Glow Pinwheel | 500 | Add to planted tree |
| 14 | Rockman Sound | 1500 | Sound pack |
| 15 | Kirby Sound | 1500 | Sound pack |
| 16 | Link Sound | 1500 | Sound pack |
| 17 | Pipe Sound | 1500 | Sound pack |
| 18 | DK Sound | 1500 | Sound pack |
| 19 | Metroid Sound | 1500 | Sound pack |
| 20 | Red Mushroom | 50 | Evolving collectible |
| 21 | Tailphire | 300 | Quest/crafting item |
| 22 | Magmanis | 30 | Collectible (lava areas) |
| 23 | Bright Drink | 150 | Invisibility effect |
| 24 | Blue Seed | 150 | Plant a special tree |
| 25 | Juicy Bango | 450 | Fruit item |
| 26 | Weak Cannon Kit | 900 | Build a cannon |

### Gums (27-32)

| ID | Name | Price | Effect |
|----|------|-------|--------|
| 27 | Red Gum | 15 | Color effect |
| 28 | Orange Gum | 15 | Color effect |
| 29 | Green Gum | 15 | Color effect |
| 30 | Blue Gum | 15 | Color effect |
| 31 | Pink Gum | 15 | Color effect |
| 32 | White Gum | 15 | Color effect |

### Currency & Sodas (33-38)

| ID | Name | Price | Effect |
|----|------|-------|--------|
| 33 | Lucky Coin | 3 | Currency item |
| 34 | Bunny Soda | 30 | Transform into bunny |
| 35 | Slime Soda | 30 | Transform into slime |
| 36 | Penguin Soda | 30 | Transform into penguin |
| 37 | Speed Soda | 3 | Speed boost |
| 38 | Jump Soda | 3 | Jump boost |

### Slimeium Materials (39-45)

| ID | Name | Price | Use |
|----|------|-------|-----|
| 39 | Sleenmium | 60 | Crafting material |
| 40 | Sledmium | 60 | Crafting material |
| 41 | Sluemium | 60 | Crafting material |
| 42 | Slinkmium | 60 | Crafting material |
| 43 | Slelloymium | 60 | Crafting material |
| 44 | Slaymium | 60 | Crafting material |
| 45 | Slackmium | 60 | Crafting material |

### Misc Materials (46-50)

| ID | Name | Price | Notes |
|----|------|-------|-------|
| 46 | Screw | 15 | Material |
| 47 | Rusty Screw | 6 | Material |
| 48 | Bug Leg | 3 | Drop item |
| 49 | Weird Coin | 75 | Rare material |
| 50 | Firestone | 30 | Quest item |

### Proof Items (51-56)

Required for clan creation and other features.

| ID | Name | Price | Use |
|----|------|-------|-----|
| 51 | Proof of Nature | 3 | Clan creation |
| 52 | Proof of Earth | 3 | Clan creation |
| 53 | Proof of Water | 3 | Quest item |
| 54 | Proof of Fire | 3 | Quest item |
| 55 | Proof of Stone | 3 | Quest item |
| 56 | Proof of Wind | 3 | Quest item |

### Collectible Items (57-61)

| ID | Name | Price | Where Found |
|----|------|-------|-------------|
| 57 | Blazing Bubble | 150 | Volcanic areas |
| 58 | Squishy Mushroom | 150 | Evolved from Red Mushroom |
| 59 | Stinky Mushroom | 3 | Evolved from Squishy Mushroom |
| 60 | Bell Twig | 30 | Forest areas |
| 61 | Irrlicht | 300 | Rare collectible |

### Special/Non-Purchasable (62+)

Items 62-100 have a price of 65000, indicating they are special items not normally purchasable from shops (quest rewards, rare drops, etc.).

## Discardable Items

Only the following item IDs can be discarded (dropped on ground):
1-61 (all standard items listed above)

Items 62+ cannot be discarded.

## Tools (Category 4)

| ID | Name | Price | Use |
|----|------|-------|-----|
| 1 | Rusty Pickaxe | 500 | Mining (lower tier) |
| 2 | Pickaxe | 1000 | Mining (higher tier) |

## Outfits (Category 1)

100 outfits available. Prices range from 50 to 5000 points.

Notable:
- Outfit 1: Default (50 points)
- Outfit 92: Price is 1337 (easter egg)
- Outfit 96: Magma Dungeon race reward

## Accessories (Category 3)

101 accessories available. Prices range from 100 to 10000 points.

## Rust Server Implementation

Item data is stored in the database for easy modification:

```sql
CREATE TABLE items (
    id SMALLINT PRIMARY KEY,
    category SMALLINT NOT NULL,
    name VARCHAR(50) NOT NULL,
    buy_price INTEGER NOT NULL,
    sell_price INTEGER GENERATED ALWAYS AS (buy_price / 3) STORED,
    discardable BOOLEAN DEFAULT TRUE,
    description TEXT
);
```

Or via configuration:
```toml
# items.toml
[[item]]
id = 1
name = "Fly Wing"
price = 10
discardable = true

[[item]]
id = 62
name = "Special Item"
price = 65000
discardable = false
```

## See Also

- [`../protocol/04-message-catalog.md`](../protocol/04-message-catalog.md) for item usage messages
- [`01-constants.md`](01-constants.md) for all item IDs as Rust constants
- [`../game-systems/04-shop-economy.md`](../game-systems/04-shop-economy.md) for shop system
