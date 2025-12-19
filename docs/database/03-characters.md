# Characters Table Reference

**See:** [`01-schema-overview.md`](01-schema-overview.md) for complete schema.

## Quick Reference

```sql
CREATE TABLE characters (
    id SERIAL PRIMARY KEY,
    account_id INTEGER UNIQUE REFERENCES accounts(id),
    username VARCHAR(20) UNIQUE NOT NULL,
    x SMALLINT DEFAULT 100,
    y SMALLINT DEFAULT 120,
    room_id SMALLINT DEFAULT 1,
    body_id SMALLINT DEFAULT 1,
    acs1_id SMALLINT DEFAULT 0,
    acs2_id SMALLINT DEFAULT 0,
    points INTEGER DEFAULT 0,
    bank_balance INTEGER DEFAULT 0,
    trees_planted SMALLINT DEFAULT 0,
    objects_built SMALLINT DEFAULT 0,
    quest_id SMALLINT DEFAULT 0,
    quest_step SMALLINT DEFAULT 0,
    quest_var SMALLINT DEFAULT 0,
    has_signature BOOLEAN DEFAULT FALSE,
    is_moderator BOOLEAN DEFAULT FALSE,
    clan_id INTEGER REFERENCES clans(id),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);
```

## Common Operations

**Load Character:**
```sql
SELECT * FROM characters WHERE account_id = $1;
```

**Update Position:**
```sql
UPDATE characters SET x = $1, y = $2, room_id = $3, updated_at = NOW() WHERE id = $4;
```

**Add Points:**
```sql
UPDATE characters SET points = points + $1 WHERE id = $2 AND points + $1 >= 0;
```

See [`01-schema-overview.md`](01-schema-overview.md) for full details.
