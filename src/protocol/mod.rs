//! Protocol handling for Slime Online 2 binary message format
//!
//! Messages use little-endian byte order with null-terminated strings.

mod messages;
mod reader;
pub mod types;
mod writer;

pub use messages::*;
pub use reader::MessageReader;
pub use types::{describe_message, MessageType};
pub use writer::MessageWriter;
