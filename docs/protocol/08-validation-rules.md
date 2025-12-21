# Server Validation Rules

This document describes the validation logic extracted from the original server's decompiled scripts.

## Overview

The original server uses a combination of:
- **Silent exits** (`exit;`) - No response sent, operation simply ignored
- **Hack alerts** - Logged suspicious activity, sometimes with silent exit
- **Error responses** - Explicit error codes sent back to client

## Validation by Category

### 1. Authentication Validation

#### Login
| Check | Failure Behavior |
|-------|------------------|
| Version mismatch | Response code 5, disconnect |
| Player already online | Response code 4, disconnect |
| Account doesn't exist | Response code 2, disconnect |
| Account banned | Response code 6, disconnect |
| IP banned | Response code 7, disconnect |
| MAC banned | Response code 8, disconnect |
| Wrong password | Response code 3, disconnect |

#### Registration
| Check | Failure Behavior |
|-------|------------------|
| Username > 10 chars | Silent exit |
| Password > 10 chars | Silent exit |
| IP banned | Response code 3, disconnect |
| MAC banned | Response code 4, disconnect |
| Account exists | Response code 2, disconnect |

---

### 2. Inventory Slot Validation

Most inventory operations validate slot bounds:

```
Valid slot range: 1-9 (for regular inventory)
Valid slot range: 1-18 for storage moves (1-9 storage, 10-18 inventory)
```

| Check | Failure Behavior |
|-------|------------------|
| Slot < 1 or > 9 | Hack alert + exit |
| Slot is empty | Hack alert + exit (usually) |

**Affected messages:**
- MSG_DISCARD_ITEM
- MSG_CHANGE_OUT
- MSG_CHANGE_ACS
- MSG_SELL (per-slot check)
- MSG_TOOL_EQUIP
- MSG_EMOTE

---

### 3. Shop Purchase Validation

| Check | Failure Behavior |
|-------|------------------|
| Room file doesn't exist | Silent exit |
| Room has no 'Shop' section | Hack alert + exit |
| Item out of stock | MSG_SHOP_BUY_FAIL code 1 |
| Not enough points | MSG_SHOP_BUY_FAIL code 2 |
| No free inventory slot | Hack alert + exit |

---

### 4. Selling Validation

| Check | Failure Behavior |
|-------|------------------|
| Slot is empty | Silent exit (per item) |

**Special handling:**
- If selling equipped tool, unequips automatically
- Excess points over MAX_POINTS go to bank

---

### 5. Banking Validation

#### Deposit
| Check | Failure Behavior |
|-------|------------------|
| Amount > current points | Hack alert (no response) |

#### Withdraw
| Check | Failure Behavior |
|-------|------------------|
| Amount > bank balance | Hack alert (no response) |

**Special handling:**
- If withdrawal would exceed MAX_POINTS, only withdraws up to limit

#### Transfer
| Check | Failure Behavior |
|-------|------------------|
| Amount > bank balance | Hack alert (no response) |
| Receiver doesn't exist | Response code 4 |

---

### 6. Clan Validation

