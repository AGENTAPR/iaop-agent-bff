//! Client adapter for the authoritative `iaop-agent-service`.

use std::time::Duration;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AgentClientError {
    #[error("network transport error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("agent not found")]
    NotFound,
    #[error("upstream service error ({status}): {message}")]
    Upstream { status: u16, message: String },
    #[error("deserialization error: {0}")]
    Serialization(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentView {
    pub id: Uuid,
    pub person_id: Uuid,
    pub agent_code: String,
    pub lifecycle_status: String,
    pub version: u64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentListResponse {
    pub data: Vec<AgentView>,
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppointmentView {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub appointment_reference: Option<String>,
    pub company_reference: Option<String>,
    pub appointment_type: String,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppointmentListResponse {
    pub data: Vec<AppointmentView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityLinkView {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub identity_provider: String,
    pub issuer: String,
    pub subject: String,
    pub status: String,
    pub linked_at: DateTime<Utc>,
    pub unlinked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityLinkListResponse {
    pub data: Vec<IdentityLinkView>,
}

#[derive(Debug, Clone)]
pub struct AgentClient {
    base_url: String,
    http: reqwest::Client,
}

impl AgentClient {
    pub fn new(base_url: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { base_url, http }
    }

    pub async fn get_agent(
        &self,
        agent_id: Uuid,
        token: &str,
        request_id: Option<Uuid>,
    ) -> Result<AgentView, AgentClientError> {
        let url = format!(
            "{}/api/v1/agents/{}",
            self.base_url.trim_end_matches('/'),
            agent_id
        );
        let mut req = self.http.get(&url).bearer_auth(token);
        if let Some(rid) = request_id {
            req = req
                .header("x-request-id", rid.to_string())
                .header("x-correlation-id", rid.to_string());
        }

        let resp = req.send().await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AgentClientError::NotFound);
        }
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(AgentClientError::Upstream {
                status,
                message: text,
            });
        }

        resp.json::<AgentView>()
            .await
            .map_err(|e| AgentClientError::Serialization(e.to_string()))
    }

    pub async fn get_agent_by_code(
        &self,
        code: &str,
        token: &str,
        request_id: Option<Uuid>,
    ) -> Result<AgentView, AgentClientError> {
        let url = format!(
            "{}/api/v1/agents/code/{}",
            self.base_url.trim_end_matches('/'),
            code
        );
        let mut req = self.http.get(&url).bearer_auth(token);
        if let Some(rid) = request_id {
            req = req
                .header("x-request-id", rid.to_string())
                .header("x-correlation-id", rid.to_string());
        }

        let resp = req.send().await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AgentClientError::NotFound);
        }
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(AgentClientError::Upstream {
                status,
                message: text,
            });
        }

        resp.json::<AgentView>()
            .await
            .map_err(|e| AgentClientError::Serialization(e.to_string()))
    }

    pub async fn get_appointments(
        &self,
        agent_id: Uuid,
        token: &str,
        request_id: Option<Uuid>,
    ) -> Result<Vec<AppointmentView>, AgentClientError> {
        let url = format!(
            "{}/api/v1/agents/{}/appointments",
            self.base_url.trim_end_matches('/'),
            agent_id
        );
        let mut req = self.http.get(&url).bearer_auth(token);
        if let Some(rid) = request_id {
            req = req.header("x-request-id", rid.to_string());
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let body: AppointmentListResponse = resp
            .json()
            .await
            .map_err(|e| AgentClientError::Serialization(e.to_string()))?;
        Ok(body.data)
    }

    pub async fn get_identity_links(
        &self,
        agent_id: Uuid,
        token: &str,
        request_id: Option<Uuid>,
    ) -> Result<Vec<IdentityLinkView>, AgentClientError> {
        let url = format!(
            "{}/api/v1/agents/{}/identity-links",
            self.base_url.trim_end_matches('/'),
            agent_id
        );
        let mut req = self.http.get(&url).bearer_auth(token);
        if let Some(rid) = request_id {
            req = req.header("x-request-id", rid.to_string());
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let body: IdentityLinkListResponse = resp
            .json()
            .await
            .map_err(|e| AgentClientError::Serialization(e.to_string()))?;
        Ok(body.data)
    }

    pub async fn resolve_identity(
        &self,
        issuer: &str,
        subject: &str,
        token: &str,
        request_id: Option<Uuid>,
    ) -> Result<Option<IdentityLinkView>, AgentClientError> {
        let url = format!(
            "{}/api/v1/identity-links/resolve",
            self.base_url.trim_end_matches('/')
        );
        let mut req = self
            .http
            .get(&url)
            .query(&[("issuer", issuer), ("subject", subject)])
            .bearer_auth(token);
        if let Some(rid) = request_id {
            req = req
                .header("x-request-id", rid.to_string())
                .header("x-correlation-id", rid.to_string());
        }

        let resp = req.send().await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(AgentClientError::Upstream {
                status,
                message: text,
            });
        }

        let link: IdentityLinkView = resp
            .json()
            .await
            .map_err(|e| AgentClientError::Serialization(e.to_string()))?;
        Ok(Some(link))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AGENT_ID: &str = "11111111-2222-3333-4444-555555555555";
    const LINK_ID: &str = "99999999-8888-7777-6666-555555555555";

    #[test]
    fn identity_link_view_deserializes_upstream_snake_case() {
        // Shape emitted by iaop-agent-service IdentityLinkDto (snake_case).
        let json = format!(
            r#"{{"id":"{LINK_ID}","agent_id":"{AGENT_ID}","identity_provider":"keycloak","issuer":"https://idp.example/realms/iaop-dev","subject":"4a6ba5c0-3238-4bec-927e-8fb471415099","status":"ACTIVE","linked_at":"2026-11-01T00:00:00Z","unlinked_at":null}}"#
        );
        let view: IdentityLinkView = serde_json::from_str(&json).unwrap();
        assert_eq!(view.agent_id, Uuid::parse_str(AGENT_ID).unwrap());
        assert_eq!(view.status, "ACTIVE");
        assert!(view.unlinked_at.is_none());
    }

    #[test]
    fn identity_link_view_rejects_camel_case_upstream_drift() {
        let json = format!(
            r#"{{"id":"{LINK_ID}","agentId":"{AGENT_ID}","identityProvider":"keycloak","issuer":"i","subject":"s","status":"ACTIVE","linkedAt":"2026-11-01T00:00:00Z","unlinkedAt":null}}"#
        );
        assert!(serde_json::from_str::<IdentityLinkView>(&json).is_err());
    }

    #[test]
    fn agent_view_deserializes_upstream_shape() {
        let json = format!(
            r#"{{"id":"{AGENT_ID}","person_id":"22222222-3333-4444-5555-666666666666","agent_code":"AGT-QA-0001","lifecycle_status":"ACTIVE","version":3,"created_at":"2026-10-01T00:00:00Z"}}"#
        );
        let view: AgentView = serde_json::from_str(&json).unwrap();
        assert_eq!(view.agent_code, "AGT-QA-0001");
        assert_eq!(view.lifecycle_status, "ACTIVE");
    }

    #[test]
    fn agent_list_response_deserializes_upstream_envelope() {
        let json = format!(
            r#"{{"data":[{{"id":"{AGENT_ID}","person_id":"22222222-3333-4444-5555-666666666666","agent_code":"AGT-QA-0001","lifecycle_status":"ACTIVE","version":1,"created_at":"2026-10-01T00:00:00Z"}}],"next_cursor":null}}"#
        );
        let resp: AgentListResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp.data.len(), 1);
        assert!(resp.next_cursor.is_none());
    }

    #[test]
    fn appointment_view_deserializes_nullable_refs_and_naive_dates() {
        // Upstream AppointmentDto uses Option refs and NaiveDate.
        let json = format!(
            r#"{{"id":"{LINK_ID}","agent_id":"{AGENT_ID}","appointment_reference":null,"company_reference":null,"appointment_type":"APPOINTMENT","effective_from":"2026-01-15","effective_to":null,"status":"ACTIVE"}}"#,
        );
        let view: AppointmentView = serde_json::from_str(&json).unwrap();
        assert!(view.appointment_reference.is_none());
        assert_eq!(
            view.effective_from,
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()
        );
    }
}
