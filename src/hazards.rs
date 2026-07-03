//! Server-driven room hazards: lavaball volleys, vertical fireballs, and
//! falling rocks (ports of the original server's `obj_lava_call_controll`,
//! `obj_lava_controll`, and `obj_rock_controll`).
//!
//! The controllers broadcast MSG_CREATE (16) to the room:
//! - case 1: a vertical fireball at a random x within the controller's span
//! - case 2: a falling rock at the controller's position
//! - case 3: "arm your lavaball callers" (the volley pattern is client-side)
//!
//! Per-room data below is extracted from the original `srvr_rooms/*.rom` files
//! ([Lava Caller] / [Lava Controller] / [Rock Controller] sections). Delays are
//! seconds; the wire `time_delay` is in original server frames (room_speed 120,
//! the client halves it to its own 60 Hz frames).

use std::sync::Arc;
use std::time::Instant;

use rand::Rng;

use crate::Server;
use crate::protocol::{MessageType, MessageWriter};

/// Original server frame rate: `.rom` second values scale by this on the wire.
const SERVER_FPS: f64 = 120.0;

/// `[Lava Caller] Send Delay` per room: (room id, delay seconds). The volley
/// trigger fires on a fixed period, so late joiners can be caught up from the
/// phase of the server clock alone.
const CALLERS: &[(u16, f64)] = &[(95, 5.0), (100, 6.0), (101, 10.0), (108, 22.0)];

/// `[Lava Controller]`: (room, x, y, end, delay-min, delay-max).
const LAVA: &[(u16, f64, f64, f64, f64, f64)] = &[
    (15, 32.0, 288.0, 192.0, 3.0, 6.0),
    (71, 320.0, 504.0, 448.0, 2.0, 3.0),
    (73, 144.0, 224.0, 264.0, 2.0, 3.0),
    (73, 392.0, 192.0, 480.0, 3.0, 4.0),
    (74, 112.0, 504.0, 336.0, 1.0, 3.0),
    (74, 528.0, 488.0, 608.0, 3.0, 5.0),
    (91, 128.0, 176.0, 256.0, 1.0, 4.0),
    (92, 1184.0, 1200.0, 1440.0, 1.0, 3.0),
    (92, 1344.0, 224.0, 1536.0, 1.0, 3.0),
    (98, 89.0, 216.0, 200.0, 1.0, 3.0),
    (99, 296.0, 352.0, 816.0, 2.0, 6.0),
    (102, 96.0, 656.0, 352.0, 1.0, 6.0),
    (102, 752.0, 480.0, 848.0, 2.0, 5.0),
];

/// `[Rock Controller]`: (room, x, y, delay-min, delay-max).
const ROCKS: &[(u16, f64, f64, f64, f64)] = &[
    (92, 288.0, 912.0, 3.0, 6.0),
    (92, 464.0, 48.0, 3.0, 6.0),
    (102, 576.0, 1.0, 3.0, 6.0),
    (102, 720.0, 1.0, 3.0, 6.0),
];

/// MSG_SERVER_TIME (118): milliseconds since the server started (the original
/// `global.server_time`); drives client-side moving-platform phase sync.
pub fn server_time_message(server: &Server) -> Vec<u8> {
    let mut writer = MessageWriter::new();
    writer
        .write_u16(MessageType::ServerTime.id())
        .write_u32(server.started_at.elapsed().as_millis() as u32);
    writer.into_bytes()
}

/// Volley catch-up for a player entering `room` (original
/// `room_check_lava_callers`): MSG_CREATE case 3 with the frames already
/// elapsed in the current caller cycle, so the client arms only the remaining
/// alarms of the pattern.
pub fn caller_catchup_messages(server: &Server, room: u16) -> Vec<Vec<u8>> {
    let elapsed = server.started_at.elapsed().as_secs_f64();
    CALLERS
        .iter()
        .filter(|(r, _)| *r == room)
        .map(|(r, delay)| {
            let in_cycle = elapsed % delay;
            let frames = (in_cycle * SERVER_FPS).min(u16::MAX as f64) as u16;
            let mut writer = MessageWriter::new();
            writer
                .write_u16(MessageType::Create.id())
                .write_u8(3)
                .write_u16(*r)
                .write_u16(frames);
            writer.into_bytes()
        })
        .collect()
}

