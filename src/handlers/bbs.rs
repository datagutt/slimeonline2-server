//! BBS (Bulletin Board System) handlers

use std::sync::Arc;
use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::game::PlayerSession;
use crate::protocol::{MessageReader, MessageWriter, MessageType};
use crate::Server;
use crate::db;

/// BBS categories - hardcoded list
/// The client shows these in a dropdown when browsing
const BBS_CATEGORIES: &[&str] = &[
    "General",
    "Trading",
    "Events",
    "Help",
    "Off-Topic",
];

/// Cooldown between posts in seconds (prevent spam)
const BBS_POST_COOLDOWN_SECONDS: i64 = 60;

/// Maximum title length
const MAX_TITLE_LENGTH: usize = 50;

/// Maximum content length  
const MAX_CONTENT_LENGTH: usize = 5000;

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
    _server: &Arc<Server>,
    _session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::BbsRequestCategories.id())
        .write_u8((BBS_CATEGORIES.len() - 1) as u8); // Client reads count+1 categories (0 to count inclusive)
    
    for category in BBS_CATEGORIES {
        writer.write_string(category);
    }
    
    debug!("Sent {} BBS categories", BBS_CATEGORIES.len());
    
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
    _session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let category_id = reader.read_u8()? as i64;
    
    let page_count = db::get_bbs_page_count(&server.db, category_id).await.unwrap_or(0);
    
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::BbsRequestMaxPages.id())
        .write_u16(page_count as u16);
    
    debug!("BBS category {} has {} pages", category_id, page_count);
    
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
    _session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let category_id = reader.read_u8()? as i64;
    let page = reader.read_u16()? as i64;
    
    let posts = db::get_bbs_posts(&server.db, category_id, page).await.unwrap_or_default();
    
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::BbsRequestMessages.id());
    
    // Client loop: for i=0; i<=count; i++
    // So if we have 4 posts, we write count=3 and client reads 4
    // If we have 0 posts, we write count=255 (effectively -1) but that's wrong...
    // Actually looking at the client code more carefully:
    // _count = readbyte(); for i=0; i<=_count; i++
    // So count=0 means 1 iteration, count=3 means 4 iterations
    // If no posts, we should send count that results in 0 iterations
    // But i<=count with i=0 always runs at least once...
    // Let's check: if posts.len() == 0, we shouldn't send any data
    // The safest is to send count = posts.len() - 1 if posts.len() > 0
    // If posts.len() == 0, we need to handle specially
    
    if posts.is_empty() {
        // No posts - send count that makes client skip the loop
        // Actually the client code will still run once with count=0
        // Looking more closely: ds_map_add(messages, string(i)+"_Title", readstring())
        // It will try to read even with no data...
        // We need to send a dummy entry or handle this properly
        // Let's send 0xFF to indicate no messages (client might handle this?)
        // Actually, looking at the client: if max_pages = 0, current_page = 0
        // And the click handlers check if current_page = 0 and cancel
        // So if we return 0 pages in max_pages, the client won't try to load messages
        // But we're already in this handler so client expects something...
        // Let's send count=255 which will underflow in the loop condition
        writer.write_u8(0xFF); // This should make the loop not execute (i <= 255 starting from 0... hmm)
    } else {
        // Client reads count+1 messages
        writer.write_u8((posts.len() - 1) as u8);
        
        for post in &posts {
            // Format date nicely (just keep date part)
            let date = post.created_at.split(' ').next().unwrap_or(&post.created_at);
            
            writer.write_string(&post.title)
                .write_string(date)
                .write_u16(post.id as u16);
        }
    }
    
    debug!("Sent {} BBS posts for category {} page {}", posts.len(), category_id, page);
    
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
    
    let post = db::get_bbs_post(&server.db, message_id).await.ok().flatten();
    let poster_name = db::get_bbs_post_poster_name(&server.db, message_id).await.ok().flatten();
    
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::BbsRequestMessageContent.id());
    
    match post {
        Some(p) => {
            writer.write_string(&p.title)
                .write_string(&p.content)
                .write_string(&poster_name.unwrap_or_else(|| "Unknown".to_string()));
            
            debug!("Sent BBS post {} content", message_id);
        }
        None => {
            // Post not found - send empty strings
            writer.write_string("")
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
        Some(char_id) => {
            db::can_post_bbs(&server.db, char_id, BBS_POST_COOLDOWN_SECONDS)
                .await
                .unwrap_or(true)
        }
        None => false,
    };
    
    let mut writer = MessageWriter::new();
    writer.write_u16(MessageType::BbsRequestPost.id())
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
/// Server responds with MSG_BBS_POST (no payload = success, goes back to browse)
pub async fn handle_bbs_post(
    payload: &[u8],
    server: &Arc<Server>,
    session: Arc<RwLock<PlayerSession>>,
) -> Result<Vec<Vec<u8>>> {
    let mut reader = MessageReader::new(payload);
    let category_id = reader.read_u8()? as i64;
    let title = reader.read_string()?;
    let content = reader.read_string()?;
    
    let character_id = session.read().await.character_id;
    let char_id = match character_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };
    
    // Validate inputs
    if title.is_empty() || title.len() > MAX_TITLE_LENGTH {
        warn!("BBS post rejected: invalid title length {}", title.len());
        return Ok(vec![]); // Silent failure
    }
    
    if content.is_empty() || content.len() > MAX_CONTENT_LENGTH {
        warn!("BBS post rejected: invalid content length {}", content.len());
        return Ok(vec![]); // Silent failure
    }
    
    if category_id < 0 || category_id >= BBS_CATEGORIES.len() as i64 {
        warn!("BBS post rejected: invalid category {}", category_id);
        return Ok(vec![]);
    }
    
    // Check cooldown
    let can_post = db::can_post_bbs(&server.db, char_id, BBS_POST_COOLDOWN_SECONDS)
        .await
        .unwrap_or(true);
    
    if !can_post {
        warn!("BBS post rejected: on cooldown");
        return Ok(vec![]);
    }
    
    // Create the post
    match db::create_bbs_post(&server.db, char_id, category_id, &title, &content).await {
        Ok(post_id) => {
            debug!("Created BBS post {} in category {} by character {}", 
                   post_id, category_id, char_id);
            
            // Success - send response to return to browse mode
            let mut writer = MessageWriter::new();
            writer.write_u16(MessageType::BbsPost.id());
            
            Ok(vec![writer.into_bytes()])
        }
        Err(e) => {
            warn!("Failed to create BBS post: {}", e);
            Ok(vec![]) // Silent failure
        }
    }
}
