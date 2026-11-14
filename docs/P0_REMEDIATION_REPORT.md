# P0 REMEDIATION REPORT — IAOP PLATFORM STABILIZATION
**Date:** 2026-11-14
**Repositories:** `iaop-agent-service`, `iaop-agency-service`, `iaop-agent-bff`

---

## 1. Remediation P0-1 & P0-2: Real Kafka Transport (`rskafka = "0.6"`)

### Implementation Details:
- Replaced the no-op logging transport in `crates/infrastructure/src/transport.rs` in both `iaop-agent-service` and `iaop-agency-service` with a real `rskafka` producer.
- Injected canonical envelope headers:
  - `event_id`: UUID string
  - `event_type`: canonical dot-notated event name
  - `event_version`: integer schema version
  - `occurred_at`: RFC3339 timestamp
  - `source`: sending service name
  - `correlation_id` & `causation_id`: trace propagation
- Target topic resolution: `{topic_prefix}.{event_type}`.
- Broker acknowledgement: awaited via `partition_client.produce(vec![record], Compression::No).await`.
- Delivery timeout: strictly bounded to 5 seconds.
- Clean error propagation: returns `Err(String)` if Kafka is unreachable, timing out, or invalid broker.

### Verifiable Proof:
```
test transport::tests::kafka_delivery_succeeds_against_live_broker ... ok
test transport::tests::kafka_delivery_fails_cleanly_when_offline ... ok
```
Real event consumed from Kafka container `iaop-kafka`:
```json
{
  "eventId": "01a1d9b3-b1ba-7299-8bcf-b6d8ed956109",
  "eventType": "agent.events.v1",
  "eventVersion": 1,
  "source": "iaop-agent-service",
  "aggregateId": "01a1d9b3-b1ba-7299-8bcf-b6d9e701e790",
  "aggregateVersion": 1,
  "occurredAt": "2026-11-14T10:56:34.746136200Z",
  "correlationId": "01a1d9b3-b1ba-7299-8bcf-b6da6ef3cedc",
  "payload": {"action": "AGENT_REAL_KAFKA_VERIFIED", "agent_code": "AG-007"}
}
```

---

## 2. Remediation P0-3: Outbox Leasing Fencing and Terminal Poison Handling

### Implementation Details:
- Applied migrations:
  - `iaop-agent-service/migrations/20261114000004_outbox_terminal_status.sql`
  - `iaop-agency-service/migrations/20261114000008_outbox_terminal_status.sql`
- `claim_batch`: Added `AND (is_terminal = FALSE)` to ensure poison events are never re-leased.
- `mark_published`: Added lease fencing `WHERE event_id = $1 AND locked_by = $2`, checking `rows_affected() > 0`. If a worker loses its lease before publishing, the completion is rejected.
- `mark_retry`: When attempts exceed `max_attempts` (5), marks `is_terminal = TRUE, next_attempt_at = NULL`. Eliminates the artificial 100-year timestamp hack.

---

## 3. Remediation P0-4: BFF Token Encryption Fail-Closed

### Implementation Details:
- Updated `iaop-agent-bff/crates/infrastructure/src/security.rs`:
  - `TokenCipher::decrypt`: Removed fallback returning raw string on base64 decoding error.
  - Reject payloads with invalid length (`< 1 + 12 + 16`) or unsupported version byte (`raw[0] != 1u8`).
  - Strict AES-GCM MAC verification: tampered ciphertext returns explicit `Err`.
- Updated `iaop-agent-bff/crates/infrastructure/src/pg_session.rs`:
  - If token decryption returns `Err`, `get_session` logs a security alert and immediately returns `None` (fails closed).
- Added comprehensive unit tests in `security.rs`:
  - `rejects_tampered_ciphertext`
  - `rejects_plaintext_fallback`
  - `rejects_invalid_base64`
  - `rejects_unsupported_version`

---

## 4. Remediation P0-5: Async Session Storage End-to-End

### Implementation Details:
- Converted `SessionStore` into an asynchronous trait using `#[async_trait::async_trait]`.
- Replaced the synchronous `SessionStore` implementation on `PgSessionStore` (which used `tokio::task::block_in_place(|| block_on(...))`) with direct async SQLx operations.
- Updated all API handlers (`auth_routes.rs`, `principal.rs`, `identity.rs`) and test harnesses to `.await` session operations.
- Production safety: `src/main.rs` panics at startup if `ENVIRONMENT=production` and `DATABASE_URL` or `BFF_SESSION_KEY` is missing.

---

## 5. Remediation P0-6: Authoritative Eligibility

### Implementation Details:
- Removed synthetic `ACTIVE -> is_eligible = true` mapping with 30-day window from `iaop-agent-bff/crates/api/src/endpoints/today.rs`.
- Created typed `EligibilityClient` in `iaop-agent-bff/crates/infrastructure/src/clients/eligibility_client.rs`.
- Fails closed when unconfigured: returns `SectionResult::unavailable("Authoritative eligibility service is not configured")` with `is_eligible: false` and `valid_until: None`.
- Proposal creation (`sales.rs`): Enforced authoritative eligibility check before proposal submission; returns `selling_not_eligible` if agent is suspended, revoked, or non-compliant.

---

## 6. Remediation P0-7: Honest Response Composition

### Implementation Details:
- Updated `iaop-agent-bff/crates/api/src/endpoints/today.rs`:
  - When downstream services (tasks, agenda, activity) are unconfigured, return `SectionResult::unavailable(...)` instead of fake `AVAILABLE` empty vectors.
  - Flags `partial: true` in response metadata to prevent masking outages from frontend consumers.
