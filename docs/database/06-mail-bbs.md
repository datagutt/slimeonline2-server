# Mail & BBS Tables Reference

**See:** [`01-schema-overview.md`](01-schema-overview.md) for complete schema.

## Mail Table

```sql
CREATE TABLE mail (
    id SERIAL PRIMARY KEY,
    from_character_id INTEGER REFERENCES characters(id),
    to_character_id INTEGER REFERENCES characters(id),
    subject VARCHAR(50),
    message TEXT,
    item_id SMALLINT,
    item_quantity SMALLINT,
    points INTEGER,
    is_read BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT NOW()
);
```

## BBS Posts Table

```sql
CREATE TABLE bbs_posts (
    id SERIAL PRIMARY KEY,
    character_id INTEGER REFERENCES characters(id),
    title VARCHAR(50),
    content TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    is_reported BOOLEAN DEFAULT FALSE
);
```

See [`01-schema-overview.md`](01-schema-overview.md) for full details.
