# Planting System

**See:** [`../protocol/04-message-catalog.md`](../protocol/04-message-catalog.md) - Planting section

## Plant Messages (11 total)

- MSG_PLANT_SPOT_FREE (63)
- MSG_PLANT_SPOT_USED (64)
- MSG_PLANT_DIE (65)
- MSG_PLANT_GROW (66)
- MSG_PLANT_ADD_PINWHEEL (67)
- MSG_PLANT_ADD_FAIRY (68)
- MSG_PLANT_GET_FRUIT (69)
- MSG_PLANT_HAS_FRUIT (70)
- MSG_TREE_PLANTED_INC (94)

## Growth Stages

0: Seedling  
1-3: Growing  
4: Full grown (produces fruit)

## Database

See [`../database/07-world-state.md`](../database/07-world-state.md) for plants table.

Growth updates every 10 minutes.
