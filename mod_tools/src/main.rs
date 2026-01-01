//! Slime Online 2 - Mod Tools CLI
//!
//! Remote administration tool for the Slime Online 2 server.
//! Connects to the server's admin API to perform moderation tasks.

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

mod client;

use client::AdminClient;

#[derive(Parser)]
#[command(name = "mod_tools")]
#[command(author = "Thomas Lekanger")]
#[command(version = "1.0.0")]
#[command(about = "CLI moderation tools for Slime Online 2 server", long_about = None)]
struct Cli {
    /// Server URL (e.g., http://localhost:8080)
    #[arg(short, long, env = "SO2_ADMIN_URL", default_value = "http://localhost:8080")]
    server: String,

    /// API key for authentication
    #[arg(short = 'k', long, env = "SO2_ADMIN_KEY")]
    api_key: Option<String>,

    /// Output format (text, json)
    #[arg(short, long, default_value = "text")]
    format: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Server management commands
    Server {
        #[command(subcommand)]
        action: ServerCommands,
    },
    /// Player management commands
    Player {
        #[command(subcommand)]
        action: PlayerCommands,
    },
    /// Ban management commands
    Ban {
        #[command(subcommand)]
        action: BanCommands,
    },
    /// Mail commands
    Mail {
        #[command(subcommand)]
        action: MailCommands,
    },
    /// Clan commands
    Clan {
        #[command(subcommand)]
        action: ClanCommands,
    },
    /// Account commands
    Account {
        #[command(subcommand)]
        action: AccountCommands,
    },
    /// Interactive mode
    Interactive,
}

#[derive(Subcommand)]
enum ServerCommands {
    /// Show server statistics
    Stats,
}

#[derive(Subcommand)]
enum PlayerCommands {
    /// List online players
    List,
    /// Get detailed player info
    Info {
        /// Player username
        username: String,
    },
    /// Kick a player
    Kick {
        /// Player username
        username: String,
        /// Reason for kick
        #[arg(short, long)]
        reason: Option<String>,
    },
    /// Ban a player
    Ban {
        /// Player username
        username: String,
        /// Ban type: ip, mac, or account
        #[arg(short = 't', long, default_value = "account")]
        ban_type: String,
        /// Reason for ban
        #[arg(short, long)]
        reason: String,
        /// Duration in hours (omit for permanent)
        #[arg(short, long)]
        duration: Option<u32>,
        /// Don't kick the player
        #[arg(long)]
        no_kick: bool,
    },
    /// Teleport a player
    Teleport {
        /// Player username
        username: String,
        /// Target room ID
        #[arg(short, long)]
        room: u16,
        /// X coordinate
        #[arg(short, long, default_value = "100")]
        x: u16,
        /// Y coordinate
        #[arg(short, long, default_value = "100")]
        y: u16,
    },
    /// Give or set points
    Points {
        /// Player username
        username: String,
        /// Amount of points
        amount: i64,
        /// Mode: set, add, subtract
        #[arg(short, long, default_value = "add")]
        mode: String,
    },
    /// Set bank balance
    Bank {
        /// Player username
        username: String,
        /// Amount
        amount: i64,
        /// Mode: set, add, subtract
        #[arg(short, long, default_value = "set")]
        mode: String,
    },
    /// Give an item to player's inventory
    GiveItem {
        /// Player username
        username: String,
        /// Category: item, outfit, accessory, tool, emote
        #[arg(short, long)]
        category: String,
        /// Inventory slot (1-9, or 1-5 for emotes)
        #[arg(short, long)]
        slot: u8,
        /// Item ID to give
        #[arg(short, long)]
        item: u16,
    },
    /// Set moderator status
    SetMod {
        /// Player username
        username: String,
        /// Enable or disable moderator status
        #[arg(long)]
        enable: bool,
    },
}

#[derive(Subcommand)]
enum BanCommands {
    /// List all bans
    List {
        /// Filter by type: ip, mac, account
        #[arg(short = 't', long)]
        ban_type: Option<String>,
        /// Include expired bans
        #[arg(long)]
        include_expired: bool,
    },
    /// Create a ban directly
    Create {
        /// Ban type: ip, mac, account
        #[arg(short = 't', long)]
        ban_type: String,
        /// Value to ban (IP, MAC, or username)
        value: String,
        /// Reason for ban
        #[arg(short, long)]
        reason: String,
        /// Duration in hours (omit for permanent)
        #[arg(short, long)]
        duration: Option<u32>,
    },
    /// Remove a ban
    Remove {
        /// Ban ID
        id: i64,
    },
}

