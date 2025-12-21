//! Item system handlers for Slime Online 2
//!
//! This module handles all item-related messages:
//! - MSG_USE_ITEM (31) - Use item from inventory
//! - MSG_DISCARD_ITEM (39) - Drop item on ground
//! - MSG_DISCARDED_ITEM_TAKE (40) - Pick up dropped item
//! - MSG_GET_ITEM (41) - Server -> Client item received

pub mod database;
mod use_item;
mod discard;
mod pickup;

pub use use_item::handle_use_item;
pub use discard::{handle_discard_item, write_room_dropped_items};
pub use pickup::handle_take_dropped_item;
