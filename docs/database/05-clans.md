# Clans Table Reference

**See:** [`01-schema-overview.md`](01-schema-overview.md) for complete schema.

## Schema

```sql
CREATE TABLE clans (
    id SERIAL PRIMARY KEY,
    name VARCHAR(20) UNIQUE NOT NULL,
    leader_id INTEGER REFERENCES characters(id),
    description TEXT,
    created_at TIMESTAMP DEFAULT NOW()
);
```

## Common Operations

**Create Clan:**
```sql
INSERT INTO clans (name, leader_id, description)
VALUES ($1, $2, $3)
RETURNING id;
```

**Get Clan Members:**
```sql
SELECT id, username FROM characters WHERE clan_id = $1;
```

**Join Clan:**
```sql
UPDATE characters SET clan_id = $1 WHERE id = $2 AND clan_id IS NULL;
```

**Leave Clan:**
```sql
UPDATE characters SET clan_id = NULL WHERE id = $1;
```

See [`01-schema-overview.md`](01-schema-overview.md) for full details.
