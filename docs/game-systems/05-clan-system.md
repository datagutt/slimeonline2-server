# Clan System

**See:** [`../protocol/04-message-catalog.md`](../protocol/04-message-catalog.md) - Clan section

## Clan Messages (7 total)

- MSG_CLAN_CREATE (110)
- MSG_CLAN_INVITE (111)
- MSG_CLAN_ACCEPT (112)
- MSG_CLAN_DECLINE (113)
- MSG_CLAN_LEAVE (114)
- MSG_CLAN_KICK (115)
- MSG_CLAN_INFO (116)

## Database

See [`../database/05-clans.md`](../database/05-clans.md) for clan table schema.

## Validation

Clan name 3-20 chars, leader permissions check. See [`../security/02-server-validation.md`](../security/02-server-validation.md).
