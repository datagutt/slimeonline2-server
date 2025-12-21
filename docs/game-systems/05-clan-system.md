# Clan System

**See:** [`../protocol/04-message-catalog.md`](../protocol/04-message-catalog.md) - Clan section

## Clan Messages (6 total)

- MSG_CLAN_CREATE (126) - Create a new clan
- MSG_CLAN_DISSOLVE (127) - Dissolve the clan (leader only)
- MSG_CLAN_INVITE (128) - Invite player to clan
- MSG_CLAN_LEAVE (129) - Leave the clan
- MSG_CLAN_INFO (130) - Clan information updates
- MSG_CLAN_ADMIN (131) - Admin actions (kick member, promote, etc.)

## Clan Creation Requirements (Original Server Defaults)

To create a clan, a player needs:
- **10,000 Slime Points** (deducted upon creation)
- **Proof of Nature** (Item ID 51) - consumed
- **Proof of Earth** (Item ID 52) - consumed
- Clan name must be 3-15 characters

## Original Server File Format

The original server stores clans in `.soc` files:

```ini
[Status]
Name=ClanName
icolor_r=0
icolor_g=255
icolor_b=0
ocolor_r=0
ocolor_g=0
ocolor_b=0
Level=1
ClanPoints=0
ClanCreated=01.07.2009
News=No news
Info=A new clan
Show Leader=1

[Members]
Unlocked=3
Leader=123
Member1=-1
Member2=-1
Member3=-1
...

[Clan Base]
Unlocked=0
```

## Rust Server Implementation

Our Rust server uses **config files** for clan rules. The database stores clan data and membership.

### clans.toml
```toml
[creation]
cost = 10000
required_items = [51, 52]  # Proof of Nature, Proof of Earth

[limits]
min_name_length = 3
max_name_length = 15
initial_member_slots = 3
max_member_slots = 10
```

See [`../database/05-clans.md`](../database/05-clans.md) for the database schema.

## MSG_CLAN_INFO Subtypes

The server sends different subtypes via `writebyte()`:
- **1**: Player joined a clan (self notification)
- **2**: Another player is in a clan (broadcast to others)
- **3**: Clan member list info
- **4**: Clan colors info
- **5**: Clan news/info text
- **6**: Player left/kicked from clan

## Validation

- Clan name: 3-15 characters (configurable)
- Max 10 members (unlockable slots)
- Leader permissions required for admin actions
- Player must not already be in a clan to create/join

See [`../security/02-server-validation.md`](../security/02-server-validation.md).
