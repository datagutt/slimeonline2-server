# Movement Physics

**See:** [`../protocol/05-movement-protocol.md`](../protocol/05-movement-protocol.md) for complete details.

## Physics Constants

```rust
pub const HSPMAX: f32 = 2.5;
pub const JUMP_SPEED: f32 = 4.5;
pub const GRAVITY: f32 = 0.2145;
pub const ACCEL_GROUND: f32 = 0.33;
pub const FRICTION_GROUND: f32 = 0.165;
```

## Movement Directions

13 direction codes:
- 1-4: Ground press (left, right, up, down)
- 5-8: Ground release
- 9: Landing
- 10-13: Air movement

Full details in [`../protocol/05-movement-protocol.md`](../protocol/05-movement-protocol.md).
