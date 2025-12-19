# Anti-Cheat Detection

**See:** [`02-server-validation.md`](02-server-validation.md) for complete validation examples.

## Detection Strategies

### Movement Validation
```rust
// Teleport detection - see 02-server-validation.md
let max_distance = calculate_max_movement(delta_time);
if distance > max_distance * 1.5 {
    log_suspicious_activity("teleport", player_id);
    ban_player(player_id, "Teleport detected");
}
```

### Item Duplication
```rust
// Transaction-based validation - see 02-server-validation.md
db.transaction(|tx| {
    verify_item_ownership(player_id, item_id, tx)?;
    remove_item(player_id, item_id, tx)?;
    add_to_recipient(recipient_id, item_id, tx)?;
})
```

### Rate Anomalies
```rust
// Detect message spam - see 02-server-validation.md
if message_count > 100 per second {
    ban_player(player_id, "Message spam");
}
```

All anti-cheat logic is detailed in [`02-server-validation.md`](02-server-validation.md).