#[derive(Subcommand)]
enum MailCommands {
    /// Send system mail to a player
    Send {
        /// Recipient username
        to: String,
        /// Message content
        message: String,
        /// Sender name
        #[arg(short, long, default_value = "System")]
        sender: String,
        /// Points to attach
        #[arg(short, long, default_value = "0")]
        points: i64,
        /// Item ID to attach
        #[arg(short, long, default_value = "0")]
        item: u16,
        /// Item category (1=outfit, 2=item, 3=accessory, 4=tool)
        #[arg(short, long, default_value = "0")]
        category: u8,
    },
    /// View a player's mailbox
    View {
        /// Player username
        username: String,
        /// Page number
        #[arg(short, long, default_value = "0")]
        page: i64,
    },
}

#[derive(Subcommand)]
enum ClanCommands {
    /// List all clans
    List,
    /// Get clan info
    Info {
        /// Clan name
        name: String,
    },
    /// Dissolve a clan
    Dissolve {
        /// Clan name
        name: String,
    },
    /// Add points to a clan
    Points {
        /// Clan name
        name: String,
        /// Points to add
        amount: i64,
    },
}

#[derive(Subcommand)]
enum AccountCommands {
    /// List accounts
    List {
        /// Search filter
        #[arg(short, long)]
        search: Option<String>,
        /// Limit results
        #[arg(short, long, default_value = "50")]
        limit: i64,
    },
    /// Get account info
    Info {
        /// Account username
        username: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let api_key = cli.api_key.unwrap_or_else(|| {
        eprintln!("{}", "Warning: No API key provided. Use --api-key or SO2_ADMIN_KEY env var.".yellow());
        String::new()
    });

    let client = AdminClient::new(&cli.server, &api_key);
    let json_output = cli.format == "json";

    match cli.command {
        Some(Commands::Server { action }) => match action {
            ServerCommands::Stats => {
                let stats = client.get_stats().await?;
                if json_output {
                    println!("{}", serde_json::to_string_pretty(&stats)?);
                } else {
                    println!("{}", "Server Statistics".bold().underline());
                    println!("  Online Players:    {}", stats.online_players.to_string().green());
                    println!("  Total Connections: {}", stats.total_connections);
                    println!("  Active Rooms:      {}", stats.rooms_active);
                }
            }
        },
        Some(Commands::Player { action }) => handle_player_command(&client, action, json_output).await?,
        Some(Commands::Ban { action }) => handle_ban_command(&client, action, json_output).await?,
        Some(Commands::Mail { action }) => handle_mail_command(&client, action, json_output).await?,
        Some(Commands::Clan { action }) => handle_clan_command(&client, action, json_output).await?,
        Some(Commands::Account { action }) => handle_account_command(&client, action, json_output).await?,
        Some(Commands::Interactive) => {
            run_interactive_mode(&client).await?;
        }
        None => {
            println!("Use --help for usage information, or 'interactive' for interactive mode.");
        }
    }

    Ok(())
}

async fn handle_player_command(client: &AdminClient, action: PlayerCommands, json: bool) -> Result<()> {
    match action {
        PlayerCommands::List => {
            let players = client.list_players().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&players)?);
            } else if players.is_empty() {
                println!("{}", "No players online.".yellow());
            } else {
                println!("{} player(s) online:\n", players.len().to_string().green());
                for p in players {
                    let mod_badge = if p.is_moderator { " [MOD]".blue().to_string() } else { String::new() };
                    println!("  {} - Room {} ({}, {}) - {} pts{}", 
                        p.username.bold(), p.room_id, p.x, p.y, p.points, mod_badge);
                }
            }
        }
        PlayerCommands::Info { username } => {
            let info = client.get_player_info(&username).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("{}", format!("Player: {}", info.username).bold().underline());
                println!("  Account ID:    {}", info.account_id);
                println!("  Character ID:  {}", info.character_id);
                println!("  Online:        {}", if info.is_online { "Yes".green() } else { "No".red() });
                println!("  Moderator:     {}", info.is_moderator);
                println!("  Banned:        {}", if info.is_banned { "Yes".red() } else { "No".green() });
                println!("  Position:      Room {} ({}, {})", info.room_id, info.x, info.y);
                if info.is_online {
                    if let (Some(r), Some(x), Some(y)) = (info.current_room, info.current_x, info.current_y) {
                        println!("  Current:       Room {} ({}, {})", r, x, y);
                    }
                }
                println!("  Points:        {}", info.points);
                println!("  Bank:          {}", info.bank_balance);
                println!("  Clan ID:       {:?}", info.clan_id);
                println!("  Created:       {}", info.created_at);
                println!("  Last Login:    {:?}", info.last_login);
            }
        }
        PlayerCommands::Kick { username, reason } => {
            let result = client.kick_player(&username, reason.as_deref()).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else if result.kicked {
                println!("{} Player {} has been kicked.", "Success!".green(), username.bold());
            } else {
                println!("{} Player {} is not online.", "Note:".yellow(), username);
            }
        }
        PlayerCommands::Ban { username, ban_type, reason, duration, no_kick } => {
            let result = client.ban_player(&username, &ban_type, &reason, duration, !no_kick).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("{} Player {} has been banned (ID: {}).", "Success!".green(), username.bold(), result.ban_id);
                if result.kicked {
                    println!("  Player was also kicked from the server.");
                }
            }
        }
        PlayerCommands::Teleport { username, room, x, y } => {
            let result = client.teleport_player(&username, room, x, y).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else if result.teleported {
                let status = if result.was_online { "(online)" } else { "(offline - saved for next login)" };
                println!("{} Teleported {} to Room {} ({}, {}) {}", 
                    "Success!".green(), username.bold(), room, x, y, status);
            }
        }
        PlayerCommands::Points { username, amount, mode } => {
            let result = client.set_points(&username, amount, &mode).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("{} Points {} queued for {}: {} {} points", 
                    "Success!".green(), mode, username.bold(), 
                    if mode == "subtract" { "-" } else { "+" }, amount.abs());
            }
        }
        PlayerCommands::Bank { username, amount, mode } => {
            let result = client.set_bank(&username, amount, &mode).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("{} Bank {} queued for {}", "Success!".green(), mode, username.bold());
            }
        }
        PlayerCommands::GiveItem { username, category, slot, item } => {
            let result = client.set_inventory(&username, &category, slot, item).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("{} Item {} added to {} slot {} for {}", 
                    "Success!".green(), item, category, slot, username.bold());
                println!("  Note: Player may need to relog to see the item.");
            }
        }
        PlayerCommands::SetMod { username, enable } => {
            let result = client.set_moderator(&username, enable).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let status = if enable { "granted".green() } else { "revoked".red() };
                println!("{} Moderator status {} for {}", "Success!".green(), status, username.bold());
            }
        }
    }
    Ok(())
}

