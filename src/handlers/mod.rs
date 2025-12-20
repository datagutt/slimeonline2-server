//! Message handlers for Slime Online 2 server

mod connection;
pub mod auth;
pub mod movement;
pub mod chat;
pub mod appearance;
pub mod gameplay;
pub mod warp;
pub mod items;
pub mod shop;
pub mod bank;
pub mod mail;
pub mod collectibles;

pub use connection::handle_connection;
