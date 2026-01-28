# Special Features

**See:** [`../protocol/04-message-catalog.md`](../protocol/04-message-catalog.md) for all message types

## Cannon System

- MSG_CANNON_ENTER (99)
- MSG_CANNON_MOVE (100)
- MSG_CANNON_SET_POWER (101)
- MSG_CANNON_SHOOT (102)

Launch players across maps.

## Racing System

- MSG_RACE_INFO (119)
- MSG_RACE_START (120)
- MSG_RACE_CHECKPOINT (121)
- MSG_RACE_END (122)

Timed races with checkpoints.

## Moving Platforms

- MSG_MOVE_GET_ON (123)
- MSG_MOVE_GET_OFF (124)

Sync player position on moving platforms.

## Building System

- MSG_BUILD_SPOT_FREE (103)
- MSG_BUILD_SPOT_USED (104)

Place decorative objects in world.

## Warp Center

- MSG_WARP_CENTER_USE (59)
- MSG_WARP_CENTER_SLOT (60-62)

Fast travel system.

## Storage System

- MSG_STORAGE_REQ (42-45)

Extra inventory storage.

All details in [`../protocol/04-message-catalog.md`](../protocol/04-message-catalog.md).
