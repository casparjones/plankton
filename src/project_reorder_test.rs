//! Integrationstests für POST /api/projects/reorder.
//!
//! Feature: `reorder_projects`-Handler
//! - POST /api/projects/reorder mit `{"ids": ["id2", "id1"]}` → 200
//! - Danach liefert GET /api/projects die Projekte in der neuen Reihenfolge.
//! - Das `order`-Feld auf ProjectDoc muss persistiert werden.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::{get, post};
    use axum::Router;
    use tower::ServiceExt;

    use crate::controllers::project_controller::{list_projects, reorder_projects};
    use crate::middleware::auth_guard;
    use crate::models::auth::AuthUser;
    use crate::services::auth_service::create_jwt;
    use crate::services::project_service::default_project;
    use crate::state::AppState;
    use crate::store::{DataStore, FileStore};

    async fn make_test_state() -> (AppState, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = DataStore::File(FileStore {
            root: dir.path().to_path_buf(),
        });
        store.ensure_users_dir().await.ok();

        let state = AppState {
            store,
            events: Arc::new(Mutex::new(HashMap::new())),
            jwt_secret: "test-secret".into(),
            cli_sessions: Arc::new(Mutex::new(HashMap::new())),
            mcp_sessions: Arc::new(Mutex::new(HashMap::new())),
            oauth_clients: Arc::new(Mutex::new(Vec::new())),
            oauth_codes: Arc::new(Mutex::new(HashMap::new())),
            oauth_refresh_tokens: Arc::new(Mutex::new(HashMap::new())),
            write_locks: Arc::new(Mutex::new(HashMap::new())),
            http_client: reqwest::Client::new(),
        };
        (state, dir)
    }

    fn make_app(state: AppState) -> Router {
        Router::new()
            .route("/api/projects", get(list_projects))
            .route("/api/projects/reorder", post(reorder_projects))
            .with_state(state)
    }

    fn make_app_with_auth(state: AppState) -> Router {
        Router::new()
            .route("/api/projects/reorder", post(reorder_projects))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_guard,
            ))
            .with_state(state)
    }

    fn make_test_user() -> AuthUser {
        AuthUser {
            id: "test-user-id".into(),
            username: "testuser".into(),
            display_name: "Test User".into(),
            password_hash: "".into(),
            role: "admin".into(),
            created_at: "2024-01-01T00:00:00Z".into(),
            updated_at: "2024-01-01T00:00:00Z".into(),
            active: true,
        }
    }

    // -----------------------------------------------------------------------
    // Test 1: Reorder → 200 + neue Reihenfolge in GET /api/projects
    // -----------------------------------------------------------------------

    /// POST /api/projects/reorder setzt die order-Felder und
    /// GET /api/projects liefert danach Projekte in der neuen Reihenfolge zurück.
    #[tokio::test]
    async fn test_reorder_projects_persists_order() {
        let (state, _dir) = make_test_state().await;

        // Zwei Projekte anlegen
        let mut p1 = default_project("Alpha".into());
        let mut p2 = default_project("Beta".into());
        p1.order = 0;
        p2.order = 1;
        let p1 = state.store.create_project(p1).await.expect("create p1");
        let p2 = state.store.create_project(p2).await.expect("create p2");

        let app = make_app(state);

        // Reihenfolge umkehren: p2 soll vor p1 kommen
        let body = serde_json::json!({ "ids": [p2.id, p1.id] });
        let req = Request::builder()
            .method("POST")
            .uri("/api/projects/reorder")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();

        let resp = app.clone().oneshot(req).await.expect("request");
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "POST /api/projects/reorder muss 200 zurückgeben"
        );

        // GET /api/projects – Reihenfolge prüfen
        let req = Request::builder()
            .method("GET")
            .uri("/api/projects")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.expect("request");
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("body");
        let projects: Vec<serde_json::Value> = serde_json::from_slice(&bytes).expect("json");

        assert_eq!(projects.len(), 2, "Genau 2 Projekte erwartet");

        // Erstes Projekt in der Liste muss Beta (p2) sein
        let first_id = projects[0]["_id"].as_str().unwrap();
        let second_id = projects[1]["_id"].as_str().unwrap();
        assert_eq!(
            first_id, p2.id,
            "Beta muss nach Reorder als erstes Projekt erscheinen"
        );
        assert_eq!(
            second_id, p1.id,
            "Alpha muss nach Reorder als zweites Projekt erscheinen"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: order-Feld wird korrekt gesetzt
    // -----------------------------------------------------------------------

    /// Das `order`-Feld auf ProjectDoc entspricht dem Index in der gesendeten Liste.
    #[tokio::test]
    async fn test_reorder_projects_sets_order_field() {
        let (state, _dir) = make_test_state().await;

        let p1 = default_project("Gamma".into());
        let p2 = default_project("Delta".into());
        let p1 = state.store.create_project(p1).await.expect("create p1");
        let p2 = state.store.create_project(p2).await.expect("create p2");

        let app = make_app(state.clone());

        // p2 an Position 0, p1 an Position 1
        let body = serde_json::json!({ "ids": [p2.id, p1.id] });
        let req = Request::builder()
            .method("POST")
            .uri("/api/projects/reorder")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let resp = app.oneshot(req).await.expect("request");
        assert_eq!(resp.status(), StatusCode::OK);

        // Direkt aus Store lesen und order-Felder prüfen
        let stored_p2 = state.store.get_project(&p2.id).await.expect("get p2");
        let stored_p1 = state.store.get_project(&p1.id).await.expect("get p1");
        assert_eq!(stored_p2.order, 0, "Delta muss order=0 haben");
        assert_eq!(stored_p1.order, 1, "Gamma muss order=1 haben");
    }

    // -----------------------------------------------------------------------
    // Test 3: Unbekannte IDs werden übersprungen (kein 500)
    // -----------------------------------------------------------------------

    /// POST mit unbekannter ID liefert 200 (kein Crash).
    #[tokio::test]
    async fn test_reorder_unknown_ids_ignored() {
        let (state, _dir) = make_test_state().await;
        let p1 = default_project("Epsilon".into());
        let p1 = state.store.create_project(p1).await.expect("create");

        let app = make_app(state);

        let body = serde_json::json!({ "ids": ["unknown-id-xyz", p1.id] });
        let req = Request::builder()
            .method("POST")
            .uri("/api/projects/reorder")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let resp = app.oneshot(req).await.expect("request");
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Unbekannte IDs dürfen keinen Fehler verursachen"
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: Auth-Guard – kein Token → 401
    // -----------------------------------------------------------------------

    /// POST /api/projects/reorder ohne Auth-Token muss 401 zurückgeben.
    #[tokio::test]
    async fn test_reorder_requires_auth() {
        let (state, _dir) = make_test_state().await;
        let app = make_app_with_auth(state);

        let body = serde_json::json!({ "ids": [] });
        let req = Request::builder()
            .method("POST")
            .uri("/api/projects/reorder")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let resp = app.clone().oneshot(req).await.expect("request");
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "Ohne Token muss /api/projects/reorder 401 zurückgeben"
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: Auth-Guard – gültiges Token → 200
    // -----------------------------------------------------------------------

    /// POST /api/projects/reorder mit gültigem JWT-Token muss 200 zurückgeben.
    #[tokio::test]
    async fn test_reorder_accepts_valid_token() {
        let (state, _dir) = make_test_state().await;
        let user = make_test_user();
        let token = create_jwt(&user, &state.jwt_secret, false).expect("JWT erstellen");
        let app = make_app_with_auth(state);

        let body = serde_json::json!({ "ids": [] });
        let req = Request::builder()
            .method("POST")
            .uri("/api/projects/reorder")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(body.to_string()))
            .unwrap();
        let resp = app.oneshot(req).await.expect("request");
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Mit gültigem Token muss /api/projects/reorder 200 zurückgeben"
        );
    }
}
