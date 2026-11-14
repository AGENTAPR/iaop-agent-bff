//! Client adapter for the authoritative `iaop-agent-service`.

use std::time::Duration;

use chrono::{DateTime, Utc};
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct AppointmentView {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub appointment_reference: String,
    pub company_reference: String,
    pub appointment_type: String,
    pub effective_from: DateTime<Utc>,
    pub effective_to: Option<DateTime<Utc>>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppointmentListResponse {
    pub data: Vec<AppointmentView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