async fn handle_ban_command(client: &AdminClient, action: BanCommands, json: bool) -> Result<()> {
    match action {
        BanCommands::List { ban_type, include_expired } => {
            let bans = client.list_bans(ban_type.as_deref(), include_expired).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&bans)?);
            } else if bans.is_empty() {
                println!("{}", "No bans found.".yellow());
            } else {
                println!("{} ban(s):\n", bans.len());
                for b in bans {
                    let expired = if b.is_expired { " [EXPIRED]".red().to_string() } else { String::new() };
                    let duration = b.expires_at.as_deref().unwrap_or("permanent");
                    println!("  [{}] {} {} - {} (until: {}){}", 
                        b.id, b.ban_type.bold(), b.value, b.reason, duration, expired);
                }
            }
        }
        BanCommands::Create { ban_type, value, reason, duration } => {
            let result = client.create_ban(&ban_type, &value, &reason, duration).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("{} Ban created with ID {}", "Success!".green(), result.id);
            }
        }
        BanCommands::Remove { id } => {
            let result = client.delete_ban(id).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else if result.deleted {
                println!("{} Ban {} has been removed.", "Success!".green(), id);
            } else {
                println!("{} Ban {} not found.", "Error:".red(), id);
            }
        }
    }
    Ok(())
}

async fn handle_mail_command(client: &AdminClient, action: MailCommands, json: bool) -> Result<()> {
    match action {
        MailCommands::Send { to, message, sender, points, item, category } => {
            let result = client.send_mail(&to, &message, &sender, points, item, category).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("{} Mail sent to {}", "Success!".green(), to.bold());
            }
        }
        MailCommands::View { username, page } => {
            let mailbox = client.get_mailbox(&username, page).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&mailbox)?);
            } else {
                println!("{} - {} total, {} unread\n", 
                    format!("Mailbox for {}", username).bold().underline(),
                    mailbox.total, mailbox.unread);
                if mailbox.mail.is_empty() {
                    println!("{}", "  No mail on this page.".yellow());
                } else {
                    for m in mailbox.mail {
                        let read = if m.is_read { "" } else { " [NEW]" };
                        println!("  [{}] From: {}{}", m.id, m.from.bold(), read.green());
                        println!("       {}", m.message);
                        if m.points > 0 || m.item_id > 0 {
                            println!("       Attachments: {} pts, item {}", m.points, m.item_id);
                        }
                        println!();
                    }
                }
            }
        }
    }
    Ok(())
}

