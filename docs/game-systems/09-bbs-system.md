# Bulletin Board System

**See:** [`../protocol/04-message-catalog.md`](../protocol/04-message-catalog.md) - BBS section

## BBS Messages (8 total)

- MSG_BBS_POST (96)
- MSG_BBS_REQUEST_MESSAGES (97)
- MSG_BBS_MESSAGES (98)
- MSG_BBS_REPORT (99)
- MSG_BBS_DELETE (100)

## Features

- Post messages (title + content)
- View posts
- Report inappropriate content
- Delete own posts

## Database

See [`../database/06-mail-bbs.md`](../database/06-mail-bbs.md) for bbs_posts table.

## Moderation

Reported posts flagged for moderator review.
