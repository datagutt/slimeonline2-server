# Collectible System

**See:** [`../protocol/04-message-catalog.md`](../protocol/04-message-catalog.md) - Collectibles section

## Collectible Messages (4 total)

- MSG_COLLECTIBLE_INFO (32)
- MSG_COLLECTIBLE_TAKE_SELF (33)
- MSG_COLLECTIBLE_TAKEN (34)
- MSG_COLLECTIBLE_EVOLVE (132)

## Evolution

Collectibles evolve over time:
- Stage 0 → 1: 5 minutes
- Stage 1 → 2: 10 minutes
- Stage 2 → 3: 15 minutes

## Database

See [`../database/07-world-state.md`](../database/07-world-state.md) for collectibles table.
