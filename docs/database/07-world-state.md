# World State Tables Reference

**See:** [`01-schema-overview.md`](01-schema-overview.md) for complete schema.

## Collectibles

```sql
CREATE TABLE collectibles (
    room_id SMALLINT,
    slot SMALLINT,
    collectible_id SMALLINT,
    x SMALLINT,
    y SMALLINT,
    evolution_stage SMALLINT,
    spawned_at TIMESTAMP,
    despawned_at TIMESTAMP,
    taken_by INTEGER REFERENCES characters(id),
    PRIMARY KEY (room_id, slot)
);
```

## Plants

```sql
CREATE TABLE plants (
    room_id SMALLINT,
    slot SMALLINT,
    plant_type SMALLINT,
    planted_by INTEGER REFERENCES characters(id),
    planted_at TIMESTAMP,
    growth_stage SMALLINT,
    has_fruit BOOLEAN,
    fruit_count SMALLINT,
    PRIMARY KEY (room_id, slot)
);
```

See [`01-schema-overview.md`](01-schema-overview.md) for all world tables.
