# Accounts Table Reference

**Document Status:** Complete  
**Last Updated:** 2024-01-08  
**Related:** [`01-schema-overview.md`](01-schema-overview.md)

## Overview

The `accounts` table stores authentication credentials and account-level metadata. See [`01-schema-overview.md`](01-schema-overview.md) for complete schema.

## Quick Reference

```sql
CREATE TABLE accounts (
    id SERIAL PRIMARY KEY,
    username VARCHAR(20) UNIQUE NOT NULL,
    password_hash VARCHAR(60) NOT NULL,  -- bcrypt
    email VARCHAR(255),
    mac_address VARCHAR(17),
    created_at TIMESTAMP DEFAULT NOW(),
    last_login TIMESTAMP,
    is_banned BOOLEAN DEFAULT FALSE,
    ban_reason TEXT
);
```

## Common Queries

### Create Account
```sql
INSERT INTO accounts (username, password_hash, mac_address)
VALUES ($1, $2, $3)
RETURNING id;
```

### Authenticate
```sql
SELECT id, password_hash, is_banned, ban_reason
FROM accounts
WHERE username = $1;
```

### Update Last Login
```sql
UPDATE accounts
SET last_login = NOW()
WHERE id = $1;
```

### Ban Account
```sql
UPDATE accounts
SET is_banned = TRUE, ban_reason = $1
WHERE id = $2;
```

## Rust Implementation

```rust
pub async fn create_account(
    pool: &PgPool,
    username: &str,
    password: &str,
    mac_address: &str,
) -> Result<i32, sqlx::Error> {
    let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).unwrap();
    
    let record = sqlx::query!(
        r#"
        INSERT INTO accounts (username, password_hash, mac_address)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
        username,
        password_hash,
        mac_address,
    )
    .fetch_one(pool)
    .await?;
    
    Ok(record.id)
}

pub async fn authenticate(
    pool: &PgPool,
    username: &str,
    password: &str,
) -> Result<Option<i32>, sqlx::Error> {
    let record = sqlx::query!(
        r#"
        SELECT id, password_hash, is_banned
        FROM accounts
        WHERE username = $1
        "#,
        username,
    )
    .fetch_optional(pool)
    .await?;
    
    if let Some(rec) = record {
        if rec.is_banned {
            return Ok(None); // Banned
        }
        
        if bcrypt::verify(password, &rec.password_hash).unwrap_or(false) {
            return Ok(Some(rec.id));
        }
    }
    
    Ok(None)
}
```

For full schema details, see [`01-schema-overview.md`](01-schema-overview.md).
