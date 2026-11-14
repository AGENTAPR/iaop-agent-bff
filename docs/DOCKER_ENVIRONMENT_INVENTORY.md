# DOCKER ENVIRONMENT INVENTORY — IAOP PRODUCTION READINESS
**Date:** 2026-11-14
**Environment:** Local Developer Workstation (Windows 11 / Docker Desktop)
**Host Resources:** 32 GB RAM, 6-Core CPU, >4.5 GB Free Memory

---

## 1. Discovered Pre-existing Docker Infrastructure

Per Section 3 of the Master Execution Prompt, existing Docker infrastructure was discovered and inspected prior to provisioning.

| Resource Name | Type | Image | Status | Ports / Binding | Reuse Disposition |
|---|---|---|---|---|---|
| `local-postgresql16` | Container | `postgres:16` (16.15) | Running (Healthy, Up 20+ hours) | `127.0.0.1:5432->5432/tcp` | **EXISTING_REUSABLE** — Reused as shared PostgreSQL host for all services |
| `ifg-cts-local-out-gateway-cts-service-1` | Container | `ifg/out-gateway-cts-service:local-mock` | Running | `127.0.0.1:8084->8080/tcp` | **EXISTING_PRESERVED** — Unaltered developer dependency |
| `iaop-kafka` | Container | `apache/kafka:3.7.0` (KRaft mode) | Running (Healthy) | `127.0.0.1:9092->9092/tcp` | **EXISTING_REUSABLE** — Single-node broker for real integration tests |

---

## 2. PostgreSQL Databases and Schemas in `local-postgresql16`

Discovered databases inside the reused PostgreSQL 16 container:

1. `agency_db` — Dedicated Agency Service database. Applied migrations 1–8.
2. `iaop_agent` — Dedicated Agent Service database. Applied migrations 1–4.
3. `iaop_agent_bff` — Dedicated Agent BFF database. Applied migrations 1–3 (`bff_sessions`, `bff_login_transactions`, `bff_user_preferences`).
4. `postgres` — PostgreSQL administrative catalog.
5. Other developer databases (`ifgpay_dev`, `iaop_integration_dev`) preserved untouched.

---

## 3. Kafka Topics and Broker Configuration

- **Bootstrap Server:** `127.0.0.1:9092`
- **Cluster Mode:** KRaft (Kafka 3.7.0, single-node, no ZooKeeper)
- **Cluster ID:** `4L622nShTUiTwyvCdLfZ2w`
- **Dedicated Topics Provisioned:**
  - `iaop.agent.events.v1` (3 partitions, replication factor 1)
  - `iaop.agency.events.v1` (3 partitions, replication factor 1)
- **Consumer Validation:** Direct console consumer verified receiving and processing events published by Rust `rskafka` transport.

---

## 4. Resource Boundary Adherence

- Zero additional PostgreSQL containers were created.
- Existing volume data in `local-postgresql16` was strictly protected.
- No unrelated projects or containers were interrupted or modified.
