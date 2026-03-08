// CouchDB Storage-Backend.

use reqwest::Client;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::ProjectDoc;

/// HTTP-Client-Wrapper für CouchDB.
#[derive(Clone)]
pub struct CouchDb {
    pub client: Client,
    /// Basis-URL, z.B. "http://admin:password@localhost:5984"
    pub base_url: String,
    /// Name der Datenbank, z.B. "plankton"
    pub db: String,
}

impl CouchDb {
    /// Stellt sicher, dass die Datenbank existiert (idempotenter PUT).
    pub async fn ensure_db(&self) -> anyhow::Result<()> {
        let url = format!("{}/{}", self.base_url, self.db);
        let resp = self.client.put(url).send().await?;
        // 412 Precondition Failed bedeutet: DB existiert bereits – kein Fehler.
        if !(resp.status().is_success() || resp.status().as_u16() == 412) {
            anyhow::bail!("Failed to ensure DB");
        }
        Ok(())
    }

    /// Listet alle Dokumente in der Datenbank auf.
    pub async fn list_projects(&self) -> Result<Vec<ProjectDoc>, ApiError> {
        #[derive(Deserialize)]
        struct AllDocs {
            rows: Vec<Row>,
        }
        #[derive(Deserialize)]
        struct Row {
            doc: Option<ProjectDoc>,
        }

        let url = format!("{}/{}/_all_docs?include_docs=true", self.base_url, self.db);
        let rows: AllDocs = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(rows.rows.into_iter().filter_map(|r| r.doc).collect())
    }

    /// Legt ein neues Dokument in CouchDB an (POST).
    pub async fn create_project(&self, mut project: ProjectDoc) -> Result<ProjectDoc, ApiError> {
        if project.id.is_empty() {
            project.id = Uuid::new_v4().to_string();
        }
        let url = format!("{}/{}", self.base_url, self.db);
        let res: serde_json::Value = self
            .client
            .post(url)
            .json(&project)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        project.rev = res["rev"].as_str().map(ToString::to_string);
        Ok(project)
    }

    /// Liest ein einzelnes Dokument aus CouchDB.
    pub async fn get_project(&self, id: &str) -> Result<ProjectDoc, ApiError> {
        let url = format!("{}/{}/{}", self.base_url, self.db, id);
        let proj = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json::<ProjectDoc>()
            .await?;
        Ok(proj)
    }

    /// Schreibt ein vorhandenes Dokument zurück (PUT mit Rev).
    pub async fn put_project(&self, mut project: ProjectDoc) -> Result<ProjectDoc, ApiError> {
        let id = project.id.clone();
        let url = format!("{}/{}/{}", self.base_url, self.db, id);
        let res: serde_json::Value = self
            .client
            .put(url)
            .json(&project)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        project.rev = res["rev"].as_str().map(ToString::to_string);
        Ok(project)
    }

    /// Löscht ein Dokument in CouchDB (erfordert aktuelle Rev).
    pub async fn delete_project(&self, id: &str, rev: &str) -> Result<(), ApiError> {
        let url = format!("{}/{}/{}?rev={}", self.base_url, self.db, id, rev);
        self.client.delete(url).send().await?.error_for_status()?;
        Ok(())
    }
}
