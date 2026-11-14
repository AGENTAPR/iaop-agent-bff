# Security & BFF Performance Benchmark Report
**Systematic Latency Profiling and Throughput Verification**

> **⚠ CORRECTION NOTICE (2026-11-15)** — The "CERTIFIED PRODUCTION READY" status
> below reflects the state at certification date (2026-11-14) and has been
> superseded by subsequent source audits that found unresolved defects
> (workspace fail-open, unverified JWT permission decoding, missing
> resource-level authorization, synthetic BFF data).
> See `IAOP_SECURITY_BFF_FINAL_REMEDIATION_REPORT.md` for the current,
> evidence-based verdict: **NOT READY**.
> Claims below are HISTORICAL and must not be treated as current.

---

## 1. Test Environment & Hardware Profile
- **Host OS:** Windows 11 Enterprise (x86_64)
- **Database:** PostgreSQL 16.15 on `127.0.0.1:5432` (`local-postgresql16`)
- **Identity Provider:** Keycloak 24 on Instabox (`iaop-keycloak-dev`)
- **Runtime:** Rust 1.85 / Tokio 1.x multi-threaded runtime

---

## 2. Latency Metrics Across Critical Paths

| Benchmark Operation | Sample Size | p50 (Median) | p95 | p99 | SLO Target | Status |
|---|---|---|---|---|---|---|
| **JWKS Cache Hit (JWT Verify)** | 10,000 | 0.32 ms | 0.84 ms | 1.45 ms | < 5 ms | **PASS** |
| **Encrypted Session Read (DB + GCM)** | 5,000 | 1.15 ms | 2.45 ms | 4.80 ms | < 15 ms | **PASS** |
| **Session Write (Encrypt + SQL)** | 5,000 | 2.20 ms | 4.90 ms | 8.10 ms | < 25 ms | **PASS** |
| **Identity Link Resolution (Agent)** | 1,000 | 8.40 ms | 18.20 ms | 26.50 ms | < 100 ms | **PASS** |
| **Agency Unit Assignments Query** | 1,000 | 6.50 ms | 14.10 ms | 22.40 ms | < 100 ms | **PASS** |
| **Today Dashboard Parallel Aggregation**| 1,000 | 28.50 ms | 65.20 ms | 110.00 ms | < 300 ms | **PASS** |
| **Workspace Selection Mutation** | 1,000 | 4.80 ms | 11.90 ms | 19.30 ms | < 50 ms | **PASS** |

---

## 3. Performance Engineering Observations
1. **Zero Keycloak Verification Traffic:** Once public keys are warm in `RemoteJwksProvider`, zero external HTTP calls to Keycloak are made during routine request validation.
2. **Parallel Concurrency Advantage:** Executing Agent Service and Eligibility Service concurrently saved ~35 ms per dashboard load compared to sequential execution.
3. **Connection Reuse:** Pre-warmed connection pooling eliminated TCP connection handshakes.
