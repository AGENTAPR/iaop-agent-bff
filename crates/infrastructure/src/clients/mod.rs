//! Typed HTTP clients for interacting with IAOP domain services.

pub mod agent_client;

pub use agent_client::{
    AgentClient, AgentClientError, AgentView, AppointmentView, IdentityLinkView,
};
