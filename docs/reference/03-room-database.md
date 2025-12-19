# Room Database

**See:** Client files at `/slime2_decompile.gmx/scripts/db_room_name.gml`

## Main Rooms

| ID | Name | Type |
|----|------|------|
| 32 | City Mountain Feet 1 | Outdoor |
| 33 | City Mountain Feet 2 | Outdoor |
| 42 | New City | Town |
| 44 | New City Outfits | Shop |
| 45 | New City Accessories | Shop |
| 46 | New City Items | Shop |
| 51 | New City Warpcenter | Special |

Extract full list from client `db_room_name.gml` script.

Room IDs are referenced in:
- [`../protocol/04-message-catalog.md`](../protocol/04-message-catalog.md) - MSG_CHANGE_ROOM
- [`../database/01-schema-overview.md`](../database/01-schema-overview.md) - characters.room_id
