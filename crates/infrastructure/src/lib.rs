#![forbid(unsafe_code)]
//! Infrastructure layer for Agent Experience BFF.
//!
//! Adapters implementing application ports: configuration, PostgreSQL
//! persistence (pool, migrations, outbox/inbox), messaging helpers,
//! and security utilities. Depends on `application` and `domain`.

pub mod clients;
pub mod config;
pub mod messaging;
pub mod oidc;
pub mod persistence;
pub mod pg_session;
pub mod security;
pub mod session;
