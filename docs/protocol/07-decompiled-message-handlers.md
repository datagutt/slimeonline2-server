# Decompiled Message Handlers Reference

This document contains detailed protocol information extracted from the original server's decompiled GML scripts.

## Table of Contents

1. [Authentication](#1-authentication)
2. [Movement](#2-movement)
3. [Chat](#3-chat)
4. [Economy (Shop/Sell/Bank)](#4-economy)
5. [Clan System](#5-clan-system)
6. [Mail System](#6-mail-system)
7. [Quest System](#7-quest-system)
8. [Plant System](#8-plant-system)
9. [Collectibles](#9-collectibles)
10. [Item Management](#10-item-management)
11. [Special Features](#11-special-features)
12. [BBS System](#12-bbs-system)
13. [Storage System](#13-storage-system)

---

## 1. Authentication

### MSG_LOGIN

**Client sends:**
```
version: string     // Expected: "0.106" or "ModAccess" for mods
username: string    // Converted to lowercase server-side
password: string    // Converted to lowercase server-side  
mac_address: string
```

**Special case - Mod login (version == "ModAccess"):**
```
mod_name: string
mod_password: string
```

**Server response:**
```
msg_type: ushort (MSG_LOGIN)
result_code: byte
```

| Result Code | Meaning |
|-------------|---------|
| 1 | Success (followed by account data) |
| 2 | Account does not exist |
| 3 | Wrong password / Mod already logged in |
| 4 | Player already logged in |
| 5 | Version mismatch |
| 6 | Account banned |
| 7 | IP banned |
| 8 | MAC banned |

**Success response (code 1) includes:**
```
msg_type: ushort (MSG_LOGIN)
result: byte (1)
pid: ushort
server_time: uint
motd: string
weekday: byte
hour: byte
minute: byte
name: string
x: ushort
y: ushort
cur_room: ushort
body_id: ushort
acs1: ushort
acs2: ushort
points: uint
signature: byte
quest_id: ushort
quest_step: byte
trees_planted: ushort
objects_built: ushort
emotes[5]: byte (5 values)
outfits[9]: ushort (9 values)
acs[9]: ushort (9 values)
items[9]: ushort (9 values)
tools[9]: byte (9 values)
```

**Validation logic:**
1. Check version matches GameVersion
2. Check player not already online (in global.online_list)
3. Check account file exists
4. Check account not banned
5. Check IP not in global.BanIP list
6. Check MAC not in global.BanMAC list
7. Verify password matches

---

### MSG_REGISTER

**Client sends:**
```
username: string    // Max 10 chars, stored lowercase
password: string    // Max 10 chars, stored lowercase
mac_address: string
```

**Server response:**
```
msg_type: ushort (MSG_REGISTER)
result_code: byte
```

| Result Code | Meaning |
|-------------|---------|
| 1 | Success |
| 2 | Account already exists |
| 3 | IP banned |
| 4 | MAC banned |

**Validation logic:**
1. Username length > 10: silent exit (no response)
2. Password length > 10: silent exit (no response)
3. IP ban check
4. MAC ban check
5. Account existence check

---

## 2. Movement

### MSG_MOVE_PLAYER

**Client sends:**
```
direction: byte (1-13)
[additional data varies by direction]
```

| Direction | Action | Additional Data |
|-----------|--------|-----------------|
| 1 | Move left | x: ushort, y: ushort |
| 2 | Move right | x: ushort, y: ushort |
| 3 | Jump (press up) | x: short (SIGNED!) |
| 4 | Duck (press down) | none |
| 5 | Release left | x: ushort, y: ushort |
| 6 | Release right | x: ushort, y: ushort |
| 7 | Release up | none |
| 8 | Release down | none |
| 9 | Land on ground | x: ushort, y: ushort |
| 10 | Press left in air | none |
| 11 | Press right in air | none |
| 12 | Release left in air | none |
| 13 | Release right in air | none |

**Server broadcasts to same room:**
```
msg_type: ushort (MSG_MOVE_PLAYER)
pid: ushort
direction: byte
[x: ushort] (if applicable)
[y: ushort] (if applicable)
```

**Note:** Direction 3 (jump) uses a SIGNED short for x position.

---

### MSG_POSITION

**Client sends:**
```
requester_pid: ushort
x: ushort
y: ushort
```

**Server responds to requester:**
```
msg_type: ushort (MSG_POSITION)
type: byte (2)
pid: ushort
x: ushort
y: ushort
ileft: byte
iright: byte
iup: byte
idown: byte
iup_press: byte
```

---

## 3. Chat

### MSG_CHAT

**Client sends:**
```
message: string
```

**Server broadcasts to same room:**
```
msg_type: ushort (MSG_CHAT)
pid: ushort
message: string
```

**Notes:**
- No content filtering performed
- Message is logged via `chat_log()`
- No length validation visible

---

## 4. Economy

### MSG_SHOP_BUY

**Client sends:**
```
buy_id: byte    // Shop slot index
```

**Validation:**
1. Room file must exist
2. Room must have 'Shop' section
3. Item must be in stock (stock > 0)
4. Player must have enough points
5. Player must have free inventory slot for category

**Error response (MSG_SHOP_BUY_FAIL):**
```
msg_type: ushort
error_code: byte
buy_id: byte
```

| Error Code | Meaning |
|------------|---------|
| 1 | Out of stock |
| 2 | Not enough points |

**Success response:**
```
msg_type: ushort (MSG_SHOP_BUY)
category: byte (1=Outfit, 2=Item, 3=Acs, 4=Tool)
slot: byte
item_id: ushort
price: ushort
```

**If item becomes sold out, broadcast (MSG_SHOP_STOCK):**
```
msg_type: ushort
status: byte (1 = sold out)
buy_id: byte
```

---

### MSG_SELL

**Client sends:**
```
category: byte (1=Outfits, 2=Items, 3=Acs, 4=Tools)
count: byte
slots[count]: byte (repeated)
```

**Validation:**
- Each slot must have an item (exit on empty)
- If selling equipped tool, unequip it

**Economy rules:**
- Sell price = buy_price / 3 (rounded)
- Excess over MAX_POINTS goes to bank automatically

**Server response:**
```
msg_type: ushort (MSG_SELL)
total_received: uint
```

---

### MSG_SELL_REQ_PRICES

**Client sends:**
```
category: byte (1-4)
```

**Server response:**
```
msg_type: ushort (MSG_SELL_REQ_PRICES)
prices[]: ushort (for each non-empty slot)
```

---

### MSG_BANK_PROCESS

**Client sends:**
```
action: byte (1=Deposit, 2=Withdraw, 3=Transfer)
```

**Action 1 - Deposit:**
```
amount: uint
```

**Action 2 - Withdraw:**
```
amount: uint
```

**Action 3 - Transfer:**
```
receiver_name: string
amount: uint
```

**Responses:**

Deposit/Withdraw OK:
```
msg_type: ushort
result: byte (1 or 2)
points: uint
bank_balance: uint
```

Transfer OK:
```
msg_type: ushort
result: byte (3)
new_bank_balance: uint
```

Transfer failed (receiver not found):
```
msg_type: ushort
result: byte (4)
```

**Validation:**
- Deposit: amount <= current points
- Withdraw: amount <= bank balance, respects MAX_POINTS
- Transfer: receiver must exist, amount <= bank balance

---

## 5. Clan System

### MSG_CLAN_CREATE

**Client sends:**
```
clan_name: string (3-15 characters)
```

**Requirements:**
- Player not in a clan
- Name 3-15 chars
- Name unique (case-insensitive)
- Has item 51 (Proof of Nature)
- Has item 52 (Proof of Earth)
- Has 10,000 SP

**Error response:**
```
msg_type: ushort (MSG_CLAN_CREATE)
error: byte (1 = name in use)
```

**Success - to self (MSG_CLAN_INFO type 1):**
```
msg_type: ushort
type: byte (1)
clan_id: ushort
is_leader: byte (1)
has_base: byte (0)
```

**Success - broadcast to all (MSG_CLAN_INFO type 2):**
```
msg_type: ushort
type: byte (2)
pid: ushort
clan_id: ushort
```

---

### MSG_CLAN_INVITE (Response to invitation)

**Client sends:**
```
response: byte (1=Accept, 2=Decline)
```

**On Accept - validation:**
1. Clan file must exist
2. Clan must have free slot
3. Player not already in clan

**Responses vary by outcome - see MSG_CLAN_INFO types.**

---

### MSG_CLAN_LEAVE

**Client sends:** (empty)

**Validation:**
- Must be in a clan
- Clan file must exist
- Must be in member list

---

### MSG_CLAN_DISSOLVE

**Client sends:** (empty)

**Validation:**
- Must be in a clan
- Must be clan leader

---

### MSG_CLAN_ADMIN

**Client sends:**
```
action: byte
```

**Action 1 - Kick member:**
```
member_slot: byte
```

**Action 2 - Invite player:**
```
target_pid: ushort
```
Note: 15-second cooldown between invites to same player

**Action 3 - Change colors:**
```
inner_r: byte
inner_g: byte
inner_b: byte
outer_r: byte
outer_g: byte
outer_b: byte
```

**Action 4 - Update info:**
```
show_leader: byte
info_text: string
```

**Action 5 - Update news:**
```
news_text: string
```

---

### MSG_CLAN_INFO (Information requests)

**Client sends:**
```
type: byte
```

**Type 1 - Get clan name/color:**
```
clan_id: ushort
```

Response:
```
msg_type: ushort
type: byte (3)
clan_id: ushort
name: string
inner_r: byte
inner_g: byte
inner_b: byte
outer_r: byte
outer_g: byte
outer_b: byte
```

**Type 2 - Get member list:** (no extra data)

**Type 3 - Get status:**
```
sub_type: byte (1=points, 2=full)
```

**Type 4 - Get info text:** (leader only)

**Type 5 - Get news:** (no extra data)

---

## 6. Mail System

### MSG_MAIL_SEND

**Client sends:**
```
paper_id: byte
font_color: byte
receiver_name: string (lowercase)
present_category: byte (0=none, 1=Outfit, 2=Item, 3=Acs, 4=Tool)
present_slot: ushort (inventory slot index)
attached_points: ushort (0-60000)
mail_text: string
```

**Validation:**
1. Player has enough points for paper cost
2. Player has enough points to attach
3. Receiver account exists
4. Receiver has free mailbox slot (max 50)

**Actions:**
- Deducts paper cost + attached points
- Removes attached item from sender's inventory
- Unequips tool if it was equipped
- Notifies online receiver

---

### MSG_MAILBOX

**Client sends:**
```
action: byte
```

| Action | Purpose | Extra Data |
|--------|---------|------------|
| 1 | Check for new mail | none |
| 2 | Open mailbox | none |
| 3 | Get page contents | page: byte (0-9) |
| 4 | Delete mail | mail_index: byte |
| 5 | Read mail | mail_index: byte |
| 6 | Take points directly | mail_index: byte |
| 7 | Send points to bank | mail_index: byte |
| 8 | Take present | mail_index: byte |

---

### MSG_MAIL_RECEIVER_CHECK

**Client sends:**
```
receiver_name: string
```

**Response:**
```
msg_type: ushort
account_exists: byte (0/1)
has_free_slot: byte (0/1)
```

---

### MSG_MAILPAPER_REQ

**Client sends:** (empty)

**Response:**
```
msg_type: ushort
paper_prices[]: byte (for each paper type)
```

---

## 7. Quest System

### MSG_QUEST_BEGIN

**Client sends:**
```
quest_id: byte
```

**Validation:**
- Quest must not be already cleared

---

### MSG_QUEST_CANCEL

**Client sends:** (empty)

**Validation:**
- Must have active quest

---

### MSG_QUEST_CLEAR

**Client sends:**
```
quest_id: byte
```

**Validation:**
- quest_id must match current active quest

---

### MSG_QUEST_STEP_INC

**Client sends:** (empty)

**Validation:**
- Must have active quest

---

### MSG_QUEST_REWARD

**Client sends:**
```
quest_id: byte
quest_step: byte
```

**Validation:**
1. Must have active quest
2. quest_id must match
3. quest_step must match
4. Quest-specific requirements (e.g., having specific items)

---

### MSG_QUEST_NPC_REQ

**Client sends:**
```
quest_id: byte
```

**Response:**
```
msg_type: ushort
quest_id: byte
cleared: byte (0/1)
```

---

## 8. Plant System

### MSG_PLANT_SET

**Client sends:**
```
seed_slot: byte (item inventory slot)
plant_spot: byte (location in room)
```

**Validation:**
1. Slot must have item
2. Room must have "Plant Spots" section
3. Spot must exist
4. Spot must be free
5. Item must be seed (id 9 or 24)

---

### MSG_PLANT_TAKE_FRUIT

**Client sends:**
```
plant_spot: byte
fruit_id: byte (1, 2, or 3)
```

**Validation:**
1. Player has free item slot
2. Player owns the plant
3. Fruit exists

---

### MSG_PLANT_ADD_FAIRY

**Client sends:**
```
item_slot: byte
plant_spot: byte
```

**Validation:**
1. Slot has item 10 (fairy)
2. Player owns plant
3. Plant has < 5 fairies

---

### MSG_PLANT_ADD_PINWHEEL

**Client sends:**
```
item_slot: byte
plant_spot: byte
```

**Validation:**
1. Slot has pinwheel (11, 12, or 13)
2. Player owns plant

---

## 9. Collectibles

### MSG_COLLECTIBLE_TAKE_SELF

**Client sends:**
```
collectible_id: byte
```

**Validation:**
1. Room has "Collectibles" section
2. Collectible ID exists
3. Collectible is available (avail = 1)
4. Player has free item slot

**Response to self:**
```
msg_type: ushort
slot: byte
item_id: ushort
```

**Broadcast to room (MSG_COLLECTIBLE_TAKEN):**
```
msg_type: ushort
collectible_id: byte
```

---

## 10. Item Management

### MSG_DISCARD_ITEM

**Client sends:**
```
slot: byte (1-9)
x: ushort
y: ushort
```

**Validation:**
1. Slot is 1-9
2. Slot not empty
3. Item is in discardable whitelist (items 1-61, with exceptions)

---

### MSG_DISCARDED_ITEM_TAKE

**Client sends:**
```
discarded_id: ushort
```

**Validation:**
- Player has free item slot

---

### MSG_CHANGE_OUT

**Client sends:**
```
slot: byte (1-9)
```

**Validation:**
1. Slot is 1-9
2. Slot has outfit

**Broadcasts to all players.**

---

### MSG_CHANGE_ACS

**Client sends:**
```
slot: byte (1-9)
```

**Validation:**
- Slot is 1-9

**Called with argument specifying acs1 or acs2.**

---

### MSG_TOOL_EQUIP

**Client sends:**
```
slot: byte
```

**Validation:**
- Slot has tool

---

### MSG_TOOL_UNEQUIP

**Client sends:** (empty)

---

## 11. Special Features

### MSG_EMOTE

**Client sends:**
```
emote_slot: byte (1-5)
```

**Validation:**
1. Slot is 1-5
2. Slot has emote

**Special case:** Dice emote (id 13) - server generates random 1-6.

---

### MSG_ONE_TIME_TAKE

**Client sends:**
```
item_id: byte
```

**Validation:**
1. Room has "One-Times" section
2. Player hasn't taken this before
3. Player has free slot for category

---

### MSG_NEW_PLAYER

**Client sends:**
```
requester_pid: ushort
x: ushort
y: ushort
```

**Response includes full player data (position, appearance, movement state).**

---

### MSG_ACTION (Hit/Hurt)

**Client sends:**
```
x: ushort
y: ushort
hurt_direction: ushort
hsp: short (signed)
vsp: short (signed)
```

---

### MSG_GET_WARP_INFO

**Client sends:**
```
category: byte (1=City, 2=Field, 3=Dungeon)
```

**Response:**
```
msg_type: ushort
[for each of 7 slots:]
  slot_index: byte
  price: ushort
```

---

## 12. BBS System

### MSG_BBS_POST

**Client sends:**
```
category_id: byte
title: string
text: string
```

**Validation:**
1. Room has BBS file
2. Player passes cooldown check

---

### MSG_BBS_REQUEST_MESSAGES

**Client sends:**
```
category: byte
page: ushort
```

---

### MSG_BBS_REQUEST_CATEGORIES

**Client sends:** (empty)

---

## 13. Storage System

### MSG_STORAGE_MOVE

**Client sends:**
```
category: byte (1=Outfits, 2=Items, 3=Acs, 4=Tools)
page: byte
slot_first: byte (1-9=storage, 10-18=inventory)
slot_second: byte
```

**Note:** If moving a tool slot, equipped tool is unequipped.

---

### MSG_STORAGE_REQ

**Client sends:** (empty or category-specific)

---

### MSG_STORAGE_PAGES

**Client sends:**
```
category: byte
page: byte
```
