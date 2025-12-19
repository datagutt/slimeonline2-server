# Items & Inventory Tables Reference

**See:** [`01-schema-overview.md`](01-schema-overview.md) for complete schema.

## Inventories Table

```sql
CREATE TABLE inventories (
    character_id INTEGER PRIMARY KEY REFERENCES characters(id),
    emote_1 to emote_5 SMALLINT,
    outfit_1 to outfit_9 SMALLINT,
    accessory_1 to accessory_9 SMALLINT,
    item_1 to item_9 SMALLINT,
    item_1_qty to item_9_qty SMALLINT,
    tool_1 to tool_7 SMALLINT,
    equipped_tool SMALLINT
);
```

## Common Operations

**Load Inventory:**
```sql
SELECT * FROM inventories WHERE character_id = $1;
```

**Update Item Slot:**
```sql
UPDATE inventories SET item_1 = $1, item_1_qty = $2 WHERE character_id = $3;
```

**Add Item (find empty slot):**
```rust
// Find first empty slot (item_N = 0)
// Or stack with existing item_N = item_id
```

See [`01-schema-overview.md`](01-schema-overview.md) for full schema.
