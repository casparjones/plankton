// Lokaler File-Store: Jedes Projekt wird als JSON-Datei gespeichert.

use std::path::PathBuf;

use uuid::Uuid;

use crate::error::ApiError;
use crate::models::ProjectDoc;

/// Lokaler File-Store: Jedes Projekt wird als `<id>.json` in `root` gespeichert.
#[derive(Clone)]
pub struct FileStore {
    pub root: PathBuf,
}

impl FileStore {
    /// Erstellt das Root-Verzeichnis, falls es nicht existiert.
    pub async fn ensure_db(&self) -> Result<(), ApiError> {
        tokio::fs::create_dir_all(&self.root).await?;
        Ok(())
    }

    /// Gibt den Dateipfad für ein Projekt zurück.
    fn project_path(&self, id: &str) -> PathBuf {
        self.root.join(format!("{id}.json"))
    }

    /// Liest alle JSON-Dateien im Root-Verzeichnis ein.
    pub async fn list_projects(&self) -> Result<Vec<ProjectDoc>, ApiError> {
        let mut out = vec![];
        let mut entries = tokio::fs::read_dir(&self.root).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let content = tokio::fs::read_to_string(path).await?;
            let project: ProjectDoc = serde_json::from_str(&content)?;
            out.push(project);
        }
        Ok(out)
    }

    /// Schreibt ein neues Projekt als JSON-Datei. Startet mit Rev "1".
    pub async fn create_project(&self, mut project: ProjectDoc) -> Result<ProjectDoc, ApiError> {
        if project.id.is_empty() {
            project.id = Uuid::new_v4().to_string();
        }
        project.rev = Some("1".into());
        let content = serde_json::to_string_pretty(&project)?;
        tokio::fs::write(self.project_path(&project.id), content).await?;
        Ok(project)
    }

    /// Liest ein Projekt aus einer JSON-Datei.
    pub async fn get_project(&self, id: &str) -> Result<ProjectDoc, ApiError> {
        let path = self.project_path(id);
        if !path.exists() {
            return Err(ApiError::NotFound(format!("Project {id} not found")));
        }
        let content = tokio::fs::read_to_string(path).await?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Überschreibt eine Projektdatei. Prüft Revisions-Übereinstimmung (optimistisches Locking).
    pub async fn put_project(&self, mut project: ProjectDoc) -> Result<ProjectDoc, ApiError> {
        let current = self.get_project(&project.id).await?;
        let current_rev = current.rev.unwrap_or_else(|| "0".into());
        let given_rev = project.rev.clone().unwrap_or_else(|| "".into());
        if given_rev != current_rev {
            return Err(ApiError::Conflict(format!(
                "Revision conflict: expected {current_rev}, got {given_rev}"
            )));
        }
        // Rev inkrementieren.
        let next_rev = current_rev.parse::<u64>().unwrap_or(0) + 1;
        project.rev = Some(next_rev.to_string());
        let content = serde_json::to_string_pretty(&project)?;
        tokio::fs::write(self.project_path(&project.id), content).await?;
        Ok(project)
    }

    /// Löscht eine Projektdatei nach Rev-Prüfung.
    pub async fn delete_project(&self, id: &str, rev: &str) -> Result<(), ApiError> {
        let current = self.get_project(id).await?;
        if current.rev.as_deref().unwrap_or("") != rev {
            return Err(ApiError::Conflict("Revision conflict on delete".into()));
        }
        tokio::fs::remove_file(self.project_path(id)).await?;
        Ok(())
    }
}
