# Shop & Economy System

**See:** [`../protocol/04-message-catalog.md`](../protocol/04-message-catalog.md) - Shop section

## Shop Messages

- MSG_SHOP_BUY (28) - Client requests to buy item
- MSG_SHOP_BUY_FAIL (29) - Purchase failed
- MSG_SHOP_STOCK (30) - Stock update (item sold out)
- MSG_SELL_REQ_PRICES (53) - Client requests sell prices
- MSG_SELL (54) - Client sells items

## Shop Buy Flow

1. Client sends MSG_SHOP_BUY with `buy_id` (shop slot index)
2. Server reads shop config from room file
3. Server validates: stock > 0, player has enough points, player has free inventory slot
4. Server deducts points, adds item to inventory, decrements stock
5. Server responds with MSG_SHOP_BUY containing category, slot, item_id, price

### MSG_SHOP_BUY Response Format
```rust
writeushort(MSG_SHOP_BUY);
writebyte(category);     // 1=outfit, 2=item, 3=acs, 4=tool
writebyte(slot);         // inventory slot item was placed in
writeushort(item_id);    // the item ID
writeushort(price);      // price paid
```

### MSG_SHOP_BUY_FAIL Reasons
- **1**: Out of stock
- **2**: Not enough points

## Sell System

Items sell for **1/3 of their buy price** (rounded).

```rust
sell_price = round(buy_price / 3)
```

If selling would exceed MAX_POINTS, overflow goes to bank automatically.

## Original Server Price Files

The original server stores prices in `.prc` files in `srvr_prices/`:

### items.prc (Item Prices)
```ini
[Prices]
1=10      # Fly Wing
2=30      # Smoke Bomb
3=15      # Apple Bomb
4=15      # Bubble Wand
5=50      # Points Bag [50]
6=200     # Points Bag [200]
7=500     # Points Bag [500]
8=10      # Chicken Mine
9=150     # Simple Seed
10=250    # Fairy
11=100    # Blue Pinwheel
12=250    # Red Pinwheel
13=500    # Glow Pinwheel
14=1500   # Rockman Sound
15=1500   # Kirby Sound
16=1500   # Link Sound
17=1500   # Pipe Sound
18=1500   # DK Sound
19=1500   # Metroid Sound
20=50     # Red Mushroom
21=300    # Tailphire
22=30     # Magmanis
23=150    # Bright Drink
24=150    # Blue Seed
25=450    # Juicy Bango
26=900    # Weak Cannon Kit
27=15     # Red Gum
28=15     # Orange Gum
29=15     # Green Gum
30=15     # Blue Gum
31=15     # Pink Gum
32=15     # White Gum
33=3      # Lucky Coin
34=30     # Bunny Soda
35=30     # Slime Soda
36=30     # Penguin Soda
37=3      # Speed Soda
38=3      # Jump Soda
39=60     # Slimeium variations (39-45)
40=60
41=60
42=60
43=60
44=60
45=60
46=15     # Screw
47=6      # Rusty Screw
48=3      # Bug Leg
49=75     # Weird Coin
50=30     # Firestone
51=3      # Proof of Nature
52=3      # Proof of Earth
53=3      # Proof of Water
54=3      # Proof of Fire
55=3      # Proof of Stone
56=3      # Proof of Wind
57=150    # Blazing Bubble
58=150    # Squishy Mushroom
59=3      # Stinky Mushroom
60=30     # Bell Twig
61=300    # Irrlicht
# 62-100 are set to 65000 (non-purchasable/special items)
```

### outfits.prc (Outfit Prices)
```ini
[Prices]
1=50
2=200
3=1000
4=1000
5=300
6=2000
7=1500
8=3000
9=1000
10=800
# ... continues to 100
# Notable: outfit 92=1337 (easter egg)
```

### acs.prc (Accessory Prices)
```ini
[Prices]
1=100
2=2000
3=500
# ... prices range from 200 to 10000
```

### tools.prc (Tool Prices)
```ini
[Prices]
1=500     # Rusty Pickaxe
2=1000    # Pickaxe
```

### mail.prc (Mail Paper Prices)
```ini
[Paper]
total=3
0=25      # Basic paper
1=40      # Medium paper
2=100     # Fancy paper
3=50
```

## Rust Server Implementation

Our Rust server uses **config files as the single source of truth** for static game data. The database only stores runtime state (current stock levels).

### prices.toml
```toml
[items]
1 = { name = "Fly Wing", price = 10 }
2 = { name = "Smoke Bomb", price = 30 }
# ... all items

[outfits]
1 = 50
2 = 200
# ... all outfits

[accessories]
1 = 100
# ... all accessories

[tools]
1 = { name = "Rusty Pickaxe", price = 500 }
2 = { name = "Pickaxe", price = 1000 }
```

### shops.toml
```toml
[room.44]
slots = [
    { cat = 1, item = 15, stock = 10 },
    { cat = 1, item = 38, stock = 2 },
]

[room.45]
slots = [
    { cat = 2, item = 1, stock = 0 },  # 0 = unlimited
]
```

### Database (runtime state only)
```sql
CREATE TABLE shop_stock (
    room_id INTEGER NOT NULL,
    slot_id INTEGER NOT NULL,
    current_stock INTEGER,
    last_restock TEXT,
    PRIMARY KEY (room_id, slot_id)
);
```

## Bank Operations

### MSG_BANK_PROCESS (45)

**Subtypes:**
- **1**: Deposit - move points from wallet to bank
- **2**: Withdraw - move points from bank to wallet
- **3**: Transfer - send points to another player's bank

**Response Format:**
```rust
writeushort(MSG_BANK_PROCESS);
writebyte(operation);    // 1=deposit, 2=withdraw, 3=transfer, 4=transfer_failed
writeuint(new_points);   // wallet balance
writeuint(new_bank);     // bank balance (not sent for transfer)
```

### Currency Limits
- MAX_POINTS (wallet): Server should cap at reasonable value
- Bank: No explicit limit in original, but should be capped

## Validation

- Verify player has enough points before purchase
- Verify shop actually exists in room
- Verify item is in stock
- Verify player has free inventory slot for that category
- Prevent overflow when selling (excess goes to bank)

See [`../security/02-server-validation.md`](../security/02-server-validation.md).
