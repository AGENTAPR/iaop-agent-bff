//! Client adapter for authoritative eligibility decisions.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum EligibilityClientError {
    #[error("network transport error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("upstream service error ({status}): {message}")]
    Upstream { status: u16, message: String },
    #[error("deserialization error: {0}")]
    Serialization(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EligibilityDecision {
    pub is_eligible: bool,
    pub reason: Option<String>,
    pub valid_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct EligibilityClient {
    base_url: String,
    http: reqwest::Client,
}

impl EligibilityClient {
    pub fn new(base_url: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { base_url, http }
    }

    pub async fn verify_eligibility(
        &self,
        agent_id: Uuid,
        token: &str,
        request_id: Option<Uuid>,
    ) -> Result<EligibilityDecision, EligibilityClientError> {
        let url = format!(
            "{}/api/v1/eligibility/agents/{}/status",
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
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(EligibilityClientError::Upstream {
                status,
                message: text,
            });
        }

        resp.json::<EligibilityDecision>()
            .await
            .map_err(|e| EligibilityClientError::Serialization(e.to_string()))
    }
}
