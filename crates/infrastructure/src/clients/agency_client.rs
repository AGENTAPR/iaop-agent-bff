//! Client adapter for the authoritative `iaop-agency-service`.

use std::time::Duration;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AgencyClientError {
    #[error("network transport error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("entity not found")]
    NotFound,
    #[error("upstream agency service error ({status}): {message}")]
    Upstream { status: u16, message: String },
    #[error("deserialization error: {0}")]
    Serialization(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentAssignmentView {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub unit_id: Uuid,
    pub assignment_type: String,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgUnitView {
    pub id: Uuid,
    pub agency_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub code: String,
    pub name: String,
    pub kind: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeadershipView {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub agent_id: Uuid,
    pub role: String,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgencyView {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitAssignmentView {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub unit_id: Uuid,
    pub assignment_type: String,
    pub status: String,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub version: i64,
}

#[derive(Clone)]
pub struct AgencyClient {
    base_url: String,
    client: reqwest::Client,
}

impl AgencyClient {
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();
        Self { base_url, client }
    }

    pub async fn get_current_assignment(
        &self,
        agent_id: Uuid,
        bearer_token: &str,
        request_id: Option<Uuid>,
    ) -> Result<Option<CurrentAssignmentView>, AgencyClientError> {
        let url = format!(
            "{}/api/v1/agents/{}/current-assignment",
            self.base_url, agent_id
        );
        let mut req = self.client.get(&url).bearer_auth(bearer_token);

        if let Some(rid) = request_id {
            req = req.header("X-Request-ID", rid.to_string());
        }

        let resp = req.send().await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(AgencyClientError::Upstream { status, message });
        }

        let body = resp.json::<Option<CurrentAssignmentView>>().await?;
        Ok(body)
    }

    pub async fn get_unit(
        &self,
        unit_id: Uuid,
        bearer_token: &str,
        request_id: Option<Uuid>,
    ) -> Result<OrgUnitView, AgencyClientError> {
        let url = format!("{}/api/v1/organization-units/{}", self.base_url, unit_id);
        let mut req = self.client.get(&url).bearer_auth(bearer_token);

        if let Some(rid) = request_id {
            req = req.header("X-Request-ID", rid.to_string());
        }

        let resp = req.send().await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AgencyClientError::NotFound);
        }

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(AgencyClientError::Upstream { status, message });
        }

        let body = resp.json::<OrgUnitView>().await?;
        Ok(body)
    }

    pub async fn get_ancestors(
        &self,
        unit_id: Uuid,
        bearer_token: &str,
        request_id: Option<Uuid>,
    ) -> Result<Vec<OrgUnitView>, AgencyClientError> {
        let url = format!(
            "{}/api/v1/organization-units/{}/ancestors",
            self.base_url, unit_id
        );
        let mut req = self.client.get(&url).bearer_auth(bearer_token);

        if let Some(rid) = request_id {
            req = req.header("X-Request-ID", rid.to_string());
        }

        let resp = req.send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(AgencyClientError::Upstream { status, message });
        }

        let body = resp.json::<Vec<OrgUnitView>>().await?;
        Ok(body)
    }

    pub async fn list_agencies(
        &self,
        bearer_token: &str,
        request_id: Option<Uuid>,
    ) -> Result<Vec<AgencyView>, AgencyClientError> {
        let url = format!("{}/api/v1/agencies", self.base_url);
        let mut req = self.client.get(&url).bearer_auth(bearer_token);

        if let Some(rid) = request_id {
            req = req.header("X-Request-ID", rid.to_string());
        }

        let resp = req.send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(AgencyClientError::Upstream { status, message });
        }

        let body = resp.json::<Vec<AgencyView>>().await?;
        Ok(body)
    }

    /// GET /api/v1/organization-units/{unit_id}/assignments
    pub async fn get_unit_assignments(
        &self,
        unit_id: Uuid,
        bearer_token: &str,
        request_id: Option<Uuid>,
    ) -> Result<Vec<UnitAssignmentView>, AgencyClientError> {
        let url = format!(
            "{}/api/v1/organization-units/{}/assignments",
            self.base_url, unit_id
        );
        let mut req = self.client.get(&url).bearer_auth(bearer_token);

        if let Some(rid) = request_id {
            req = req.header("X-Request-ID", rid.to_string());
        }

        let resp = req.send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(AgencyClientError::Upstream { status, message });
        }

        let body = resp.json::<Vec<UnitAssignmentView>>().await?;
        Ok(body)
    }
}
