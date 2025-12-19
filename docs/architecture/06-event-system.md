# Event-Driven Message Handling

**Document Status:** Complete  
**Last Updated:** 2024-01-08  
**Related:** [`01-overview.md`](01-overview.md), [`02-connection-manager.md`](02-connection-manager.md)

## Overview

The Event System provides an **event-driven architecture** for handling incoming messages, coordinating between different subsystems, and managing asynchronous operations. This document describes the message routing, handler patterns, and event coordination mechanisms.

**Key Concepts:**
- **Message Router:** Routes incoming messages to appropriate handlers
- **Handler Registry:** Maps message types to handler functions
- **Async Event Queue:** Channels for decoupling message processing
- **Event Bus:** Pub/sub for cross-system notifications

## Architecture

```
┌────────────────────────────────────────────────────────┐
│                   Event System                          │
├────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────────────────────────────────────────┐  │
│  │           Message Router                         │  │
│  │  - Parse message type (u16)                     │  │
│  │  - Route to handler                              │  │
│  │  - Error handling                                │  │
│  └──────────────┬──────────────────────────────────┘  │
│                 │                                       │
│                 v                                       │
│  ┌─────────────────────────────────────────────────┐  │
│  │         Handler Registry                         │  │
│  │  HashMap<MessageType, HandlerFn>                │  │
│  └──────────────┬──────────────────────────────────┘  │
│                 │                                       │
│    ┌────────────┴────────────┐                        │
│    v            v             v                        │
│  ┌──────┐  ┌──────┐     ┌──────┐                     │
│  │Auth  │  │Move  │ ... │Quest │                     │
│  │Handler│ │Handler│    │Handler│                    │
│  └──────┘  └──────┘     └──────┘                     │
│                                                         │
│  ┌─────────────────────────────────────────────────┐  │
│  │           Event Queue                            │  │
│  │  mpsc::channel for async events                 │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
└────────────────────────────────────────────────────────┘
```

## Message Router

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type MessageHandler = Arc<
    dyn Fn(MessageContext) -> BoxFuture<'static, Result<(), MessageError>> 
    + Send 
    + Sync
>;

pub struct MessageRouter {
    handlers: Arc<RwLock<HashMap<u16, MessageHandler>>>,
    default_handler: Option<MessageHandler>,
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            default_handler: None,
        }
    }
    
    /// Register message handler
    pub async fn register<F, Fut>(
        &self,
        message_type: u16,
        handler: F,
    ) where
        F: Fn(MessageContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), MessageError>> + Send + 'static,
    {
        let handler = Arc::new(move |ctx: MessageContext| {
            Box::pin(handler(ctx)) as BoxFuture<'static, Result<(), MessageError>>
        });
        
        self.handlers.write().await.insert(message_type, handler);
    }
    
    /// Route message to appropriate handler
    pub async fn route_message(
        &self,
        conn_id: ConnectionId,
        msg_type: u16,
        payload: &[u8],
    ) -> Result<(), MessageError> {
        let handlers = self.handlers.read().await;
        
        if let Some(handler) = handlers.get(&msg_type) {
            let ctx = MessageContext {
                conn_id,
                msg_type,
                payload: payload.to_vec(),
            };
            
            handler(ctx).await
        } else if let Some(default) = &self.default_handler {
            log::warn!("Unhandled message type: {}", msg_type);
            
            let ctx = MessageContext {
                conn_id,
                msg_type,
                payload: payload.to_vec(),
            };
            
            default(ctx).await
        } else {
            Err(MessageError::UnhandledMessageType(msg_type))
        }
    }
}

