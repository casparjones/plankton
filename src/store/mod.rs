// Storage-Backend: CouchDB und lokaler File-Store.

pub mod couch;
pub mod file;

use std::path::PathBuf;

use crate::error::ApiError;
use crate::models::*;

pub use couch::CouchDb;
pub use file::FileStore;

/// Enum, das CouchDB und den lokalen File-Store vereint.
/// Alle Methoden werden über `DataStore::*` aufgerufen und delegieren
/// intern an das passende Backend.
#[derive(Clone)]
pub enum DataStore {
    Couch(CouchDb),
    File(FileStore),
}

impl DataStore {
    pub async fn list_projects(&self) -> Result<Vec<ProjectDoc>, ApiError> {
        match self {
            DataStore::Couch(c) => c.list_projects().await,
            DataStore::File(f) => f.list_projects().await,
        }
    }

    pub async fn create_project(&self, project: ProjectDoc) -> Result<ProjectDoc, ApiError> {
        match self {
            DataStore::Couch(c) => c.create_project(project).await,
            DataStore::File(f) => f.create_project(project).await,
        }
    }

    pub async fn get_project(&self, id: &str) -> Result<ProjectDoc, ApiError> {
        match self {
            DataStore::Couch(c) => c.get_project(id).await,
            DataStore::File(f) => f.get_project(id).await,
        }
    }

    /// Löst eine Projekt-Referenz auf: akzeptiert UUID oder Slug.
    pub async fn resolve_project(&self, id_or_slug: &str) -> Result<ProjectDoc, ApiError> {
        // Try direct ID lookup first
        match self.get_project(id_or_slug).await {
            Ok(p) => return Ok(p),
            Err(ApiError::NotFound(_)) => {}
            Err(e) => return Err(e),
        }
        // Fallback: search by slug
        let projects = self.list_projects().await?;
        projects
            .into_iter()
            .find(|p| p.slug == id_or_slug)
            .ok_or_else(|| ApiError::NotFound(format!("Project '{id_or_slug}' not found")))
    }

    /// Löst eine Projekt-Referenz auf und gibt nur die ID zurück.
    pub async fn resolve_project_id(&self, id_or_slug: &str) -> Result<String, ApiError> {
        // If it looks like a UUID, try direct lookup
        if id_or_slug.len() == 36 && id_or_slug.contains('-') {
            match self.get_project(id_or_slug).await {
                Ok(p) => return Ok(p.id),
                Err(ApiError::NotFound(_)) => {}
                Err(e) => return Err(e),
            }
        }
        // Search by slug
        let projects = self.list_projects().await?;
        projects
            .iter()
            .find(|p| p.slug == id_or_slug || p.id == id_or_slug)
            .map(|p| p.id.clone())
            .ok_or_else(|| ApiError::NotFound(format!("Project '{id_or_slug}' not found")))
    }

    pub async fn put_project(&self, project: ProjectDoc) -> Result<ProjectDoc, ApiError> {
        match self {
            DataStore::Couch(c) => c.put_project(project).await,
            DataStore::File(f) => f.put_project(project).await,
        }
    }

    pub async fn delete_project(&self, id: &str, rev: &str) -> Result<(), ApiError> {
        match self {
            DataStore::Couch(c) => c.delete_project(id, rev).await,
            DataStore::File(f) => f.delete_project(id, rev).await,
        }
    }

    // ---- User-Management (immer Dateisystem-basiert) ----

    fn users_root(&self) -> PathBuf {
        PathBuf::from("data/users")
    }

    fn user_path(&self, id: &str) -> PathBuf {
        self.users_root().join(format!("{id}.json"))
    }

    pub async fn ensure_users_dir(&self) -> Result<(), ApiError> {
        tokio::fs::create_dir_all(self.users_root()).await?;
        Ok(())
    }