#### Create
| Check | Failure Behavior |
|-------|------------------|
| Already in clan | Silent exit (code doesn't run) |
| Name < 3 chars | Hack alert + exit |
| Name > 15 chars | Hack alert + exit |
| Missing Proof of Nature (51) | Hack alert + exit |
| Missing Proof of Earth (52) | Hack alert + exit |
| Points < 10,000 | Hack alert + exit |
| Name already exists | Response code 1 |

#### Admin Actions
| Check | Failure Behavior |
|-------|------------------|
| Not in a clan | Silent exit |
| Clan file doesn't exist | Silent exit |
| Not clan leader | Hack alert + exit |

#### Kick Member
| Check | Failure Behavior |
|-------|------------------|
| No member at slot | Silent exit |

#### Invite
| Check | Failure Behavior |
|-------|------------------|
| Clan has no free slot | Silent (doesn't send invite) |
| Target already in clan | Silent exit |
| Target in cooldown period | Silent (doesn't send invite) |

**Invite cooldown:** 15 seconds per target

#### Leave
| Check | Failure Behavior |
|-------|------------------|
| Not in a clan | Silent exit |
| Clan file doesn't exist | Silent exit |
| Not in member list | Silent exit |

#### Dissolve
| Check | Failure Behavior |
|-------|------------------|
| Not in a clan | Silent (code doesn't run) |
| Clan file doesn't exist | Silent (code doesn't run) |
| Not clan leader | Hack alert + exit |

---

### 7. Mail Validation

#### Send Mail
| Check | Failure Behavior |
|-------|------------------|
| Not enough points for paper | Hack alert + exit |
| Not enough points to attach | Hack alert + exit |
| Receiver doesn't exist | Silent exit |
| Receiver mailbox full | Silent (mail not sent) |

#### Take Points (action 6)
| Check | Failure Behavior |
|-------|------------------|
| Mail has no points | Hack alert + exit |

#### Send to Bank (action 7)
| Check | Failure Behavior |
|-------|------------------|
| Mail has no points | Hack alert + exit |

#### Take Present (action 8)
| Check | Failure Behavior |
|-------|------------------|
| No present attached | Hack alert + exit |
| No free slot | Hack alert + exit |

---

### 8. Quest Validation

#### Begin Quest
| Check | Failure Behavior |
|-------|------------------|
| Quest already cleared | Hack alert + exit |

#### Cancel Quest
| Check | Failure Behavior |
|-------|------------------|
| No active quest | Hack alert + exit |

#### Clear Quest
| Check | Failure Behavior |
|-------|------------------|
| quest_id mismatch | Hack alert + exit |

#### Step Increment
| Check | Failure Behavior |
|-------|------------------|
| No active quest | Hack alert + exit |

#### Claim Reward
| Check | Failure Behavior |
|-------|------------------|
| No active quest | Hack alert + exit |
| quest_id mismatch | Hack alert + exit |
| quest_step mismatch | Hack alert + exit |
| Requirements not met | Hack alert + exit |

---

### 9. Plant Validation

#### Plant Seed
| Check | Failure Behavior |
|-------|------------------|
| Slot is empty | Silent exit |
| Room has no plant spots | Silent exit |
| Spot doesn't exist | Silent exit |
| Spot not free | Returns seed to player |
| Item not a seed (9, 24) | Silent (not planted) |

#### Take Fruit
| Check | Failure Behavior |
|-------|------------------|
| No free item slot | Silent exit |
| Not plant owner | Silent (code doesn't run) |
| Fruit doesn't exist | Silent (code doesn't run) |

#### Add Fairy
| Check | Failure Behavior |
|-------|------------------|
| Slot is empty | Silent exit |
| Item not fairy (10) | Silent exit |
| Not plant owner | Silent exit |
| Already 5 fairies | Silent exit |

#### Add Pinwheel
| Check | Failure Behavior |
|-------|------------------|
| Slot is empty | Silent exit |
| Item not pinwheel (11,12,13) | Silent exit |
| Not plant owner | Silent exit |

---

### 10. Collectible Validation

| Check | Failure Behavior |
|-------|------------------|
| Room has no collectibles | Hack alert + exit |
| Collectible ID doesn't exist | Hack alert + exit |
| Collectible not available | Hack alert + exit |
| No free item slot | Hack alert + exit |

---

### 11. One-Time Items

| Check | Failure Behavior |
|-------|------------------|
| Room has no one-times | Hack alert + exit |
| Already taken | Hack alert + exit |
| No free slot | Hack alert + exit |

---

### 12. Discard Item

| Check | Failure Behavior |
|-------|------------------|
| Slot < 1 or > 9 | Hack alert + exit |
| Slot is empty | Hack alert + exit |
| Item not discardable | Hack alert + exit |

**Discardable items whitelist:** IDs 1-61 (with some exceptions)

---

### 13. Emote

| Check | Failure Behavior |
|-------|------------------|
| Slot < 1 or > 5 | Hack alert + exit |
| No emote in slot | Hack alert + exit |

---

### 14. BBS Posting

| Check | Failure Behavior |
|-------|------------------|
| Room has no BBS | Hack alert + exit |
| Cooldown not passed | Response code 0 |

---

## Constants and Limits

| Constant | Value | Notes |
|----------|-------|-------|
| MAX_POINTS | Configurable | Maximum on-hand currency |
| Inventory slots | 9 | Per category |
| Storage slots | 180 | 20 pages × 9 slots |
| Mailbox slots | 50 | Max mails |
| Emote slots | 5 | |
| Clan name min | 3 | Characters |
| Clan name max | 15 | Characters |
| Username max | 10 | Characters |
| Password max | 10 | Characters |
| Clan members | 10 | Plus leader |
| Plant fairies max | 5 | |
| Clan create cost | 10,000 SP | |
| Invite cooldown | 15 seconds | |
| Sell price ratio | 1/3 | Of buy price |

---

## Hack Alert Patterns

The original server logs hack alerts for:

1. **Impossible states** - Operating on empty slots, non-existent items
2. **Resource manipulation** - Trying to spend more than owned
3. **Permission violations** - Non-leader clan admin, taking others' plants
4. **Protocol violations** - Invalid slot numbers, mismatched quest data
5. **Room context violations** - Accessing features the room doesn't have

### Hack Alert Categories

```
Inventory manipulation:
- Empty slot operations
- Invalid slot indices
- Non-discardable items

Economy exploits:
- Insufficient funds
- Shop without shop section
- Free slot missing

Permission issues:
- Clan admin without leadership
- Taking others' plant fruits

Quest manipulation:
- ID mismatches
- Step mismatches
- Already cleared quests

Room context:
- Missing collectibles section
- Missing one-times section
- Missing plant spots
- Missing BBS
```

---

## Implementation Recommendations

### Error Response Strategy

1. **Authentication errors** - Always send explicit error codes
2. **Inventory errors** - Log + silent exit (don't help attackers)
3. **Economy errors** - Send error codes for legitimate failures (out of stock, insufficient funds)
4. **Permission errors** - Log + silent exit
5. **Protocol errors** - Log + silent exit

### Rate Limiting Points

The original server has minimal rate limiting. Recommended additions:

- Chat messages
- Movement updates
- Shop purchases
- Mail sending
- Clan invites (already has 15s cooldown)
- BBS posts (already has cooldown)

### Critical Validations

Always validate:
1. Player is logged in and authenticated
2. Slot indices are within bounds
3. Items exist before operating on them
4. Sufficient currency before deducting
5. Room context supports the operation
6. Player has permission (ownership, leadership)
