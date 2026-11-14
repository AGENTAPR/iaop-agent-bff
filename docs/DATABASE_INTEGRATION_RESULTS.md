# DATABASE INTEGRATION RESULTS — REAL POSTGRESQL 16
**PostgreSQL Container:** `local-postgresql16` (`postgres:16.15`, healthy on `127.0.0.1:5432`)
**Test Execution Mode:** Real SQLx queries, live connection pools, transactions, and migration execution.

---

## 1. Agency Service (`iaop-agency-service`)

- **Database:** `agency_db`
- **Migrations Applied:** 8 migrations (`20261113000001` through `20261114000008`)
- **Command:** `cargo test --manifest-path iaop-agency-service/Cargo.toml --test db_tests`
- **Result:** **9 passed; 0 failed; 0 ignored** in 0.60s

```
running 9 tests
test idempotency_replay_and_conflict ... ok
test agency_crud_and_unique_code ... ok
test outbox_and_audit_written_atomically ... ok
test outbox_lease_claiming_fenced_completion_and_poison_handling ... ok
test leadership_assign_end_and_duplicate_role_rejected ... ok
test assignment_overlap_rejected_and_transfer_preserves_history ... ok
test concurrent_reparent_one_wins_or_both_valid ... ok
test hierarchy_ancestors_descendants_and_cycle_rejection ... ok
test temporal_hierarchy_as_of ... ok
```

---

## 2. Agent Service (`iaop-agent-service`)

- **Database:** `iaop_agent`
- **Migrations Applied:** 4 migrations (`20261113000001` through `20261114000004`)
- **Command:** `cargo test --manifest-path iaop-agent-service/Cargo.toml --test db_tests`
- **Result:** **2 passed; 0 failed; 0 ignored** in 0.14s

```
running 2 tests
test migrations_and_agent_crud_in_real_database ... ok
test outbox_lease_claiming_fenced_completion_and_poison_handling ... ok
```

---

## 3. Agent BFF (`iaop-agent-bff`)

- **Database:** `iaop_agent_bff`
- **Migrations Applied:** 3 migrations (`20261113000001` through `20261114000003`)
- **Command:** `cargo test --manifest-path iaop-agent-bff/Cargo.toml --test db_tests`
- **Result:** **2 passed; 0 failed; 0 ignored** in 0.28s

```
running 2 tests
test atomic_single_use_login_transactions_in_real_database ... ok
test encrypted_session_persistence_and_fail_closed_on_tampering ... ok
```

---

## 4. Key Assertions Verified in Real PostgreSQL

1. **Token Ciphertext at Rest:** Verified with `SELECT access_token FROM bff_sessions` that tokens are never stored in plaintext and contain no plaintext JWT header prefixes.
2. **Fail-Closed on Tampered Ciphertext:** Directly corrupting the SQL column via `UPDATE bff_sessions SET access_token = '!!!tampered!!!'` causes `store.get_session()` to return `None`.
3. **Atomic Outbox Lease Fencing:** When Worker A claims an event, Worker B attempting to mark it published receives `rows_affected == 0` and is rejected.
4. **Poison Event Terminal Status:** After 5 failed attempts, event is set to `is_terminal = TRUE, next_attempt_at = NULL` and is permanently excluded from future `claim_batch` queries.
5. **Organizational Hierarchy & Cycle Rejection:** Proved tree reparenting prevents cycles and enforces temporal overlap constraints.
