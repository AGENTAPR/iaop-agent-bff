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
pub struct CurrentAssignmentView {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub unit_id: Uuid,
    pub assignment_type: String,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct LeadershipView {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub agent_id: Uuid,
    pub role: String,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgencyView {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Returns ancestor unit IDs (upstream contract: `GET .../ancestors` -> `[string]`).
    /// Callers resolve each ID via `get_unit` for full unit details.
    pub async fn get_ancestors(
        &self,
        unit_id: Uuid,
        bearer_token: &str,
        request_id: Option<Uuid>,
    ) -> Result<Vec<Uuid>, AgencyClientError> {
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

        let body = resp.json::<Vec<Uuid>>().await?;
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

#[cfg(test)]
mod contract_tests {
    //! Consumer contract tests: fixtures mirror the ACTUAL serialized shape
    //! produced by iaop-agency-service (snake_case fields, SCREAMING_SNAKE
    //! enums, ISO dates, UUID strings). If upstream changes casing or shape,
    //! these tests fail before the mismatch reaches production.

    use super::*;

    const AGENT_ID: &str = "11111111-2222-3333-4444-555555555555";
    const UNIT_ID: &str = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
    const ASSIGNMENT_ID: &str = "99999999-8888-7777-6666-555555555555";

    #[test]
    fn current_assignment_view_deserializes_upstream_shape() {
        // Mirrors AssignmentDto from iaop-agency-service (snake_case).
        let json = format!(
            r#"{{"id":"{ASSIGNMENT_ID}","agent_id":"{AGENT_ID}","unit_id":"{UNIT_ID}","assignment_type":"PRIMARY","status":"ACTIVE","effective_from":"2025-01-15","effective_to":null,"version":3}}"#
        );
        let view: CurrentAssignmentView = serde_json::from_str(&json).unwrap();
        assert_eq!(view.agent_id.to_string(), AGENT_ID);
        assert_eq!(view.unit_id.to_string(), UNIT_ID);
        assert_eq!(view.assignment_type, "PRIMARY");
        assert_eq!(
            view.effective_from,
            NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()
        );
        assert!(view.effective_to.is_none());
    }

    #[test]
    fn org_unit_view_deserializes_upstream_shape() {
        // Mirrors UnitDto (snake_case, nullable parent_id).
        let json = format!(
            r#"{{"id":"{UNIT_ID}","agency_id":"{AGENT_ID}","parent_id":null,"code":"AG-SBY-01","name":"Agency South Bay","kind":"AGENCY","status":"ACTIVE","version":1}}"#
        );
        let view: OrgUnitView = serde_json::from_str(&json).unwrap();
        assert_eq!(view.code, "AG-SBY-01");
        assert_eq!(view.kind, "AGENCY");
        assert!(view.parent_id.is_none());
    }

    #[test]
    fn agency_view_deserializes_upstream_shape() {
        // Mirrors AgencyDto.
        let json = format!(
            r#"{{"id":"{AGENT_ID}","code":"GA-001","name":"General Agency","status":"ACTIVE","version":2}}"#
        );
        let view: AgencyView = serde_json::from_str(&json).unwrap();
        assert_eq!(view.code, "GA-001");
        assert_eq!(view.status, "ACTIVE");
    }

    #[test]
    fn unit_assignment_view_deserializes_upstream_shape() {
        // Mirrors AssignmentDto for the unit-assignments list endpoint.
        let json = format!(
            r#"[{{"id":"{ASSIGNMENT_ID}","agent_id":"{AGENT_ID}","unit_id":"{UNIT_ID}","assignment_type":"SECONDARY","status":"ENDED","effective_from":"2024-06-01","effective_to":"2025-01-14","version":7}}]"#
        );
        let views: Vec<UnitAssignmentView> = serde_json::from_str(&json).unwrap();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].assignment_type, "SECONDARY");
        assert_eq!(views[0].status, "ENDED");
        assert_eq!(views[0].version, 7);
    }

    #[test]
    fn ancestors_response_is_array_of_uuid_strings() {
        // Upstream GET .../ancestors returns `[string]` unit IDs.
        let json = format!(r#"["{UNIT_ID}","{AGENT_ID}"]"#);
        let ids: Vec<Uuid> = serde_json::from_str(&json).unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0].to_string(), UNIT_ID);
    }

    #[test]
    fn leadership_view_deserializes_upstream_shape() {
        // Mirrors LeadershipDto.
        let json = format!(
            r#"{{"id":"{ASSIGNMENT_ID}","unit_id":"{UNIT_ID}","agent_id":"{AGENT_ID}","role":"UNIT_MANAGER","effective_from":"2025-02-01","effective_to":null,"status":"ACTIVE","version":1}}"#
        );
        let view: LeadershipView = serde_json::from_str(&json).unwrap();
        assert_eq!(view.role, "UNIT_MANAGER");
        assert_eq!(view.unit_id.to_string(), UNIT_ID);
    }

    #[test]
    fn camel_case_payload_is_rejected() {
        // Regression guard: upstream uses snake_case; a camelCase payload must
        // NOT silently deserialize (missing required fields -> error).
        let json = r#"{"agentId":"11111111-2222-3333-4444-555555555555","unitId":"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"}"#;
        assert!(serde_json::from_str::<CurrentAssignmentView>(json).is_err());
    }
}