    pub async fn list_users(&self) -> Result<Vec<AuthUser>, ApiError> {
        let dir = self.users_root();
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut out = vec![];
        let mut entries = tokio::fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let content = tokio::fs::read_to_string(path).await?;
            let user: AuthUser = serde_json::from_str(&content)?;
            out.push(user);
        }
        Ok(out)
    }

    pub async fn get_user(&self, id: &str) -> Result<AuthUser, ApiError> {
        let path = self.user_path(id);
        if !path.exists() {
            return Err(ApiError::NotFound(format!("User {id} not found")));
        }
        let content = tokio::fs::read_to_string(path).await?;
        Ok(serde_json::from_str(&content)?)
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<AuthUser, ApiError> {
        let users = self.list_users().await?;
        users
            .into_iter()
            .find(|u| u.username == username)
            .ok_or_else(|| ApiError::NotFound(format!("User '{username}' not found")))
    }

    pub async fn create_user(&self, mut user: AuthUser) -> Result<AuthUser, ApiError> {
        if user.id.is_empty() {
            user.id = uuid::Uuid::new_v4().to_string();
        }
        let content = serde_json::to_string_pretty(&user)?;
        tokio::fs::write(self.user_path(&user.id), content).await?;
        Ok(user)
    }

    pub async fn update_user(&self, user: AuthUser) -> Result<AuthUser, ApiError> {
        let path = self.user_path(&user.id);
        if !path.exists() {
            return Err(ApiError::NotFound(format!("User {} not found", user.id)));
        }
        let content = serde_json::to_string_pretty(&user)?;
        tokio::fs::write(path, content).await?;
        Ok(user)
    }

    pub async fn delete_user(&self, id: &str) -> Result<(), ApiError> {
        let path = self.user_path(id);
        if !path.exists() {
            return Err(ApiError::NotFound(format!("User {id} not found")));
        }
        tokio::fs::remove_file(path).await?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Token-Verwaltung (immer File-basiert)
    // ------------------------------------------------------------------

    fn tokens_root(&self) -> PathBuf {
        PathBuf::from("data/tokens")
    }

    fn token_path(&self, id: &str) -> PathBuf {
        self.tokens_root().join(format!("{id}.json"))
    }

    async fn ensure_tokens_dir(&self) -> Result<(), ApiError> {
        tokio::fs::create_dir_all(self.tokens_root()).await?;
        Ok(())
    }

    pub async fn list_tokens(&self) -> Result<Vec<AgentToken>, ApiError> {
        self.ensure_tokens_dir().await?;
        let mut tokens = Vec::new();
        let mut dir = tokio::fs::read_dir(self.tokens_root()).await?;
        while let Some(entry) = dir.next_entry().await? {
            if entry.path().extension().map(|e| e == "json").unwrap_or(false) {
                let data = tokio::fs::read_to_string(entry.path()).await?;
                let token: AgentToken = serde_json::from_str(&data)?;
                tokens.push(token);
            }
        }
        Ok(tokens)
    }

    pub async fn get_token(&self, id: &str) -> Result<AgentToken, ApiError> {
        let data = tokio::fs::read_to_string(self.token_path(id))
            .await
            .map_err(|_| ApiError::NotFound("Token not found".into()))?;
        Ok(serde_json::from_str(&data)?)
    }

    pub async fn get_token_by_value(&self, token_value: &str) -> Result<AgentToken, ApiError> {
        let tokens = self.list_tokens().await?;
        tokens
            .into_iter()
            .find(|t| t.token == token_value && t.active)
            .ok_or_else(|| ApiError::NotFound("Token not found or inactive".into()))
    }

    pub async fn create_token(&self, token: AgentToken) -> Result<AgentToken, ApiError> {
        self.ensure_tokens_dir().await?;
        let data = serde_json::to_string_pretty(&token)?;
        tokio::fs::write(self.token_path(&token.id), data).await?;
        Ok(token)
    }

    pub async fn update_token(&self, token: AgentToken) -> Result<AgentToken, ApiError> {
        let data = serde_json::to_string_pretty(&token)?;
        tokio::fs::write(self.token_path(&token.id), data).await?;
        Ok(token)
    }

    pub async fn delete_token(&self, id: &str) -> Result<(), ApiError> {
        tokio::fs::remove_file(self.token_path(id))
            .await
            .map_err(|_| ApiError::NotFound("Token not found".into()))?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // OAuth-Persistence (Codes, Clients, Refresh Tokens)
    // ------------------------------------------------------------------

    fn oauth_root(&self) -> PathBuf { PathBuf::from("data/oauth") }
    fn oauth_codes_root(&self) -> PathBuf { self.oauth_root().join("codes") }
    fn oauth_clients_root(&self) -> PathBuf { self.oauth_root().join("clients") }
    fn oauth_refresh_root(&self) -> PathBuf { self.oauth_root().join("refresh") }

    pub async fn ensure_oauth_dirs(&self) -> Result<(), ApiError> {
        tokio::fs::create_dir_all(self.oauth_codes_root()).await?;
        tokio::fs::create_dir_all(self.oauth_clients_root()).await?;
        tokio::fs::create_dir_all(self.oauth_refresh_root()).await?;
        Ok(())
    }

    // OAuth Codes
    pub async fn save_oauth_code(&self, code: &OAuthAuthCode) -> Result<(), ApiError> {
        self.ensure_oauth_dirs().await?;
        let path = self.oauth_codes_root().join(format!("{}.json", code.code));
        tokio::fs::write(path, serde_json::to_string_pretty(code)?).await?;
        Ok(())
    }

    pub async fn take_oauth_code(&self, code: &str) -> Result<OAuthAuthCode, ApiError> {
        let path = self.oauth_codes_root().join(format!("{code}.json"));
        let data = tokio::fs::read_to_string(&path).await
            .map_err(|_| ApiError::BadRequest("Invalid or expired code".into()))?;
        let auth_code: OAuthAuthCode = serde_json::from_str(&data)?;
        // Einmalig: Datei löschen
        let _ = tokio::fs::remove_file(path).await;
        Ok(auth_code)
    }

    // OAuth Clients
    pub async fn save_oauth_client(&self, client: &OAuthClient) -> Result<(), ApiError> {
        self.ensure_oauth_dirs().await?;
        let path = self.oauth_clients_root().join(format!("{}.json", client.client_id));
        tokio::fs::write(path, serde_json::to_string_pretty(client)?).await?;
        Ok(())
    }

    pub async fn get_oauth_client(&self, client_id: &str) -> Result<OAuthClient, ApiError> {
        let path = self.oauth_clients_root().join(format!("{client_id}.json"));
        let data = tokio::fs::read_to_string(path).await
            .map_err(|_| ApiError::NotFound("OAuth client not found".into()))?;
        Ok(serde_json::from_str(&data)?)
    }

    pub async fn list_oauth_clients(&self) -> Result<Vec<OAuthClient>, ApiError> {
        self.ensure_oauth_dirs().await?;
        let mut clients = Vec::new();
        let mut dir = tokio::fs::read_dir(self.oauth_clients_root()).await?;
        while let Some(entry) = dir.next_entry().await? {
            if entry.path().extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(data) = tokio::fs::read_to_string(entry.path()).await {
                    if let Ok(client) = serde_json::from_str(&data) {
                        clients.push(client);
                    }
                }
            }
        }
        Ok(clients)
    }

    // OAuth Refresh Tokens
    pub async fn save_refresh_token(&self, token: &OAuthRefreshToken) -> Result<(), ApiError> {
        self.ensure_oauth_dirs().await?;
        let path = self.oauth_refresh_root().join(format!("{}.json", token.token));
        tokio::fs::write(path, serde_json::to_string_pretty(token)?).await?;
        Ok(())
    }

    pub async fn take_refresh_token(&self, token: &str) -> Result<OAuthRefreshToken, ApiError> {
        let path = self.oauth_refresh_root().join(format!("{token}.json"));
        let data = tokio::fs::read_to_string(&path).await
            .map_err(|_| ApiError::BadRequest("Invalid refresh token".into()))?;
        let refresh: OAuthRefreshToken = serde_json::from_str(&data)?;
        let _ = tokio::fs::remove_file(path).await;
        Ok(refresh)
    }
}
