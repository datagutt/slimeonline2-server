//! Authentication handlers (login and registration)

use std::sync::Arc;

use anyhow::Result;
use chrono::{Datelike, Timelike};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::constants::*;
use crate::db;
use crate::game::PlayerSession;
use crate::protocol::{
    LoginRequest, LoginSuccessData, MessageReader, MessageWriter, RegisterRequest,
    write_login_failure, write_register_response,
};
use crate::Server;

/// Handle login request
pub async fn handle_login(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    
    let login = match LoginRequest::parse(&mut reader) {
        Ok(req) => req,
        Err(e) => {
            warn!("Failed to parse login request: {}", e);
            let mut writer = MessageWriter::new();
            write_login_failure(&mut writer, LOGIN_VERSION_MISMATCH);
            return Ok(vec![writer.into_bytes()]);
        }
    };

    debug!("Login attempt from user: {}", login.username);

    // Validate the request
    if let Err(msg) = login.validate() {
        warn!("Login validation failed: {}", msg);
        let mut writer = MessageWriter::new();
        write_login_failure(&mut writer, LOGIN_VERSION_MISMATCH);
        return Ok(vec![writer.into_bytes()]);
    }

    // Check IP ban
    let ip = session.read().await.ip_address.clone();
    if db::is_ip_banned(&server.db, &ip).await.unwrap_or(false) {
        warn!("Login attempt from banned IP: {}", ip);
        let mut writer = MessageWriter::new();
        write_login_failure(&mut writer, LOGIN_IP_BANNED_1);
        return Ok(vec![writer.into_bytes()]);
    }

    // Check MAC ban
    if db::is_mac_banned(&server.db, &login.mac_address).await.unwrap_or(false) {
        warn!("Login attempt from banned MAC: {}", login.mac_address);
        let mut writer = MessageWriter::new();
        write_login_failure(&mut writer, LOGIN_IP_BANNED_2);
        return Ok(vec![writer.into_bytes()]);
    }

    // Find account
    let account = match db::find_account_by_username(&server.db, &login.username).await {
        Ok(Some(acc)) => acc,
        Ok(None) => {
            debug!("Account not found: {}", login.username);
            let mut writer = MessageWriter::new();
            write_login_failure(&mut writer, LOGIN_NO_ACCOUNT);
            return Ok(vec![writer.into_bytes()]);
        }
        Err(e) => {
            error!("Database error finding account: {}", e);
            let mut writer = MessageWriter::new();
            write_login_failure(&mut writer, LOGIN_NO_ACCOUNT);
            return Ok(vec![writer.into_bytes()]);
        }
    };

    // Check if account is banned
    if account.is_banned {
        warn!("Login attempt on banned account: {}", login.username);
        let mut writer = MessageWriter::new();
        write_login_failure(&mut writer, LOGIN_ACCOUNT_BANNED);
        return Ok(vec![writer.into_bytes()]);
    }

    // Verify password
    let password_valid = bcrypt::verify(&login.password, &account.password_hash)
        .unwrap_or(false);
    
    if !password_valid {
        debug!("Wrong password for: {}", login.username);
        let mut writer = MessageWriter::new();
        write_login_failure(&mut writer, LOGIN_WRONG_PASSWORD);
        return Ok(vec![writer.into_bytes()]);
    }

    // Check if already logged in
    if server.is_player_online(&login.username) {
        warn!("Account already logged in: {}", login.username);
        let mut writer = MessageWriter::new();
        write_login_failure(&mut writer, LOGIN_ALREADY_LOGGED_IN);
        return Ok(vec![writer.into_bytes()]);
    }

    // Get or create character
    let character = match db::find_character_by_account(&server.db, account.id).await {
        Ok(Some(char)) => char,
        Ok(None) => {
            // Create new character for this account
            match db::create_character(&server.db, account.id, &login.username).await {
                Ok(_char_id) => {
                    match db::find_character_by_account(&server.db, account.id).await {
                        Ok(Some(char)) => char,
                        _ => {
                            error!("Failed to retrieve newly created character");
                            let mut writer = MessageWriter::new();
                            write_login_failure(&mut writer, LOGIN_NO_ACCOUNT);
                            return Ok(vec![writer.into_bytes()]);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to create character: {}", e);
                    let mut writer = MessageWriter::new();
                    write_login_failure(&mut writer, LOGIN_NO_ACCOUNT);
                    return Ok(vec![writer.into_bytes()]);
                }
            }
        }
        Err(e) => {
            error!("Database error finding character: {}", e);
            let mut writer = MessageWriter::new();
            write_login_failure(&mut writer, LOGIN_NO_ACCOUNT);
            return Ok(vec![writer.into_bytes()]);
        }
    };

    // Get inventory
    let inventory = match db::get_inventory(&server.db, character.id).await {
        Ok(Some(inv)) => inv,
        Ok(None) => {
            error!("No inventory found for character {}", character.id);
            let mut writer = MessageWriter::new();
            write_login_failure(&mut writer, LOGIN_NO_ACCOUNT);
            return Ok(vec![writer.into_bytes()]);
        }
        Err(e) => {
            error!("Database error getting inventory: {}", e);
            let mut writer = MessageWriter::new();
            write_login_failure(&mut writer, LOGIN_NO_ACCOUNT);
            return Ok(vec![writer.into_bytes()]);
        }
    };

    // Update last login time
    let _ = db::update_last_login(&server.db, account.id).await;

    // Assign player ID
    let player_id = server.next_player_id();

    // Update session
    {
        let mut session_guard = session.write().await;
        session_guard.player_id = Some(player_id);
        session_guard.account_id = Some(account.id);
        session_guard.character_id = Some(character.id);
        session_guard.username = Some(login.username.clone());
        session_guard.room_id = character.room_id as u16;
        session_guard.x = character.x as u16;
        session_guard.y = character.y as u16;
        session_guard.body_id = character.body_id as u16;
        session_guard.acs1_id = character.acs1_id as u16;
        session_guard.acs2_id = character.acs2_id as u16;
        session_guard.points = character.points as u32;
        session_guard.is_authenticated = true;
    }

    // Add player to room
    let session_id = session.read().await.session_id;
    server.game_state.add_player_to_room(player_id, character.room_id as u16, session_id).await;
    server.active_player_ids.insert(player_id, session_id);

    // Get current time
    let now = chrono::Local::now();
    let day = match now.weekday() {
        chrono::Weekday::Sun => SUNDAY,
        chrono::Weekday::Mon => MONDAY,
        chrono::Weekday::Tue => TUESDAY,
        chrono::Weekday::Wed => WEDNESDAY,
        chrono::Weekday::Thu => THURSDAY,
        chrono::Weekday::Fri => FRIDAY,
        chrono::Weekday::Sat => SATURDAY,
    };

    // Build success response
    let login_data = LoginSuccessData {
        player_id,
        server_time: now.timestamp() as u32,
        motd: server.config.motd.clone(),
        day,
        hour: now.hour() as u8,
        minute: now.minute() as u8,
        username: login.username.clone(),
        spawn_x: character.x as u16,
        spawn_y: character.y as u16,
        spawn_room: character.room_id as u16,
        body_id: character.body_id as u16,
        acs1_id: character.acs1_id as u16,
        acs2_id: character.acs2_id as u16,
        points: character.points as u32,
        has_signature: character.has_signature,
        quest_id: character.quest_id as u16,
        quest_step: character.quest_step as u8,
        trees_planted: character.trees_planted as u16,
        objects_built: character.objects_built as u16,
        emotes: inventory.emotes(),
        outfits: inventory.outfits(),
        accessories: inventory.accessories(),
        items: inventory.items(),
        tools: inventory.tools(),
    };

    let mut writer = MessageWriter::new();
    login_data.write(&mut writer);

    info!("Player {} logged in as ID {}", login.username, player_id);

    // Notify other players in the room
    let room_players = server.game_state.get_room_players(character.room_id as u16).await;
    let mut responses = vec![writer.into_bytes()];

    for other_player_id in room_players {
        if other_player_id == player_id {
            continue;
        }

        // Send new player notification to existing players
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(&other_session_id) {
                let new_player = crate::protocol::NewPlayerInfo {
                    x: character.x as u16,
                    y: character.y as u16,
                    player_id,
                    room_id: character.room_id as u16,
                    username: login.username.clone(),
                    body_id: character.body_id as u16,
                    acs1_id: character.acs1_id as u16,
                    acs2_id: character.acs2_id as u16,
                    ileft: 0,
                    iright: 0,
                    iup: 0,
                    idown: 0,
                    iup_press: 0,
                };
                let mut nw = MessageWriter::new();
                new_player.write_case1(&mut nw);
                other_session.write().await.queue_message(nw.into_bytes());
            }
        }

        // Get info about existing player to send to new player
        if let Some(other_session_id) = server.game_state.players_by_id.get(&other_player_id) {
            if let Some(other_session) = server.sessions.get(&other_session_id) {
                let other_guard = other_session.read().await;
                if let Some(other_username) = &other_guard.username {
                    let existing_player = crate::protocol::NewPlayerInfo {
                        x: other_guard.x,
                        y: other_guard.y,
                        player_id: other_player_id,
                        room_id: other_guard.room_id,
                        username: other_username.clone(),
                        body_id: other_guard.body_id,
                        acs1_id: other_guard.acs1_id,
                        acs2_id: other_guard.acs2_id,
                        ileft: 0,
                        iright: 0,
                        iup: 0,
                        idown: 0,
                        iup_press: 0,
                    };
                    let mut nw = MessageWriter::new();
                    existing_player.write_case2(&mut nw);
                    responses.push(nw.into_bytes());
                }
            }
        }
    }

    Ok(responses)
}

/// Handle registration request
pub async fn handle_register(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    
    let register = match RegisterRequest::parse(&mut reader) {
        Ok(req) => req,
        Err(e) => {
            warn!("Failed to parse register request: {}", e);
            let mut writer = MessageWriter::new();
            write_register_response(&mut writer, REGISTER_EXISTS);
            return Ok(vec![writer.into_bytes()]);
        }
    };

    debug!("Registration attempt for user: {}", register.username);

    // Validate the request
    if let Err(msg) = register.validate() {
        warn!("Registration validation failed: {}", msg);
        let mut writer = MessageWriter::new();
        write_register_response(&mut writer, REGISTER_EXISTS);
        return Ok(vec![writer.into_bytes()]);
    }

    // Check IP ban
    let ip = session.read().await.ip_address.clone();
    if db::is_ip_banned(&server.db, &ip).await.unwrap_or(false) {
        warn!("Registration attempt from banned IP: {}", ip);
        let mut writer = MessageWriter::new();
        write_register_response(&mut writer, REGISTER_IP_BANNED);
        return Ok(vec![writer.into_bytes()]);
    }

    // Check MAC ban
    if db::is_mac_banned(&server.db, &register.mac_address).await.unwrap_or(false) {
        warn!("Registration attempt from banned MAC: {}", register.mac_address);
        let mut writer = MessageWriter::new();
        write_register_response(&mut writer, REGISTER_MAC_BANNED);
        return Ok(vec![writer.into_bytes()]);
    }

    // Check if username already exists
    if db::username_exists(&server.db, &register.username).await.unwrap_or(true) {
        debug!("Username already exists: {}", register.username);
        let mut writer = MessageWriter::new();
        write_register_response(&mut writer, REGISTER_EXISTS);
        return Ok(vec![writer.into_bytes()]);
    }

    // Hash the password
    let password_hash = match bcrypt::hash(&register.password, bcrypt::DEFAULT_COST) {
        Ok(hash) => hash,
        Err(e) => {
            error!("Failed to hash password: {}", e);
            let mut writer = MessageWriter::new();
            write_register_response(&mut writer, REGISTER_EXISTS);
            return Ok(vec![writer.into_bytes()]);
        }
    };

    // Create the account
    match db::create_account(&server.db, &register.username, &password_hash, &register.mac_address).await {
        Ok(account_id) => {
            // Create a character for the account
            match db::create_character(&server.db, account_id, &register.username).await {
                Ok(_) => {
                    info!("New account registered: {}", register.username);
                    let mut writer = MessageWriter::new();
                    write_register_response(&mut writer, REGISTER_SUCCESS);
                    Ok(vec![writer.into_bytes()])
                }
                Err(e) => {
                    error!("Failed to create character: {}", e);
                    let mut writer = MessageWriter::new();
                    write_register_response(&mut writer, REGISTER_EXISTS);
                    Ok(vec![writer.into_bytes()])
                }
            }
        }
        Err(e) => {
            error!("Failed to create account: {}", e);
            let mut writer = MessageWriter::new();
            write_register_response(&mut writer, REGISTER_EXISTS);
            Ok(vec![writer.into_bytes()])
        }
    }
}
