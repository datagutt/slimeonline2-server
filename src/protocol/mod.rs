//! Protocol handling for Slime Online 2 binary message format
//!
//! Messages use little-endian byte order with null-terminated strings.

mod reader;
mod writer;
mod messages;

pub use reader::MessageReader;
pub use writer::MessageWriter;
pub use messages::*;