async fn handle_clan_command(client: &AdminClient, action: ClanCommands, json: bool) -> Result<()> {
    match action {
        ClanCommands::List => {
            let clans = client.list_clans().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&clans)?);
            } else if clans.is_empty() {
                println!("{}", "No clans found.".yellow());
            } else {
                println!("{} clan(s):\n", clans.len());
                for c in clans {
                    println!("  {} - {}/{} members, {} pts", 
                        c.name.bold(), c.member_count, c.max_members, c.points);
                }
            }
        }
        ClanCommands::Info { name } => {
            let clan = client.get_clan(&name).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&clan)?);
            } else {
                println!("{}", format!("Clan: {}", clan.name).bold().underline());
                println!("  ID:          {}", clan.id);
                println!("  Leader ID:   {}", clan.leader_id);
                println!("  Members:     {}", clan.members.len());
                println!("  Max Members: {}", clan.max_members);
                println!("  Points:      {}", clan.points);
                println!("  Level:       {}", clan.level);
                println!("  Description: {:?}", clan.description);
                println!("  News:        {:?}", clan.news);
                println!("  Created:     {}", clan.created_at);
                println!("\n  Members:");
                for m in &clan.members {
                    let leader = if m.is_leader { " [LEADER]".yellow().to_string() } else { String::new() };
                    println!("    - {}{}", m.username, leader);
                }
            }
        }
        ClanCommands::Dissolve { name } => {
            let result = client.dissolve_clan(&name).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("{} Clan {} dissolved. {} members removed.", 
                    "Success!".green(), name.bold(), result.members_removed);
            }
        }
        ClanCommands::Points { name, amount } => {
            let result = client.add_clan_points(&name, amount).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("{} Added {} points to clan {}. New total: {}", 
                    "Success!".green(), amount, name.bold(), result.new_total);
            }
        }
    }
    Ok(())
}

async fn handle_account_command(client: &AdminClient, action: AccountCommands, json: bool) -> Result<()> {
    match action {
        AccountCommands::List { search, limit } => {
            let accounts = client.list_accounts(search.as_deref(), limit).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&accounts)?);
            } else if accounts.is_empty() {
                println!("{}", "No accounts found.".yellow());
            } else {
                println!("{} account(s):\n", accounts.len());
                for a in accounts {
                    let banned = if a.is_banned { " [BANNED]".red().to_string() } else { String::new() };
                    println!("  {} - created {}{}", a.username.bold(), a.created_at, banned);
                }
            }
        }
        AccountCommands::Info { username } => {
            let account = client.get_account(&username).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&account)?);
            } else {
                println!("{}", format!("Account: {}", account.username).bold().underline());
                println!("  ID:           {}", account.id);
                println!("  MAC:          {}", account.mac_address);
                println!("  Banned:       {}", if account.is_banned { "Yes".red() } else { "No".green() });
                if let Some(reason) = &account.ban_reason {
                    println!("  Ban Reason:   {}", reason);
                }
                println!("  Has Character:{}", account.has_character);
                println!("  Online:       {}", if account.is_online { "Yes".green() } else { "No".normal() });
                println!("  Created:      {}", account.created_at);
                println!("  Last Login:   {:?}", account.last_login);
            }
        }
    }
    Ok(())
}

