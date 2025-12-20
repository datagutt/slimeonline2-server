//! Shop system handlers for Slime Online 2
//!
//! This module handles all shop-related messages:
//! - MSG_SHOP_BUY (28) - Purchase item from shop
//! - MSG_SHOP_BUY_FAIL (29) - Server -> Client purchase failed
//! - MSG_ROOM_SHOP_INFO (27) - Server -> Client shop items in room

mod buy;

pub use buy::handle_shop_buy;
pub use buy::build_room_shop_info;
