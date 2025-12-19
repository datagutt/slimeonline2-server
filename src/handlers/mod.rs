//! Message handlers for Slime Online 2 server

mod connection;
pub mod auth;

pub use connection::handle_connection;
