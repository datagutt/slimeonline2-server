//! BBS (Bulletin Board System) handlers

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::db;
use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageType, MessageWriter};
use crate::validation::validate_bbs_post;
use crate::Server;

/// Cooldown between posts in seconds (prevent spam)
const BBS_POST_COOLDOWN_SECONDS: i64 = 60;

/// Handle MSG_BBS_REQUEST_GUI (135)
/// Client clicked on a bulletin board NPC/object
/// Server responds to trigger GUI creation, then client requests categories
pub async fn handle_bbs_request_gui(
    _payload: &[u8],
    _server: &Arc<Server>,
    _session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    // Just echo back MSG_BBS_REQUEST_GUI to trigger GUI creation
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::BbsRequestGui.id());

    debug!("BBS GUI requested");

    Ok(vec![writer.into_bytes()])
}

/// Handle MSG_BBS_REQUEST_CATEGORIES (134)
/// Client wants the list of available BBS categories
///
/// Server responds with:
/// - count (u8): number of categories
/// - For each category: name (string)
pub async fn handle_bbs_request_categories(
    _payload: &[u8],
    server: &Arc<Server>,
    _session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let categories = &server.game_config.game.bbs.categories;

    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::BbsRequestCategories.id())
        .write_u8((categories.len() - 1) as u8); // Client reads count+1 categories (0 to count inclusive)

    for category in categories {
        writer.write_string(category);
    }

    debug!("Sent {} BBS categories", categories.len());

    Ok(vec![writer.into_bytes()])
}

/// Handle MSG_BBS_REQUEST_MAX_PAGES (136)
/// Client wants to know total pages for a category
///
/// Client sends:
/// - category_id (u8)
///
/// Server responds with:
/// - max_pages (u16)
pub async fn handle_bbs_request_max_pages(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let category_id = reader.read_u8()? as i64;

    // Get player's current room for per-room BBS
    let room_id = session.read().await.room_id as i64;

    let page_count = db::get_bbs_page_count(&server.db, room_id, category_id)
        .await
        .unwrap_or(0);

    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::BbsRequestMaxPages.id())
        .write_u16(page_count as u16);

    debug!(
        "BBS room {} category {} has {} pages",
        room_id, category_id, page_count
    );

    Ok(vec![writer.into_bytes()])
}

/// Handle MSG_BBS_REQUEST_MESSAGES (137)
/// Client wants list of messages for a category page
///
/// Client sends:
/// - category_id (u8)
/// - page (u16): 1-based page number
///
/// Server responds with:
/// - count (u8): number of messages on this page (0-3 for 4 messages, client reads count+1)
/// - For each message:
///   - title (string)
///   - date (string)
///   - id (u16)
pub async fn handle_bbs_request_messages(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let category_id = reader.read_u8()? as i64;
    let page = reader.read_u16()? as i64;

    // Get player's current room for per-room BBS
    let room_id = session.read().await.room_id as i64;

    // The client sends page=0 when a category is empty (max_pages=0 parks
    // current_page at 0 but still requests the list). The original server's
    // ini lookups simply found nothing; our SQL OFFSET (page-1)*4 would clamp
    // to 0 and wrongly return the first page, so short-circuit to empty.
    let posts = if page < 1 {
        Vec::new()
    } else {
        db::get_bbs_posts(&server.db, room_id, category_id, page)
            .await
            .unwrap_or_default()
    };

    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::BbsRequestMessages.id());

    // The count byte is `entries - 1` (the client loops i=0..=count); the
    // original server writes size-1 even for an empty list, i.e. 0xFF with no
    // records, and GM's zero-default buffer reads absorb it. Keep that exact
    // encoding: receivers treat 0xFF as "no messages".
    if posts.is_empty() {
        writer.write_u8(0xFF);
    } else {
        // Client reads count+1 messages
        writer.write_u8((posts.len() - 1) as u8);

        for post in &posts {
            // Format date nicely (just keep date part)
            let date = post
                .created_at
                .split(' ')
                .next()
                .unwrap_or(&post.created_at);

            writer
                .write_string(&post.title)
                .write_string(date)
                .write_u16(post.id as u16);
        }
    }

    debug!(
        "Sent {} BBS posts for category {} page {}",
        posts.len(),
        category_id,
        page
    );

    Ok(vec![writer.into_bytes()])
}

