# Characters Table Reference

**See:** [`01-schema-overview.md`](01-schema-overview.md) for complete schema.

## Original Server Account File Format (.soa)

The original server stores accounts in `.soa` INI files:

```ini
[Setup]
Name=playername           # Lowercase
Caps Name=PlayerName      # Original case
Pass=password             # Plaintext (we use bcrypt)
Register Date=01.07.2009 - 12:30
Register IP=127.0.0.1
Register MAC=00:00:00:00:00:00
Last IP=127.0.0.1
Last MAC=00:00:00:00:00:00
ID=123                    # Player ID
Hacks=0                   # Hack detection counter
Banned=0                  # 0=not banned, 1=banned

[Status]
signature=1               # Has signature permission
sig_bg=1                  # Signature background
x=385                     # Position X
y=71                      # Position Y
room=32                   # Current room
points=0                  # Wallet
bank=0                    # Bank balance
outfit=1                  # Equipped outfit
acs1=0                    # Accessory slot 1
acs2=0                    # Accessory slot 2
trees=0                   # Trees planted stat
objectsbuilt=0            # Objects built stat
clan=0                    # Clan ID (0=none)

[Quest]
questid=0                 # Current quest ID
queststep=0               # Quest progress

[Mailpaper-Unlocked]
0=1
1=1
2=1

[Emotes]
1=5
2=4
3=6
4=8
5=7

[Outfits]
1=0 through 9=0           # 9 outfit slots

[Acs]
1=0 through 9=0           # 9 accessory slots

[Items]
1=1                       # Default: Fly Wing
2=1                       # Default: Smoke Bomb  
3=1                       # Default: Apple Bomb
4=24                      # Default: Blue Seed
5=0 through 9=0

[Tools]
1=0 through 9=0           # 9 tool slots

[House]
Points=0                  # House points

[One-Times]
# Tracks collected one-time items

[Storage-Outfits]
1=0 through 180=0         # 180 storage slots

[Storage-Items]
1=0 through 180=0

[Storage-Acs]
1=0 through 180=0

[Storage-Tools]
1=0 through 180=0

[Mailbox]
1sender=Slime Team
1date=01.07.2009
1text=Welcome to Slime Online!#We hope you enjoy#your stay.##The Team
1points=50
1presentcat=0
1presentid=0
1read=0
1paper=0
1font=0
```

## Database Schema

```sql
CREATE TABLE characters (
    id SERIAL PRIMARY KEY,
    account_id INTEGER UNIQUE REFERENCES accounts(id),
    username VARCHAR(10) UNIQUE NOT NULL,
    
    -- Position (defaults from original server)
    x SMALLINT DEFAULT 385,
    y SMALLINT DEFAULT 71,
    room_id SMALLINT DEFAULT 32,
    
    -- Appearance
    body_id SMALLINT DEFAULT 1,
    acs1_id SMALLINT DEFAULT 0,
    acs2_id SMALLINT DEFAULT 0,
    
    -- Currency
    points INTEGER DEFAULT 0,
    bank_balance INTEGER DEFAULT 0,
    
    -- Stats
    trees_planted SMALLINT DEFAULT 0,
    objects_built SMALLINT DEFAULT 0,
    
    -- Quest
    quest_id SMALLINT DEFAULT 0,
    quest_step SMALLINT DEFAULT 0,
    
    -- Permissions
    signature SMALLINT DEFAULT 1,
    signature_bg SMALLINT DEFAULT 1,
    
    -- Clan
    clan_id INTEGER REFERENCES clans(id),
    
    -- Timestamps
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);
```

## Default New Account Values

From `[Acc_Sample].soa`:
- Position: (385, 71) in room 32
- Outfit: 1 (default)
- Starting items: Fly Wing, Smoke Bomb, Apple Bomb, Blue Seed
- Starting emotes: [5, 4, 6, 8, 7]
- Welcome mail with 50 points attached

## Common Operations

**Load Character:**
```sql
SELECT * FROM characters WHERE account_id = $1;
```

**Update Position:**
```sql
UPDATE characters SET x = $1, y = $2, room_id = $3, updated_at = NOW() WHERE id = $4;
```

**Add Points (with overflow to bank):**
```sql
-- In Rust, check if wallet would overflow MAX_POINTS
-- If so, put excess in bank automatically
```

See [`01-schema-overview.md`](01-schema-overview.md) for full details.
