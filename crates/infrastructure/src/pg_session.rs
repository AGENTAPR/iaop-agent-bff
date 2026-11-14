//! PostgreSQL-backed session store for production use.
//!
//! Implements `AsyncSessionStore` using SQLx + PgPool without compile-time query macros
//! so that compilation does not require a live database or offline query data.

use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};

use crate::session::{AsyncSessionStore, LoginTransaction, Session};

/// PostgreSQL-backed session store.
pub struct PgSessionStore {
    pool: PgPool,
}

impl PgSessionStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl AsyncSessionStore for PgSessionStore {
    async fn create_session(&self, s: Session) -> Result<(), String> {
        let sql = r#"
            INSERT INTO bff_sessions (
                id, subject, display_name,
                access_token, refresh_token, access_expires_at,
                csrf_token, idle_timeout_secs, absolute_timeout_secs,
                workspace_id, agent_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#;

        sqlx::query(sql)
            .bind(&s.id)
            .bind(&s.subject)
            .bind(&s.display_name)
            .bind(&s.access_token)
            .bind(&s.refresh_token)
            .bind(s.access_expires_at)
            .bind(&s.csrf_token)
            .bind(s.idle_timeout.as_secs() as i64)
            .bind(s.absolute_timeout.as_secs() as i64)
            .bind(&s.workspace_id)
            .bind(s.agent_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "pg_session: create_session failed");
                "session store error".to_string()
            })?;

        Ok(())
    }

    async fn get_session(&self, id: &str) -> Option<Session> {
        let sql = r#"
            SELECT
                id, subject, display_name,
                access_token, refresh_token, access_expires_at,
                csrf_token, idle_timeout_secs, absolute_timeout_secs,
                created_at, last_seen_at, workspace_id, agent_id
            FROM bff_sessions
            WHERE id = $1
              AND last_seen_at + (idle_timeout_secs || ' seconds')::interval > now()
              AND created_at   + (absolute_timeout_secs || ' seconds')::interval > now()
        "#;

        let row = sqlx::query(sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .ok()??;

        let now = Instant::now();
        let idle_timeout_secs: i64 = row.try_get("idle_timeout_secs").ok()?;
        let absolute_timeout_secs: i64 = row.try_get("absolute_timeout_secs").ok()?;
        let idle_timeout = Duration::from_secs(idle_timeout_secs.max(0) as u64);
        let absolute_timeout = Duration::from_secs(absolute_timeout_secs.max(0) as u64);

        let created_at: DateTime<Utc> = row.try_get("created_at").ok()?;
        let last_seen_at: DateTime<Utc> = row.try_get("last_seen_at").ok()?;

        let age = Utc::now()
            .signed_duration_since(created_at)
            .to_std()
            .unwrap_or(Duration::ZERO);
        let idle = Utc::now()
            .signed_duration_since(last_seen_at)
            .to_std()
            .unwrap_or(Duration::ZERO);

        Some(Session {
            id: row.try_get("id").ok()?,
            subject: row.try_get("subject").ok()?,
            display_name: row.try_get("display_name").ok()?,
            access_token: row.try_get("access_token").ok()?,
            refresh_token: row.try_get("refresh_token").ok()?,
            access_expires_at: row.try_get("access_expires_at").ok()?,
            created_at: now.checked_sub(age).unwrap_or(now),
            last_seen: now.checked_sub(idle).unwrap_or(now),
            idle_timeout,
            absolute_timeout,
            csrf_token: row.try_get("csrf_token").ok()?,
            workspace_id: row.try_get("workspace_id").ok()?,
            agent_id: row.try_get("agent_id").ok()?,
        })
    }

    async fn update_session(&self, s: &Session) -> Result<(), String> {
        let sql = r#"
            UPDATE bff_sessions
            SET last_seen_at = now(),
                workspace_id = $2,
                agent_id     = $3
            WHERE id = $1
        "#;

        sqlx::query(sql)
            .bind(&s.id)
            .bind(&s.workspace_id)
            .bind(s.agent_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "pg_session: update_session failed");
                "session store error".to_string()
            })?;

        Ok(())
    }

    async fn delete_session(&self, id: &str) {
        let sql = "DELETE FROM bff_sessions WHERE id = $1";
        if let Err(e) = sqlx::query(sql).bind(id).execute(&self.pool).await {
            tracing::warn!(error = %e, "pg_session: delete_session failed");
        }
    }

    async fn put_login_txn(&self, tx: LoginTransaction) -> Result<(), String> {
        let expires_at = Utc::now()
            + chrono::Duration::from_std(tx.ttl).unwrap_or(chrono::Duration::seconds(300));

        let sql = r#"
            INSERT INTO bff_login_transactions
                (state, nonce, pkce_verifier, return_to, browser_binding, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (state) DO NOTHING
        "#;

        sqlx::query(sql)
            .bind(&tx.state)
            .bind(&tx.nonce)
            .bind(&tx.pkce_verifier)
            .bind(&tx.return_to)
            .bind(&tx.browser_binding)
            .bind(expires_at)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "pg_session: put_login_txn failed");
                "session store error".to_string()
            })?;

        Ok(())
    }

    async fn take_login_txn(&self, state: &str) -> Option<LoginTransaction> {
        let sql = r#"
            DELETE FROM bff_login_transactions
            WHERE state = $1 AND expires_at > now()
            RETURNING state, nonce, pkce_verifier, return_to, browser_binding, expires_at
        "#;

        let row = sqlx::query(sql)
            .bind(state)
            .fetch_optional(&self.pool)
            .await
            .ok()??;

        Some(LoginTransaction {
            state: row.try_get("state").ok()?,
            nonce: row.try_get("nonce").ok()?,
            pkce_verifier: row.try_get("pkce_verifier").ok()?,
            return_to: row.try_get("return_to").ok()?,
            browser_binding: row.try_get("browser_binding").ok()?,
            created_at: Instant::now(),
            ttl: Duration::from_secs(1),
        })
    }
}