pub struct MessageContext {
    pub conn_id: ConnectionId,
    pub msg_type: u16,
    pub payload: Vec<u8>,
}
```

## Handler Implementation Pattern

### Basic Handler

```rust
pub async fn handle_chat_message(
    ctx: MessageContext,
    server: Arc<Server>,
) -> Result<(), MessageError> {
    // 1. Parse payload
    let mut reader = MessageReader::new(&ctx.payload);
    let message = reader.read_string()?;
    
    // 2. Get player
    let player_id = server.connection_manager
        .get_player_id(ctx.conn_id)
        .ok_or(MessageError::PlayerNotFound)?;
    
    // 3. Validate
    if message.len() > 200 {
        return Err(MessageError::MessageTooLong);
    }
    
    if message.is_empty() {
        return Err(MessageError::EmptyMessage);
    }
    
    // Apply profanity filter
    let filtered = server.profanity_filter.filter(&message);
    
    // 4. Execute logic
    server.broadcast_manager.broadcast_chat(
        player_id,
        &filtered,
        &server.player_manager,
    ).await?;
    
    // 5. Log
    log::info!("Player {} chat: {}", player_id, filtered);
    
    Ok(())
}
```

### Stateful Handler with Closure

```rust
pub fn create_login_handler(
    server: Arc<Server>,
) -> impl Fn(MessageContext) -> BoxFuture<'static, Result<(), MessageError>> {
    move |ctx: MessageContext| {
        let server = server.clone();
        
        Box::pin(async move {
            // Parse credentials
            let mut reader = MessageReader::new(&ctx.payload);
            let username = reader.read_string()?;
            let password = reader.read_string()?;
            let mac_address = reader.read_string()?;
            let version = reader.read_string()?;
            
            // Validate version
            if version != "0.106" {
                return server.send_login_error(
                    ctx.conn_id,
                    "Wrong client version",
                ).await;
            }
            
            // Authenticate
            let account_id = server.auth_service
                .authenticate(&username, &password)
                .await?;
            
            // Check bans
            server.ban_service
                .check_bans(account_id, &ctx.conn_id, &mac_address)
                .await?;
            
            // Create session
            let player_id = server.player_manager
                .login_player(account_id, ctx.conn_id)
                .await?;
            
            // Send success response
            server.send_login_success(ctx.conn_id, player_id).await?;
            
            // Broadcast new player to others
            server.broadcast_new_player(player_id).await?;
            
            Ok(())
        })
    }
}
```

## Handler Registry Setup

```rust
pub async fn setup_handlers(server: Arc<Server>) -> MessageRouter {
    let router = MessageRouter::new();
    
    // Authentication
    router.register(MSG_LOGIN, {
        let server = server.clone();
        move |ctx| handle_login(ctx, server.clone())
    }).await;
    
    router.register(MSG_REGISTER, {
        let server = server.clone();
        move |ctx| handle_register(ctx, server.clone())
    }).await;
    
    router.register(MSG_LOGOUT, {
        let server = server.clone();
        move |ctx| handle_logout(ctx, server.clone())
    }).await;
    
    // Movement
    router.register(MSG_MOVE_PLAYER, {
        let server = server.clone();
        move |ctx| handle_movement(ctx, server.clone())
    }).await;
    
    // Chat
    router.register(MSG_CHAT, {
        let server = server.clone();
        move |ctx| handle_chat(ctx, server.clone())
    }).await;
    
    // Items
    router.register(MSG_USE_ITEM, {
        let server = server.clone();
        move |ctx| handle_use_item(ctx, server.clone())
    }).await;
    
    router.register(MSG_DISCARD_ITEM, {
        let server = server.clone();
        move |ctx| handle_discard_item(ctx, server.clone())
    }).await;
    
    // ... register all 141 message types ...
    
    router
}
```

## Event Queue System

For operations that need to be processed asynchronously:

```rust
pub struct EventQueue {
    tx: mpsc::UnboundedSender<GameEvent>,
    rx: Arc<Mutex<mpsc::UnboundedReceiver<GameEvent>>>,
}

pub enum GameEvent {
    PlayerLoggedIn {
        player_id: PlayerId,
        account_id: AccountId,
    },
    PlayerLoggedOut {
        player_id: PlayerId,
    },
    PlayerChangedRoom {
        player_id: PlayerId,
        old_room: RoomId,
        new_room: RoomId,
    },
    ItemDiscarded {
        player_id: PlayerId,
        room_id: RoomId,
        item_id: u16,
        quantity: u16,
    },
    CollectibleTaken {
        player_id: PlayerId,
        room_id: RoomId,
        slot: u8,
    },
    // ... more events
}