/// Handle MSG_BBS_REQUEST_MESSAGE_CONTENT (138)
/// Client wants to read a specific message
///
/// Client sends:
/// - category_id (u8)
/// - message_id (u16)
///
/// Server responds with:
/// - title (string)
/// - text (string)
/// - poster (string)
pub async fn handle_bbs_request_message_content(
    payload: &[u8],
    server: &Arc<Server>,
    _session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let _category_id = reader.read_u8()?; // Not needed for lookup
    let message_id = reader.read_u16()? as i64;

    let post = db::get_bbs_post(&server.db, message_id)
        .await
        .ok()
        .flatten();
    let poster_name = db::get_bbs_post_poster_name(&server.db, message_id)
        .await
        .ok()
        .flatten();

    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::BbsRequestMessageContent.id());

    match post {
        Some(p) => {
            writer
                .write_string(&p.title)
                .write_string(&p.content)
                .write_string(&poster_name.unwrap_or_else(|| "Unknown".to_string()));

            debug!("Sent BBS post {} content", message_id);
        }
        None => {
            // Post not found - send empty strings
            writer
                .write_string("")
                .write_string("Post not found")
                .write_string("System");

            warn!("BBS post {} not found", message_id);
        }
    }

    Ok(vec![writer.into_bytes()])
}

/// Handle MSG_BBS_REPORT_MESSAGE (139)
/// Client is reporting an inappropriate message
///
/// Client sends:
/// - category_id (u8)
/// - message_id (u16)
///
/// Server responds with MSG_BBS_REPORT_MESSAGE (no payload = success)
pub async fn handle_bbs_report_message(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let _category_id = reader.read_u8()?;
    let message_id = reader.read_u16()? as i64;

    let username = session.read().await.username.clone();

    if let Err(e) = db::report_bbs_post(&server.db, message_id).await {
        warn!("Failed to report BBS post {}: {}", message_id, e);
    } else {
        debug!("BBS post {} reported by {:?}", message_id, username);
    }

    // Always respond with success to return client to browse mode
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::BbsReportMessage.id());

    Ok(vec![writer.into_bytes()])
}

/// Handle MSG_BBS_REQUEST_POST (140)
/// Client wants to open the post creation form
/// Server checks if user can post (cooldown, etc.)
///
/// Server responds with:
/// - allow (u8): 1 = can post, 0 = on cooldown
pub async fn handle_bbs_request_post(
    _payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let character_id = session.read().await.character_id;

    let can_post = match character_id {
        Some(char_id) => db::can_post_bbs(&server.db, char_id, BBS_POST_COOLDOWN_SECONDS)
            .await
            .unwrap_or(true),
        None => false,
    };

    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::BbsRequestPost.id())
        .write_u8(if can_post { 1 } else { 0 });

    debug!("BBS post request: can_post = {}", can_post);

    Ok(vec![writer.into_bytes()])
}

/// Handle MSG_BBS_POST (141)
/// Client is submitting a new post
///
/// Client sends:
/// - category_id (u8)
/// - title (string)
/// - text (string)
///
/// Server responds with MSG_BBS_POST (no payload = success, goes back to
/// browse). Any rejection answers MSG_BBS_REQUEST_POST with allow=0 like the
/// original `case_msg_bbs_post.gml` else-branch, which flips the client to
/// its "can't post yet" screen instead of leaving the window hidden.
pub async fn handle_bbs_post(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let category_id = reader.read_u8()? as i64;
    let title = reader.read_string()?;
    let content = reader.read_string()?;

    let rejected = || {
        let mut writer = MessageWriter::new();
        writer
            .write_u16(MessageType::BbsRequestPost.id())
            .write_u8(0);
        Ok(vec![writer.into_bytes()])
    };

    // Get player's current room and character_id for per-room BBS
    let (character_id, room_id) = {
        let session_guard = session.read().await;
        (session_guard.character_id, session_guard.room_id as i64)
    };
    let char_id = match character_id {
        Some(id) => id,
        None => return rejected(),
    };

    // Validate inputs using validation module with config limits
    let limits = &server.game_config.game.limits;
    if let Err(e) = validate_bbs_post(
        &title,
        &content,
        limits.max_bbs_title,
        limits.max_bbs_content,
    ) {
        warn!("BBS post rejected: {} - {}", e.field, e.message);
        return rejected();
    }

    let num_categories = server.game_config.game.bbs.categories.len();
    if category_id < 0 || category_id >= num_categories as i64 {
        warn!("BBS post rejected: invalid category {}", category_id);
        return rejected();
    }

    // Check cooldown
    let can_post = db::can_post_bbs(&server.db, char_id, BBS_POST_COOLDOWN_SECONDS)
        .await
        .unwrap_or(true);

    if !can_post {
        warn!("BBS post rejected: on cooldown");
        return rejected();
    }

    // Create the post
    match db::create_bbs_post(&server.db, char_id, room_id, category_id, &title, &content).await {
        Ok(post_id) => {
            debug!(
                "Created BBS post {} in room {} category {} by character {}",
                post_id, room_id, category_id, char_id
            );

            // Success - send response to return to browse mode
            let mut writer = MessageWriter::new();
            writer.write_u16(MessageType::BbsPost.id());

            Ok(vec![writer.into_bytes()])
        }
        Err(e) => {
            warn!("Failed to create BBS post: {}", e);
            rejected()
        }
    }
}