/// Queue a message to everyone currently in `room`.
async fn broadcast_room(server: &Server, room: u16, bytes: Vec<u8>) {
    for player_id in server.game_state.get_room_players(room).await {
        if let Some(session_id) = server.game_state.players_by_id.get(&player_id)
            && let Some(handle) = server.sessions.get(&session_id)
        {
            handle.queue_message(bytes.clone()).await;
        }
    }
}

/// The hazard scheduler: runs for the lifetime of the server.
pub async fn run(server: Arc<Server>) {
    // Callers fire on fixed periods anchored to server start; the others re-arm
    // with a random delay each shot (GM `alarm[0] = min + random(max - min)`,
    // starting at max like the original loader).
    let mut caller_cycles: Vec<u64> = CALLERS
        .iter()
        .map(|(_, delay)| (server.started_at.elapsed().as_secs_f64() / delay) as u64)
        .collect();
    let now = Instant::now();
    let mut lava_next: Vec<Instant> = LAVA
        .iter()
        .map(|(_, _, _, _, _, max)| now + std::time::Duration::from_secs_f64(*max))
        .collect();
    let mut rock_next: Vec<Instant> = ROCKS
        .iter()
        .map(|(_, _, _, _, max)| now + std::time::Duration::from_secs_f64(*max))
        .collect();

    let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
    loop {
        interval.tick().await;
        let now = Instant::now();
        let elapsed = server.started_at.elapsed().as_secs_f64();

        for (i, (room, delay)) in CALLERS.iter().enumerate() {
            let cycle = (elapsed / delay) as u64;
            if cycle > caller_cycles[i] {
                caller_cycles[i] = cycle;
                let mut writer = MessageWriter::new();
                writer
                    .write_u16(MessageType::Create.id())
                    .write_u8(3)
                    .write_u16(*room)
                    .write_u16(0);
                broadcast_room(&server, *room, writer.into_bytes()).await;
            }
        }

        for (i, (room, x, y, end, min, max)) in LAVA.iter().enumerate() {
            if now < lava_next[i] {
                continue;
            }
            let (send_x, send_dir) = {
                let mut rng = rand::thread_rng();
                // GM: send_x = (x_start+16) + round(random(x_end - x_start) - 16)
                let send_x = (x + 16.0) + (rng.gen_range(0.0..(end - x)) - 16.0).round();
                let mut dir = 70.0 + rng.gen_range(0.0..40.0f64).round();
                if send_x < x + 64.0 && dir > 100.0 {
                    dir = 100.0;
                } else if send_x > x + end - 64.0 && dir < 80.0 {
                    dir = 80.0;
                }
                lava_next[i] = now
                    + std::time::Duration::from_secs_f64(rng.gen_range(*min..*max));
                (send_x.max(0.0) as u16, dir as u8)
            };
            let mut writer = MessageWriter::new();
            writer
                .write_u16(MessageType::Create.id())
                .write_u8(1)
                .write_u16(*room)
                .write_u16(send_x)
                .write_u16(*y as u16)
                .write_u8(send_dir);
            broadcast_room(&server, *room, writer.into_bytes()).await;
        }

        for (i, (room, x, y, min, max)) in ROCKS.iter().enumerate() {
            if now < rock_next[i] {
                continue;
            }
            {
                let mut rng = rand::thread_rng();
                rock_next[i] = now
                    + std::time::Duration::from_secs_f64(rng.gen_range(*min..*max));
            }
            let mut writer = MessageWriter::new();
            writer
                .write_u16(MessageType::Create.id())
                .write_u8(2)
                .write_u16(*room)
                .write_u16(*x as u16)
                .write_u16(*y as u16);
            broadcast_room(&server, *room, writer.into_bytes()).await;
        }
    }
}
