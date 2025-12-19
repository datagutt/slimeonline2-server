# NPC Database

**Extract from:** Client object files at `/slime2_decompile.gmx/objects/NPC_*.object.gmx`

## Common NPCs

- NPC_Shop: Shop keeper
- NPC_Mail: Mail service
- NPC_Quest: Quest giver
- NPC_Clan: Clan manager

## NPC Properties

```rust
pub struct Npc {
    pub npc_id: u16,
    pub name: String,
    pub x: u16,
    pub y: u16,
    pub room_id: u16,
    pub dialog: Vec<String>,
    pub function: NpcFunction,
}

pub enum NpcFunction {
    Shop(u16),      // Shop ID
    Quest(u16),     // Quest ID
    Dialog,         // Just talks
    Service,        // Mail, clan, etc.
}
```

Extract NPC positions and dialogs from client room files as needed.

See [`../protocol/04-message-catalog.md`](../protocol/04-message-catalog.md) for NPC interaction messages.
