//! HTTP client for the admin API

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Admin API client
pub struct AdminClient {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

/// Standard API response wrapper
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn into_result(self) -> Result<T> {
        if self.success {
            self.data
                .ok_or_else(|| anyhow::anyhow!("No data in response"))
        } else {
            Err(anyhow::anyhow!(self
                .error
                .unwrap_or_else(|| "Unknown error".to_string())))
        }
    }
}

// Response types
#[derive(Debug, Deserialize, Serialize)]
pub struct ServerStats {
    pub online_players: usize,
    pub total_connections: usize,
    pub rooms_active: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OnlinePlayer {
    pub username: String,
    pub player_id: u16,
    pub room_id: u16,
    pub x: u16,
    pub y: u16,
    pub points: u32,
    pub is_moderator: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerInfo {
    pub account_id: i64,
    pub username: String,
    pub created_at: String,
    pub last_login: Option<String>,
    pub is_banned: bool,
    pub ban_reason: Option<String>,
    pub character_id: i64,
    pub x: i16,
    pub y: i16,
    pub room_id: i16,
    pub body_id: i16,
    pub acs1_id: i16,
    pub acs2_id: i16,
    pub points: i64,
    pub bank_balance: i64,
    pub is_moderator: bool,
    pub clan_id: Option<i64>,
    pub is_online: bool,
    pub current_room: Option<u16>,
    pub current_x: Option<u16>,
    pub current_y: Option<u16>,
    pub items: [u16; 9],
    pub outfits: [u16; 9],
    pub accessories: [u16; 9],
    pub tools: [u8; 9],
    pub emotes: [u8; 5],
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KickResponse {
    pub kicked: bool,
    pub was_online: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BanResponse {
    pub banned: bool,
    pub ban_id: i64,
    pub kicked: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TeleportResponse {
    pub teleported: bool,
    pub was_online: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct QueuedResponse {
    pub queued: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ModeratorResponse {
    pub updated: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BanRecord {
    pub id: i64,
    pub ban_type: String,
    pub value: String,
    pub reason: String,
    pub banned_by: Option<String>,
    pub banned_at: String,
    pub expires_at: Option<String>,
    pub is_expired: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateBanResponse {
    pub id: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeleteBanResponse {
    pub deleted: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MailEntry {
    pub id: i64,
    pub from: String,
    pub message: String,
    pub item_id: i64,
    pub item_category: i64,
    pub points: i64,
    pub is_read: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MailboxResponse {
    pub total: i64,
    pub unread: i64,
    pub mail: Vec<MailEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClanSummary {
    pub id: i64,
    pub name: String,
    pub leader_id: i64,
    pub member_count: i64,
    pub max_members: i64,
    pub points: i64,
    pub level: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClanMemberInfo {
    pub character_id: i64,
    pub username: String,
    pub is_leader: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClanDetail {
    pub id: i64,
    pub name: String,
    pub leader_id: i64,
    pub color_inner: i64,
    pub color_outer: i64,
    pub level: i64,
    pub points: i64,
    pub max_members: i64,
    pub description: Option<String>,
    pub news: Option<String>,
    pub show_name: bool,
    pub has_base: bool,
    pub created_at: String,
    pub members: Vec<ClanMemberInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DissolveClanResponse {
    pub dissolved: bool,
    pub members_removed: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AddPointsResponse {
    pub new_total: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccountSummary {
    pub id: i64,
    pub username: String,
    pub is_banned: bool,
    pub created_at: String,
    pub last_login: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccountDetail {
    pub id: i64,
    pub username: String,
    pub mac_address: String,
    pub is_banned: bool,
    pub ban_reason: Option<String>,
    pub created_at: String,
    pub last_login: Option<String>,
    pub has_character: bool,
    pub character_id: Option<i64>,
    pub is_online: bool,
}

impl AdminClient {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            client: reqwest::Client::new(),
        }
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .context("Failed to send request")?;

        let status = response.status();
        let body = response.text().await.context("Failed to read response")?;

        if !status.is_success() {
            // Try to parse as API error
            if let Ok(api_resp) = serde_json::from_str::<ApiResponse<()>>(&body) {
                return Err(anyhow::anyhow!(api_resp
                    .error
                    .unwrap_or_else(|| format!("HTTP {}", status))));
            }
            return Err(anyhow::anyhow!("HTTP {}: {}", status, body));
        }

        let api_response: ApiResponse<T> =
            serde_json::from_str(&body).context("Failed to parse response")?;
        api_response.into_result()
    }

    async fn post<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .json(body)
            .send()
            .await
            .context("Failed to send request")?;

        let status = response.status();
        let resp_body = response.text().await.context("Failed to read response")?;

        if !status.is_success() {
            if let Ok(api_resp) = serde_json::from_str::<ApiResponse<()>>(&resp_body) {
                return Err(anyhow::anyhow!(api_resp
                    .error
                    .unwrap_or_else(|| format!("HTTP {}", status))));
            }
            return Err(anyhow::anyhow!("HTTP {}: {}", status, resp_body));
        }

        let api_response: ApiResponse<T> =
            serde_json::from_str(&resp_body).context("Failed to parse response")?;
        api_response.into_result()
    }

    async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .delete(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .context("Failed to send request")?;

        let status = response.status();
        let body = response.text().await.context("Failed to read response")?;

        if !status.is_success() {
            if let Ok(api_resp) = serde_json::from_str::<ApiResponse<()>>(&body) {
                return Err(anyhow::anyhow!(api_resp
                    .error
                    .unwrap_or_else(|| format!("HTTP {}", status))));
            }
            return Err(anyhow::anyhow!("HTTP {}: {}", status, body));
        }

        let api_response: ApiResponse<T> =
            serde_json::from_str(&body).context("Failed to parse response")?;
        api_response.into_result()
    }

    // Server endpoints
    pub async fn get_stats(&self) -> Result<ServerStats> {
        self.get("/api/server/stats").await
    }

    // Player endpoints
    pub async fn list_players(&self) -> Result<Vec<OnlinePlayer>> {
        self.get("/api/players").await
    }

    pub async fn get_player_info(&self, username: &str) -> Result<PlayerInfo> {
        self.get(&format!("/api/players/{}", username)).await
    }

    pub async fn kick_player(&self, username: &str, reason: Option<&str>) -> Result<KickResponse> {
        #[derive(Serialize)]
        struct KickReq<'a> {
            reason: Option<&'a str>,
        }
        self.post(
            &format!("/api/players/{}/kick", username),
            &KickReq { reason },
        )
        .await
    }

    pub async fn ban_player(
        &self,
        username: &str,
        ban_type: &str,
        reason: &str,
        duration_hours: Option<u32>,
        kick: bool,
    ) -> Result<BanResponse> {
        #[derive(Serialize)]
        struct BanReq<'a> {
            ban_type: &'a str,
            reason: &'a str,
            duration_hours: Option<u32>,
            kick: bool,
        }
        self.post(
            &format!("/api/players/{}/ban", username),
            &BanReq {
                ban_type,
                reason,
                duration_hours,
                kick,
            },
        )
        .await
    }

    pub async fn teleport_player(
        &self,
        username: &str,
        room_id: u16,
        x: u16,
        y: u16,
    ) -> Result<TeleportResponse> {
        #[derive(Serialize)]
        struct TeleportReq {
            room_id: u16,
            x: u16,
            y: u16,
        }
        self.post(
            &format!("/api/players/{}/teleport", username),
            &TeleportReq { room_id, x, y },
        )
        .await
    }

    pub async fn set_points(
        &self,
        username: &str,
        points: i64,
        mode: &str,
    ) -> Result<QueuedResponse> {
        #[derive(Serialize)]
        struct PointsReq<'a> {
            points: i64,
            mode: &'a str,
        }
        self.post(
            &format!("/api/players/{}/points", username),
            &PointsReq { points, mode },
        )
        .await
    }

    pub async fn set_bank(
        &self,
        username: &str,
        balance: i64,
        mode: &str,
    ) -> Result<QueuedResponse> {
        #[derive(Serialize)]
        struct BankReq<'a> {
            balance: i64,
            mode: &'a str,
        }
        self.post(
            &format!("/api/players/{}/bank", username),
            &BankReq { balance, mode },
        )
        .await
    }

    pub async fn set_inventory(
        &self,
        username: &str,
        category: &str,
        slot: u8,
        item_id: u16,
    ) -> Result<QueuedResponse> {
        #[derive(Serialize)]
        struct InvReq<'a> {
            category: &'a str,
            slot: u8,
            item_id: u16,
        }
        self.post(
            &format!("/api/players/{}/inventory", username),
            &InvReq {
                category,
                slot,
                item_id,
            },
        )
        .await
    }

    pub async fn set_moderator(
        &self,
        username: &str,
        is_moderator: bool,
    ) -> Result<ModeratorResponse> {
        #[derive(Serialize)]
        struct ModReq {
            is_moderator: bool,
        }
        self.post(
            &format!("/api/players/{}/moderator", username),
            &ModReq { is_moderator },
        )
        .await
    }

    // Ban endpoints
    pub async fn list_bans(
        &self,
        ban_type: Option<&str>,
        include_expired: bool,
    ) -> Result<Vec<BanRecord>> {
        let mut path = "/api/bans".to_string();
        let mut params = vec![];
        if let Some(bt) = ban_type {
            params.push(format!("ban_type={}", bt));
        }
        if include_expired {
            params.push("include_expired=true".to_string());
        }
        if !params.is_empty() {
            path = format!("{}?{}", path, params.join("&"));
        }
        self.get(&path).await
    }

    pub async fn create_ban(
        &self,
        ban_type: &str,
        value: &str,
        reason: &str,
        duration_hours: Option<u32>,
    ) -> Result<CreateBanResponse> {
        #[derive(Serialize)]
        struct CreateBanReq<'a> {
            ban_type: &'a str,
            value: &'a str,
            reason: &'a str,
            duration_hours: Option<u32>,
        }
        self.post(
            "/api/bans",
            &CreateBanReq {
                ban_type,
                value,
                reason,
                duration_hours,
            },
        )
        .await
    }

    pub async fn delete_ban(&self, id: i64) -> Result<DeleteBanResponse> {
        self.delete(&format!("/api/bans/{}", id)).await
    }

    // Mail endpoints
    pub async fn send_mail(
        &self,
        to: &str,
        message: &str,
        sender: &str,
        points: i64,
        item_id: u16,
        item_category: u8,
    ) -> Result<QueuedResponse> {
        #[derive(Serialize)]
        struct MailReq<'a> {
            to: &'a str,
            message: &'a str,
            sender: &'a str,
            points: i64,
            item_id: u16,
            item_category: u8,
        }
        self.post(
            "/api/mail/send",
            &MailReq {
                to,
                message,
                sender,
                points,
                item_id,
                item_category,
            },
        )
        .await
    }

    pub async fn get_mailbox(&self, username: &str, page: i64) -> Result<MailboxResponse> {
        self.get(&format!("/api/mail/{}?page={}", username, page))
            .await
    }

    // Clan endpoints
    pub async fn list_clans(&self) -> Result<Vec<ClanSummary>> {
        self.get("/api/clans").await
    }

    pub async fn get_clan(&self, name: &str) -> Result<ClanDetail> {
        self.get(&format!("/api/clans/{}", name)).await
    }

    pub async fn dissolve_clan(&self, name: &str) -> Result<DissolveClanResponse> {
        self.delete(&format!("/api/clans/{}", name)).await
    }

    pub async fn add_clan_points(&self, name: &str, points: i64) -> Result<AddPointsResponse> {
        #[derive(Serialize)]
        struct PointsReq {
            points: i64,
        }
        self.post(
            &format!("/api/clans/{}/points", name),
            &PointsReq { points },
        )
        .await
    }

    // Account endpoints
    pub async fn list_accounts(
        &self,
        search: Option<&str>,
        limit: i64,
    ) -> Result<Vec<AccountSummary>> {
        let mut path = format!("/api/accounts?limit={}", limit);
        if let Some(s) = search {
            path = format!("{}&search={}", path, s);
        }
        self.get(&path).await
    }

    pub async fn get_account(&self, username: &str) -> Result<AccountDetail> {
        self.get(&format!("/api/accounts/{}", username)).await
    }
}
