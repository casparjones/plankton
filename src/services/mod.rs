// Service-Module: Business-Logik und Hilfsfunktionen.

pub mod auth_service;
pub mod git_service;
pub mod project_service;
pub mod webhook_service;

pub use auth_service::*;
pub use git_service::*;
pub use project_service::*;
// webhook_service: explizite Verwendung via crate::services::webhook_service
