// Git-Synchronisation: klont/öffnet ein Repository, schreibt Projektdaten als JSON, committed und pusht.

use std::path::{Path, PathBuf};

use chrono::Utc;
use git2::{Cred, FetchOptions, PushOptions, RemoteCallbacks, Repository, Signature};
use tracing::{error, info};

use crate::models::project::{GitConfig, ProjectDoc};
use crate::state::AppState;

/// Basis-Verzeichnis für geklonte Repos.
fn repo_dir(project_id: &str) -> PathBuf {
    PathBuf::from("data/git").join(project_id)
}

/// Erzeugt RemoteCallbacks mit eingebetteten Credentials aus der URL.
fn make_callbacks(repo_url: &str) -> RemoteCallbacks<'_> {
    let mut callbacks = RemoteCallbacks::new();
    // HTTPS-Token-Auth: URL enthält Token, z.B. https://token:ghp_xxx@github.com/user/repo.git
    let url = repo_url.to_string();
    callbacks.credentials(move |_url, username_from_url, _allowed_types| {
        // Versuche Credentials aus der URL zu extrahieren
        if let Ok(parsed) = url::Url::parse(&url) {
            let user = if parsed.username().is_empty() {
                username_from_url.unwrap_or("git").to_string()
            } else {
                parsed.username().to_string()
            };
            let pass = parsed.password().unwrap_or("").to_string();
            if !pass.is_empty() {
                return Cred::userpass_plaintext(&user, &pass);
            }
        }
        // Fallback: Default-Credentials
        Cred::default()
    });
    callbacks
}

/// Klont ein Repository oder öffnet es, wenn es bereits existiert.
fn open_or_clone(repo_url: &str, branch: &str, local_path: &Path) -> Result<Repository, git2::Error> {
    if local_path.join(".git").exists() {
        let repo = Repository::open(local_path)?;
        // Pull: fetch + reset auf remote branch
        {
            let mut fo = FetchOptions::new();
            fo.remote_callbacks(make_callbacks(repo_url));
            let mut remote = repo.find_remote("origin")?;
            remote.fetch(&[branch], Some(&mut fo), None)?;
        }
        // Reset auf den neuesten Remote-Stand
        {
            let fetch_head = repo.find_reference(&format!("refs/remotes/origin/{branch}"))?;
            let commit = fetch_head.peel_to_commit()?;
            repo.reset(commit.as_object(), git2::ResetType::Hard, None)?;
        }
        Ok(repo)
    } else {
        // Verzeichnis erstellen und klonen
        std::fs::create_dir_all(local_path)
            .map_err(|e| git2::Error::from_str(&format!("Verzeichnis erstellen fehlgeschlagen: {e}")))?;

        let mut fo = FetchOptions::new();
        fo.remote_callbacks(make_callbacks(repo_url));

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fo);
        builder.branch(branch);
        builder.clone(repo_url, local_path)
    }
}

/// Synchronisiert ein Projekt ins konfigurierte Git-Repository.
/// Schreibt die Projektdaten als JSON, committed und pusht.
pub fn sync_project_to_git(project: &ProjectDoc, config: &GitConfig) -> Result<(), String> {
    let local_path = repo_dir(&project.id);
    let branch = &config.branch;

    // Repository öffnen oder klonen
    let repo = open_or_clone(&config.repo_url, branch, &local_path)
        .map_err(|e| format!("Git open/clone fehlgeschlagen: {e}"))?;

    // Projektdaten als JSON schreiben (ohne git-Feld, um Rekursion zu vermeiden)
    let mut export = project.clone();
    export.git = None;
    let json = serde_json::to_string_pretty(&export)
        .map_err(|e| format!("JSON-Serialisierung fehlgeschlagen: {e}"))?;

    let file_path = local_path.join(&config.path);
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Unterverzeichnis erstellen fehlgeschlagen: {e}"))?;
    }
    std::fs::write(&file_path, &json)
        .map_err(|e| format!("Datei schreiben fehlgeschlagen: {e}"))?;

    // Git add
    let mut index = repo.index()
        .map_err(|e| format!("Index öffnen fehlgeschlagen: {e}"))?;
    index.add_path(Path::new(&config.path))
        .map_err(|e| format!("Git add fehlgeschlagen: {e}"))?;
    index.write()
        .map_err(|e| format!("Index schreiben fehlgeschlagen: {e}"))?;

    // Prüfen ob es Änderungen gibt
    let tree_oid = index.write_tree()
        .map_err(|e| format!("Tree schreiben fehlgeschlagen: {e}"))?;

    let head = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    if let Some(ref parent) = head {
        if parent.tree().ok().map(|t| t.id()) == Some(tree_oid) {
            // Keine Änderungen – nichts zu tun
            return Ok(());
        }
    }

    let tree = repo.find_tree(tree_oid)
        .map_err(|e| format!("Tree finden fehlgeschlagen: {e}"))?;

    // Commit erstellen
    let sig = Signature::now("Plankton", "plankton@localhost")
        .map_err(|e| format!("Signatur erstellen fehlgeschlagen: {e}"))?;
    let message = format!("plankton: update {}", project.title);

    let parents: Vec<&git2::Commit> = head.as_ref().map(|c| vec![c]).unwrap_or_default();
    repo.commit(
        Some(&format!("refs/heads/{branch}")),
        &sig,
        &sig,
        &message,
        &tree,
        &parents,
    )
    .map_err(|e| format!("Commit fehlgeschlagen: {e}"))?;

    // Push
    let mut remote = repo.find_remote("origin")
        .map_err(|e| format!("Remote 'origin' nicht gefunden: {e}"))?;
    let mut push_opts = PushOptions::new();
    push_opts.remote_callbacks(make_callbacks(&config.repo_url));
    remote.push(
        &[&format!("refs/heads/{branch}:refs/heads/{branch}")],
        Some(&mut push_opts),
    )
    .map_err(|e| format!("Push fehlgeschlagen: {e}"))?;

    Ok(())
}

/// Führt den Git-Sync für ein Projekt aus und aktualisiert die GitConfig (last_push/last_error).
pub async fn perform_git_sync(state: &AppState, project_id: &str) -> Result<(), String> {
    let mut project = state.store.get_project(project_id).await
        .map_err(|e| format!("Projekt laden fehlgeschlagen: {e}"))?;

    let config = match &project.git {
        Some(c) if c.enabled => c.clone(),
        Some(_) => return Err("Git-Sync ist deaktiviert".into()),
        None => return Err("Keine Git-Konfiguration vorhanden".into()),
    };

    // Sync in einem Blocking-Thread (git2 ist synchron)
    let project_clone = project.clone();
    let config_clone = config.clone();
    let result = tokio::task::spawn_blocking(move || {
        sync_project_to_git(&project_clone, &config_clone)
    })
    .await
    .map_err(|e| format!("Spawn-Fehler: {e}"))?;

    // GitConfig aktualisieren
    let git = project.git.as_mut().unwrap();
    match &result {
        Ok(()) => {
            git.last_push = Some(Utc::now().to_rfc3339());
            git.last_error = None;
            info!("Git-Sync erfolgreich für Projekt {project_id}");
        }
        Err(err) => {
            git.last_error = Some(err.clone());
            error!("Git-Sync fehlgeschlagen für Projekt {project_id}: {err}");
        }
    }

    // Projekt mit aktualisierter GitConfig speichern
    state.store.put_project(project).await
        .map_err(|e| format!("Projekt speichern fehlgeschlagen: {e}"))?;

    result
}