async fn run_interactive_mode(client: &AdminClient) -> Result<()> {
    use rustyline::DefaultEditor;

    println!("{}", "\nSlime Online 2 - Mod Tools Interactive Mode".bold());
    println!("Type 'help' for available commands, 'quit' to exit.\n");

    let mut rl = DefaultEditor::new()?;

    loop {
        let readline = rl.readline("mod> ");
        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                rl.add_history_entry(line)?;

                let parts: Vec<&str> = line.split_whitespace().collect();
                match parts.as_slice() {
                    ["quit"] | ["exit"] | ["q"] => {
                        println!("Goodbye!");
                        break;
                    }
                    ["help"] | ["?"] => {
                        print_interactive_help();
                    }
                    ["stats"] => {
                        match client.get_stats().await {
                            Ok(stats) => {
                                println!("Online: {}, Connections: {}, Rooms: {}", 
                                    stats.online_players, stats.total_connections, stats.rooms_active);
                            }
                            Err(e) => println!("{} {}", "Error:".red(), e),
                        }
                    }
                    ["players"] => {
                        match client.list_players().await {
                            Ok(players) => {
                                if players.is_empty() {
                                    println!("No players online.");
                                } else {
                                    for p in players {
                                        println!("  {} - Room {} - {} pts", p.username, p.room_id, p.points);
                                    }
                                }
                            }
                            Err(e) => println!("{} {}", "Error:".red(), e),
                        }
                    }
                    ["player", username] => {
                        match client.get_player_info(username).await {
                            Ok(info) => {
                                let online = if info.is_online { "online" } else { "offline" };
                                println!("{} ({}) - {} pts, bank {}", info.username, online, info.points, info.bank_balance);
                            }
                            Err(e) => println!("{} {}", "Error:".red(), e),
                        }
                    }
                    ["kick", username] => {
                        match client.kick_player(username, None).await {
                            Ok(r) => {
                                if r.kicked {
                                    println!("Kicked {}", username);
                                } else {
                                    println!("{} is not online", username);
                                }
                            }
                            Err(e) => println!("{} {}", "Error:".red(), e),
                        }
                    }
                    ["give", username, amount] => {
                        if let Ok(pts) = amount.parse::<i64>() {
                            match client.set_points(username, pts, "add").await {
                                Ok(_) => println!("Gave {} points to {}", pts, username),
                                Err(e) => println!("{} {}", "Error:".red(), e),
                            }
                        } else {
                            println!("Invalid amount");
                        }
                    }
                    ["tp", username, room] => {
                        if let Ok(r) = room.parse::<u16>() {
                            match client.teleport_player(username, r, 100, 100).await {
                                Ok(_) => println!("Teleported {} to room {} (100, 100)", username, r),
                                Err(e) => println!("{} {}", "Error:".red(), e),
                            }
                        } else {
                            println!("Invalid room ID");
                        }
                    }
                    ["tp", username, room, x, y] => {
                        let room_parsed = room.parse::<u16>();
                        let x_parsed = x.parse::<u16>();
                        let y_parsed = y.parse::<u16>();
                        
                        match (room_parsed, x_parsed, y_parsed) {
                            (Ok(r), Ok(px), Ok(py)) => {
                                match client.teleport_player(username, r, px, py).await {
                                    Ok(_) => println!("Teleported {} to room {} ({}, {})", username, r, px, py),
                                    Err(e) => println!("{} {}", "Error:".red(), e),
                                }
                            }
                            _ => println!("Invalid arguments. Usage: tp <name> <room> <x> <y>"),
                        }
                    }
                    ["bans"] => {
                        match client.list_bans(None, false).await {
                            Ok(bans) => {
                                if bans.is_empty() {
                                    println!("No active bans.");
                                } else {
                                    for b in bans {
                                        println!("  [{}] {} {} - {}", b.id, b.ban_type, b.value, b.reason);
                                    }
                                }
                            }
                            Err(e) => println!("{} {}", "Error:".red(), e),
                        }
                    }
                    ["unban", id] => {
                        if let Ok(ban_id) = id.parse::<i64>() {
                            match client.delete_ban(ban_id).await {
                                Ok(r) => {
                                    if r.deleted {
                                        println!("Ban {} removed", ban_id);
                                    } else {
                                        println!("Ban {} not found", ban_id);
                                    }
                                }
                                Err(e) => println!("{} {}", "Error:".red(), e),
                            }
                        } else {
                            println!("Invalid ban ID");
                        }
                    }
                    ["clans"] => {
                        match client.list_clans().await {
                            Ok(clans) => {
                                if clans.is_empty() {
                                    println!("No clans.");
                                } else {
                                    for c in clans {
                                        println!("  {} - {}/{} members", c.name, c.member_count, c.max_members);
                                    }
                                }
                            }
                            Err(e) => println!("{} {}", "Error:".red(), e),
                        }
                    }
                    _ => {
                        println!("Unknown command. Type 'help' for available commands.");
                    }
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                println!("Goodbye!");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}

fn print_interactive_help() {
    println!("{}", "Available commands:".bold());
    println!("  stats                    - Show server statistics");
    println!("  players                  - List online players");
    println!("  player <name>            - Show player info");
    println!("  kick <name>              - Kick a player");
    println!("  give <name> <pts>        - Give points to a player");
    println!("  tp <name> <room>         - Teleport player to room (100, 100)");
    println!("  tp <name> <room> <x> <y> - Teleport player to room at coordinates");
    println!("  bans                     - List active bans");
    println!("  unban <id>               - Remove a ban");
    println!("  clans                    - List all clans");
    println!("  help                     - Show this help");
    println!("  quit                     - Exit interactive mode");
    println!();
    println!("For advanced commands, use the CLI directly:");
    println!("  mod_tools --help");
}
