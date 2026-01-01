//! Message handlers for Slime Online 2 server

pub mod appearance;
pub mod auth;
pub mod bank;
pub mod bbs;
pub mod chat;
pub mod clan;
pub mod collectibles;
mod connection;
pub mod gameplay;
pub mod items;
pub mod mail;
pub mod movement;
pub mod quest;
pub mod shop;
pub mod upgrader;
pub mod warp;

pub use connection::handle_connection;