impl EventQueue {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tx,
            rx: Arc::new(Mutex::new(rx)),
        }
    }
    
    pub fn send(&self, event: GameEvent) -> Result<(), SendError<GameEvent>> {
        self.tx.send(event)
    }
    
    pub async fn process_events(
        &self,
        server: Arc<Server>,
    ) {
        let mut rx = self.rx.lock().await;
        
        while let Some(event) = rx.recv().await {
            if let Err(e) = self.handle_event(event, &server).await {
                log::error!("Event processing error: {}", e);
            }
        }
    }
    
    async fn handle_event(
        &self,
        event: GameEvent,
        server: &Server,
    ) -> Result<(), EventError> {
        match event {
            GameEvent::PlayerLoggedIn { player_id, account_id } => {
                log::info!("Player {} logged in (account {})", player_id, account_id);
                
                // Update online player count
                server.metrics.increment_online_players();
                
                // Log to database
                server.db_log_login(player_id, account_id).await?;
            }
            
            GameEvent::PlayerLoggedOut { player_id } => {
                log::info!("Player {} logged out", player_id);
                
                // Update metrics
                server.metrics.decrement_online_players();
                
                // Save player data
                server.player_manager.save_player(player_id).await?;
            }
            
            GameEvent::PlayerChangedRoom { player_id, old_room, new_room } => {
                // Update room player lists
                server.world_manager
                    .update_room_player(player_id, old_room, new_room)
                    .await?;
                
                // Broadcast to both rooms
                server.broadcast_manager
                    .handle_room_transition(player_id, old_room, new_room)
                    .await?;
            }
            
            GameEvent::ItemDiscarded { player_id, room_id, item_id, quantity } => {
                // Create discarded item in world
                server.world_manager
                    .spawn_discarded_item(room_id, item_id, quantity, player_id)
                    .await?;
                
                // Log for potential rollback
                server.db_log_item_discard(player_id, item_id, quantity).await?;
            }
            
            GameEvent::CollectibleTaken { player_id, room_id, slot } => {
                // Remove from world
                server.world_manager
                    .remove_collectible(room_id, slot)
                    .await?;
                
                // Award to player
                server.player_manager
                    .award_collectible(player_id, slot)
                    .await?;
            }
        }
        
        Ok(())
    }
}
```

## Event Bus (Pub/Sub)

For cross-system notifications:

```rust
use tokio::sync::broadcast;

pub struct EventBus {
    tx: broadcast::Sender<ServerEvent>,
}

pub enum ServerEvent {
    PlayerJoined(PlayerId),
    PlayerLeft(PlayerId),
    GlobalAnnouncement(String),
    ServerShutdown,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }
    
    pub fn publish(&self, event: ServerEvent) {
        let _ = self.tx.send(event);
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<ServerEvent> {
        self.tx.subscribe()
    }
}

// Usage in subsystems
pub async fn listen_for_player_events(
    event_bus: Arc<EventBus>,
    metrics: Arc<Metrics>,
) {
    let mut rx = event_bus.subscribe();
    
    while let Ok(event) = rx.recv().await {
        match event {
            ServerEvent::PlayerJoined(player_id) => {
                metrics.increment_online_players();
                log::info!("Player {} joined (via event bus)", player_id);
            }
            ServerEvent::PlayerLeft(player_id) => {
                metrics.decrement_online_players();
                log::info!("Player {} left (via event bus)", player_id);
            }
            ServerEvent::GlobalAnnouncement(msg) => {
                log::info!("Global announcement: {}", msg);
            }
            ServerEvent::ServerShutdown => {
                log::warn!("Shutdown signal received");
                break;
            }
        }
    }
}
```

## Error Handling

```rust
#[derive(Debug)]
pub enum MessageError {
    ParseError(String),
    PlayerNotFound,
    InvalidPayload,
    UnhandledMessageType(u16),
    ValidationError(String),
    DatabaseError(sqlx::Error),
    BroadcastError,
}

impl MessageError {
    pub fn should_disconnect(&self) -> bool {
        matches!(self, 
            MessageError::InvalidPayload | 
            MessageError::ValidationError(_)
        )
    }
}

