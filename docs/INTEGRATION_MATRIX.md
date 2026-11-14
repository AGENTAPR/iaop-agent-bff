# AGENA IAOP Agent BFF — Integration Matrix

This matrix documents downstream service dependencies, timeout limits, idempotency policies, retry behavior, and contract verification status.

| Downstream Service | Base URL Config | Client Adapter | Timeout | Retry Policy | Idempotency Header | Domain Status in Workspace | Contract Verification |
|---|---|---|---|---|---|---|---|
| **iaop-agent-service** | `AGENT_SERVICE_URL` | `AgentClient` (`crates/infrastructure/src/clients/agent_client.rs`) | Connect: 3s, Request: 10s | Idempotent reads only; no unsafe POST retry | Not needed on reads | **FULLY IMPLEMENTED** (37 files: Aggregate, commands, queries, PostgreSQL) | ✅ Verified against actual service API |
| **iaop-agency-service** | `AGENCY_SERVICE_URL` | Generic Downstream Adapter | Connect: 3s, Request: 10s | 2x exponential backoff on 503/429 | `idempotency-key` on transfers | SCAFFOLD_ONLY | DRAFT Contract (Scaffold) |
| **iaop-recruitment-service**| `RECRUITMENT_SERVICE_URL`| Generic Downstream Adapter | Connect: 3s, Request: 10s | None on candidate creation | `idempotency-key` on referrals | SCAFFOLD_ONLY | DRAFT Contract (Scaffold) |
| **iaop-onboarding-service** | `ONBOARDING_SERVICE_URL` | Generic Downstream Adapter | Connect: 3s, Request: 10s | None on submission | `idempotency-key` on task submit | SCAFFOLD_ONLY | DRAFT Contract (Scaffold) |
| **iaop-learning-service** | `LEARNING_SERVICE_URL` | Generic Downstream Adapter | Connect: 3s, Request: 10s | Reads retryable; 1x on enroll | `idempotency-key` on enrollment | SCAFFOLD_ONLY | DRAFT Contract (Scaffold) |
| **iaop-certification-service**| `CERTIFICATION_SERVICE_URL`| Generic Downstream Adapter | Connect: 3s, Request: 10s | Reads retryable | `idempotency-key` on evidence upload | SCAFFOLD_ONLY | DRAFT Contract (Scaffold) |
| **iaop-sales-service** | `SALES_SERVICE_URL` | Generic Downstream Adapter | Connect: 3s, Request: 10s | Idempotent GET retry; No POST retry without key | `idempotency-key` on proposal submit | SCAFFOLD_ONLY | DRAFT Contract (Scaffold) |
| **iaop-portfolio-service** | `PORTFOLIO_SERVICE_URL` | Generic Downstream Adapter | Connect: 3s, Request: 10s | 2x on idempotent policy reads | None (read only in BFF) | SCAFFOLD_ONLY | DRAFT Contract (Scaffold) |
| **iaop-commission-service**| `COMMISSION_SERVICE_URL`| Generic Downstream Adapter | Connect: 3s, Request: 10s | 2x on ledger reads | None (BFF never writes ledger) | SCAFFOLD_ONLY | DRAFT Contract (Scaffold) |
| **iaop-performance-service**| `PERFORMANCE_SERVICE_URL`| Generic Downstream Adapter | Connect: 3s, Request: 10s | 2x on metric reads | None | SCAFFOLD_ONLY | DRAFT Contract (Scaffold) |
| **iaop-workflow-service** | `WORKFLOW_SERVICE_URL` | Generic Downstream Adapter | Connect: 3s, Request: 10s | None on task complete | `idempotency-key` on task mutations | SCAFFOLD_ONLY | DRAFT Contract (Scaffold) |
| **iaop-notification-service**| `NOTIFICATION_SERVICE_URL`| Generic Downstream Adapter | Connect: 3s, Request: 10s | 2x on list; 1x on mark-read | None | SCAFFOLD_ONLY | DRAFT Contract (Scaffold) |
| **iaop-document-service** | `DOCUMENT_SERVICE_URL` | Generic Downstream Adapter | Connect: 3s, Request: 10s | 1x on intent creation | Pre-signed URL generation | SCAFFOLD_ONLY | DRAFT Contract (Scaffold) |
| **Core Claims Authority** | `CLAIMS_SERVICE_URL` | Generic Downstream Adapter | Connect: 3s, Request: 10s | No automatic retry on submit | `idempotency-key` on case creation | UNRESOLVED OWNER | DRAFT Contract |
| **Core Customer Master** | `CUSTOMER_SERVICE_URL` | Generic Downstream Adapter | Connect: 3s, Request: 10s | No automatic retry on create | `idempotency-key` on customer intake | UNRESOLVED OWNER | DRAFT Contract |
