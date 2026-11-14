//! Server-side session store. In-memory implementation for dev;
//! PostgreSQL adapter is the production target (see docs).
//! Tokens live ONLY here — never in cookies or browser storage.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub subject: String,
    pub display_name: Option<String>,
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// When the access token expires (UTC).
    pub access_expires_at: DateTime<Utc>,
    pub created_at: Instant,
    pub last_seen: Instant,
    /// Idle timeout: session dies if unused beyond this.
    pub idle_timeout: Duration,
    /// Absolute lifetime cap.
    pub absolute_timeout: Duration,
    /// CSRF token bound to this session (double-submit).
    pub csrf_token: String,
    /// Optional workspace context.
    pub workspace_id: Option<String>,
    /// Optional linked agent ID.
    pub agent_id: Option<Uuid>,
}

impl Session {
    pub fn is_expired(&self) -> bool {
        self.last_seen.elapsed() > self.idle_timeout
            || self.created_at.elapsed() > self.absolute_timeout
    }
}

/// Pending login transaction (pre-authentication).
#[derive(Debug, Clone)]
pub struct LoginTransaction {
    pub state: String,
    pub nonce: String,
    pub pkce_verifier: String,
    pub return_to: String,
    /// Redirect URI used for this transaction (defaults to OidcConfig.redirect_uri).
    /// Stored so token exchange uses the exact URI sent to the authorize endpoint.
    pub redirect_uri: Option<String>,
    /// Browser binding: hash of a cookie set at login initiation.
    pub browser_binding: String,
    pub created_at: Instant,
    pub ttl: Duration,
}

impl LoginTransaction {
    pub fn is_valid(&self) -> bool {
        self.created_at.elapsed() < self.ttl
    }
}

/// Async session store trait — production target is PostgreSQL adapter [`PgSessionStore`].
/// Tokens live ONLY here — never in cookies or browser storage.
#[async_trait::async_trait]
pub trait SessionStore: Send + Sync {
    async fn create_session(&self, s: Session) -> Result<(), String>;
    async fn get_session(&self, id: &str) -> Option<Session>;
    async fn update_session(&self, s: &Session) -> Result<(), String>;
    /// Atomically persist only the workspace context for a session.
    /// Fails if the session does not exist — never silently succeeds.
    async fn persist_workspace(
        &self,
        session_id: &str,
        workspace_id: Option<&str>,
    ) -> Result<(), String>;
    /// Atomically replace token material after a refresh. Fails if the
    /// session does not exist or is expired — never silently succeeds.
    async fn update_tokens(
        &self,
        session_id: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        access_expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), String>;
    async fn delete_session(&self, id: &str);
    async fn put_login_txn(&self, tx: LoginTransaction) -> Result<(), String>;
    /// Atomically consumes a login transaction (single-use).
    async fn take_login_txn(&self, state: &str) -> Option<LoginTransaction>;
}

/// Compatibility alias: `AsyncSessionStore` is now identical to [`SessionStore`].
pub use SessionStore as AsyncSessionStore;

/// In-memory store (dev/single-instance). NOT for multi-instance prod.
#[derive(Default)]
pub struct InMemorySessionStore {
    sessions: Mutex<HashMap<String, Session>>,
    txns: Mutex<HashMap<String, LoginTransaction>>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl SessionStore for InMemorySessionStore {
    async fn create_session(&self, s: Session) -> Result<(), String> {
        self.sessions
            .lock()
            .map_err(|_| "store poisoned".to_string())?
            .insert(s.id.clone(), s);
        Ok(())
    }

    async fn get_session(&self, id: &str) -> Option<Session> {
        self.sessions
            .lock()
            .ok()?
            .get(id)
            .filter(|s| !s.is_expired())
            .cloned()
    }

    async fn update_session(&self, s: &Session) -> Result<(), String> {
        let mut map = self
            .sessions
            .lock()
            .map_err(|_| "store poisoned".to_string())?;
        map.insert(s.id.clone(), s.clone());
        Ok(())
    }

    async fn persist_workspace(
        &self,
        session_id: &str,
        workspace_id: Option<&str>,
    ) -> Result<(), String> {
        let mut map = self
            .sessions
            .lock()
            .map_err(|_| "store poisoned".to_string())?;
        let s = map
            .get_mut(session_id)
            .ok_or_else(|| "session not found".to_string())?;
        if s.is_expired() {
            return Err("session expired".to_string());
        }
        s.workspace_id = workspace_id.map(|w| w.to_string());
        Ok(())
    }

    async fn update_tokens(
        &self,
        session_id: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        access_expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), String> {
        let mut map = self
            .sessions
            .lock()
            .map_err(|_| "store poisoned".to_string())?;
        let s = map
            .get_mut(session_id)
            .ok_or_else(|| "session not found".to_string())?;
        if s.is_expired() {
            return Err("session expired".to_string());
        }
        s.access_token = access_token.to_string();
        if let Some(rt) = refresh_token {
            s.refresh_token = Some(rt.to_string());
        }
        s.access_expires_at = access_expires_at;
        Ok(())
    }

    async fn delete_session(&self, id: &str) {
        if let Ok(mut map) = self.sessions.lock() {
            map.remove(id);
        }
    }

    async fn put_login_txn(&self, tx: LoginTransaction) -> Result<(), String> {
        self.txns
            .lock()
            .map_err(|_| "store poisoned".to_string())?
            .insert(tx.state.clone(), tx);
        Ok(())
    }

    async fn take_login_txn(&self, state: &str) -> Option<LoginTransaction> {
        self.txns
            .lock()
            .ok()?
            .remove(state)
            .filter(|t| t.is_valid())
    }
}

/// Generates a new opaque session ID.
pub fn new_session_id() -> String {
    Uuid::now_v7().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_expiry_logic() {
        let s = Session {
            id: "x".into(),
            subject: "u".into(),
            display_name: None,
            access_token: "t".into(),
            refresh_token: None,
            access_expires_at: Utc::now(),
            created_at: Instant::now(),
            last_seen: Instant::now(),
            idle_timeout: Duration::from_secs(3600),
            absolute_timeout: Duration::from_secs(86400),
            csrf_token: "c".into(),
            workspace_id: None,
            agent_id: None,
        };
        assert!(!s.is_expired());
    }

    #[tokio::test]
    async fn txn_single_use() {
        let store = InMemorySessionStore::new();
        let tx = LoginTransaction {
            state: "s1".into(),
            nonce: "n".into(),
            pkce_verifier: "v".into(),
            return_to: "/".into(),
            redirect_uri: None,
            browser_binding: "b".into(),
            created_at: Instant::now(),
            ttl: Duration::from_secs(300),
        };
        store.put_login_txn(tx).await.unwrap();
        assert!(store.take_login_txn("s1").await.is_some());
        assert!(store.take_login_txn("s1").await.is_none()); // replay rejected
    }
}
