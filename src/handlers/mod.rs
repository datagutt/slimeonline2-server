//! Message handlers for Slime Online 2 server

pub mod appearance;
pub mod auth;
pub mod bank;
pub mod bbs;
pub mod cannon;
pub mod chat;
pub mod clan;
pub mod collectibles;
mod connection;
pub mod gameplay;
pub mod items;
pub mod mail;
pub mod movement;
pub mod music;
pub mod one_time;
pub mod planting;
pub mod quest;
pub mod racing;
pub mod shop;
pub mod storage;
pub mod upgrader;
pub mod vending;
pub mod warp;

pub use connection::handle_connection;