// Error handling in router
pub async fn route_with_error_handling(
    router: &MessageRouter,
    ctx: MessageContext,
) -> Result<(), MessageError> {
    match router.route_message(ctx.conn_id, ctx.msg_type, &ctx.payload).await {
        Ok(()) => Ok(()),
        Err(e) => {
            log::error!(
                "Message handling error (type={}, conn={}): {:?}",
                ctx.msg_type,
                ctx.conn_id,
                e
            );
            
            if e.should_disconnect() {
                log::warn!("Disconnecting connection {} due to error", ctx.conn_id);
                // Trigger disconnect
            }
            
            Err(e)
        }
    }
}
```

## Message Pipeline

Full pipeline from connection to response:

```rust
pub async fn message_pipeline(
    raw_message: Vec<u8>,
    conn_id: ConnectionId,
    server: Arc<Server>,
) -> Result<(), PipelineError> {
    // 1. Decrypt (already done by connection manager)
    
    // 2. Parse message type
    if raw_message.len() < 2 {
        return Err(PipelineError::MessageTooShort);
    }
    
    let msg_type = u16::from_le_bytes([raw_message[0], raw_message[1]]);
    let payload = &raw_message[2..];
    
    // 3. Rate limit check
    if !server.rate_limiter.check(conn_id, msg_type).await {
        return Err(PipelineError::RateLimited);
    }
    
    // 4. Authentication check
    if !is_authenticated(conn_id, &server).await {
        if msg_type != MSG_LOGIN && msg_type != MSG_REGISTER {
            return Err(PipelineError::NotAuthenticated);
        }
    }
    
    // 5. Route to handler
    let ctx = MessageContext {
        conn_id,
        msg_type,
        payload: payload.to_vec(),
    };
    
    server.message_router.route_message(
        ctx.conn_id,
        ctx.msg_type,
        &ctx.payload,
    ).await?;
    
    // 6. Update metrics
    server.metrics.increment_messages_processed();
    
    Ok(())
}
```

## Middleware Pattern

For cross-cutting concerns:

```rust
pub trait Middleware: Send + Sync {
    fn process(
        &self,
        ctx: &mut MessageContext,
    ) -> BoxFuture<'static, Result<(), MiddlewareError>>;
}

pub struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn process(
        &self,
        ctx: &mut MessageContext,
    ) -> BoxFuture<'static, Result<(), MiddlewareError>> {
        Box::pin(async move {
            log::trace!(
                "Processing message: type={}, size={}",
                ctx.msg_type,
                ctx.payload.len()
            );
            Ok(())
        })
    }
}

pub struct RateLimitMiddleware {
    limiter: Arc<RateLimiter>,
}

impl Middleware for RateLimitMiddleware {
    fn process(
        &self,
        ctx: &mut MessageContext,
    ) -> BoxFuture<'static, Result<(), MiddlewareError>> {
        let limiter = self.limiter.clone();
        let conn_id = ctx.conn_id;
        let msg_type = ctx.msg_type;
        
        Box::pin(async move {
            if !limiter.check(conn_id, msg_type).await {
                return Err(MiddlewareError::RateLimited);
            }
            Ok(())
        })
    }
}

// Apply middleware chain
pub struct MiddlewareChain {
    middleware: Vec<Arc<dyn Middleware>>,
}

impl MiddlewareChain {
    pub async fn process(
        &self,
        ctx: &mut MessageContext,
    ) -> Result<(), MiddlewareError> {
        for mw in &self.middleware {
            mw.process(ctx).await?;
        }
        Ok(())
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_message_routing() {
        let router = MessageRouter::new();
        let called = Arc::new(AtomicBool::new(false));
        
        let called_clone = called.clone();
        router.register(100, move |_ctx| {
            let called = called_clone.clone();
            Box::pin(async move {
                called.store(true, Ordering::Relaxed);
                Ok(())
            })
        }).await;
        
        let ctx = MessageContext {
            conn_id: Uuid::new_v4(),
            msg_type: 100,
            payload: vec![],
        };
        
        router.route_message(ctx.conn_id, ctx.msg_type, &ctx.payload).await.unwrap();
        
        assert!(called.load(Ordering::Relaxed));
    }
    
    #[tokio::test]
    async fn test_event_queue() {
        let queue = EventQueue::new();
        
        queue.send(GameEvent::PlayerLoggedIn {
            player_id: 1,
            account_id: 100,
        }).unwrap();
        
        // Process event
        // ...
    }
}
```

## Summary

The Event System provides:
- ✅ **Message routing** with type-based dispatch
- ✅ **Handler registry** for all 141 message types
- ✅ **Async event queue** for decoupled processing
- ✅ **Event bus** for pub/sub notifications
- ✅ **Middleware pattern** for cross-cutting concerns
- ✅ **Error handling** with disconnect logic
- ✅ **Message pipeline** from raw bytes to response

**Key Patterns:**
- Handler functions return `Future<Result<(), MessageError>>`
- Closures capture server Arc for state access
- Event queue decouples message handling from side effects
- Event bus enables cross-system communication
- Middleware for logging, rate limiting, auth checks

**Next:** Implement handlers for all 141 message types using this architecture.
