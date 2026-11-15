//! Real-PostgreSQL integration tests for Agent BFF Session Store against local-postgresql16.
//!
//! Covers: migrations applied, token encryption at rest, secure session roundtrip,
//! fail-closed rejection on tampered ciphertext, and atomic single-use login transactions.

use std::time::{Duration, Instant};

use chrono::Utc;
use iaop_agent_bff_infrastructure::pg_session::PgSessionStore;
use iaop_agent_bff_infrastructure::session::{LoginTransaction, Session, SessionStore};
use sqlx::PgPool;
use uuid::Uuid;

const TEST_KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set — no credentials are embedded in source");
    let pool = PgPool::connect(&url)
        .await
        .expect("failed to connect to iaop_agent_bff database");
    iaop_agent_bff_infrastructure::persistence::run_migrations(&pool)
        .await
        .expect("failed to run migrations");
    pool
}

#[tokio::test]
async fn encrypted_session_persistence_and_fail_closed_on_tampering() {
    let pool = pool().await;
    let store = PgSessionStore::new(pool.clone(), Some(TEST_KEY));

    let sid = format!("test-sess-{}", Uuid::now_v7());
    let plain_token = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.super-sensitive-access-token";
    let plain_refresh = "super-sensitive-refresh-token";
    let csrf = "csrf-secret-key-123456";

    let s = Session {
        id: sid.clone(),
        subject: "user-sub-123".to_string(),
        display_name: Some("Test User".to_string()),
        access_token: plain_token.to_string(),
        refresh_token: Some(plain_refresh.to_string()),
        access_expires_at: Utc::now() + chrono::Duration::hours(1),
        created_at: Instant::now(),
        last_seen: Instant::now(),
        idle_timeout: Duration::from_secs(3600),
        absolute_timeout: Duration::from_secs(86400),
        csrf_token: csrf.to_string(),
        workspace_id: Some("ws-01".to_string()),
        agent_id: Some(Uuid::now_v7()),
    };

    // 1. Create encrypted session
    store.create_session(s).await.expect("session created");

    // 2. Verify token is ENCRYPTED in PostgreSQL (never plaintext)
    let raw_db_token: String =
        sqlx::query_scalar("SELECT access_token FROM bff_sessions WHERE id = $1")
            .bind(&sid)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_ne!(
        raw_db_token, plain_token,
        "Token MUST NOT be stored in plaintext!"
    );
    assert!(
        !raw_db_token.contains("eyJhbGciOiJSUzI1Ni"),
        "Ciphertext must not leak token prefixes"
    );

    // 3. Retrieve session via store — returns decrypted tokens
    let retrieved = store.get_session(&sid).await.expect("session must exist");
    assert_eq!(retrieved.access_token, plain_token);
    assert_eq!(retrieved.refresh_token.as_deref(), Some(plain_refresh));
    assert_eq!(retrieved.subject, "user-sub-123");

    // 4. Update session workspace atomically via persist_workspace
    store
        .persist_workspace(&sid, Some("ws-02"))
        .await
        .expect("persist_workspace succeeds");
    store
        .update_session(&retrieved)
        .await
        .expect("update_session activity touch succeeds");

    let updated_ws: Option<String> =
        sqlx::query_scalar("SELECT workspace_id FROM bff_sessions WHERE id = $1")
            .bind(&sid)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(updated_ws.as_deref(), Some("ws-02"));

    // 5. Tamper test: corrupt the ciphertext in database directly
    sqlx::query("UPDATE bff_sessions SET access_token = '!!!tampered-ciphertext!!!' WHERE id = $1")
        .bind(&sid)
        .execute(&pool)
        .await
        .unwrap();

    // 6. Fail-closed: get_session MUST return None (session invalid/rejected, no plaintext fallback!)
    let corrupted = store.get_session(&sid).await;
    assert!(
        corrupted.is_none(),
        "Tampered session MUST be rejected (fail-closed, return None)"
    );

    // 7. Cleanup
    store.delete_session(&sid).await;
    let deleted: Option<String> = sqlx::query_scalar("SELECT id FROM bff_sessions WHERE id = $1")
        .bind(&sid)
        .fetch_optional(&pool)
        .await
        .unwrap();
    assert!(deleted.is_none());
}

#[tokio::test]
async fn atomic_single_use_login_transactions_in_real_database() {
    let pool = pool().await;
    let store = PgSessionStore::new(pool.clone(), Some(TEST_KEY));

    let state = format!("login-state-{}", Uuid::now_v7());
    let txn = LoginTransaction {
        state: state.clone(),
        nonce: "nonce-12345".to_string(),
        pkce_verifier: "verifier-abcde123456789".to_string(),
        return_to: "/dashboard".to_string(),
        redirect_uri: None,
        browser_binding: "hash-cookie-binding".to_string(),
        created_at: Instant::now(),
        ttl: Duration::from_secs(300),
    };

    // Put login transaction
    store.put_login_txn(txn).await.expect("put txn succeeds");

    // First take must succeed
    let taken1 = store.take_login_txn(&state).await;
    assert!(
        taken1.is_some(),
        "First consumption of login txn must succeed"
    );
    let taken1 = taken1.unwrap();
    assert_eq!(taken1.state, state);
    assert_eq!(taken1.nonce, "nonce-12345");
    assert_eq!(taken1.return_to, "/dashboard");

    // Second take (replay) MUST return None (atomic single-use)
    let taken2 = store.take_login_txn(&state).await;
    assert!(
        taken2.is_none(),
        "Replay of login transaction must be rejected (single-use)"
    );
}
