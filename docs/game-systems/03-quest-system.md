# Quest System

**See:** [`../protocol/04-message-catalog.md`](../protocol/04-message-catalog.md) - Quests section

## Quest Messages (10 total)

- MSG_QUEST_SET (82-86)
- MSG_QUEST_STEP (87)
- MSG_QUEST_GIVE_ITEM (88-91)

## Quest State

```rust
pub struct QuestState {
    pub quest_id: u16,
    pub quest_step: u16,
    pub quest_var: u16,
}
```

Stored in characters table. See [`../database/01-schema-overview.md`](../database/01-schema-overview.md).
