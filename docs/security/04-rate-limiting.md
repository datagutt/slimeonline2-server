# Rate Limiting

**See:** [`02-server-validation.md`](02-server-validation.md) for complete implementation.

## Rate Limit Tiers

### Per-Connection
- 100 messages/second (general)
- 60 movement messages/second
- 5 chat messages/second
- 10 item operations/second

### Per-IP
- 3 connections maximum
- 500 total connections server-wide

## Implementation

```rust
pub struct RateLimiter {
    message_times: VecDeque<Instant>,
    max_messages: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn check_rate(&mut self) -> RateLimitResult {
        // See 02-server-validation.md for full implementation
    }
}
```

All rate limiting code is in [`02-server-validation.md`](02-server-validation.md).
