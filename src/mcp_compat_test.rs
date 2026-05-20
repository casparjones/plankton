//! Integrationstests für MCP Streamable HTTP Transport – claude.ai Kompatibilität
//! (Ticket c88bd011)
//!
//! Prüft:
//! 1. `initialize`-Handshake: korrekte `protocolVersion`, `capabilities`, `serverInfo`
//! 2. `tools/list`: alle registrierten Tools mit vollständigen `inputSchema`-Feldern
//! 3. JSON-RPC 2.0 Fehlerformat bei unbekannter Methode: `code`, `message`, `id`
//! 4. `initialize`-Antwort enthält `mcp-session-id` Response-Header
//! 5. Notification (kein `id`) liefert keine Antwort (202 Accepted)
//! 6. CORS: `Access-Control-Allow-Origin` Header ist gesetzt (permissive)
//! 7. SSE-Stream via `GET /mcp`: Session muss vorhanden sein (kein auto-create)
//! 8. `DELETE /mcp` mit bekannter Session → 200; unbekannte Session → 404

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::{broadcast, Mutex};

    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use axum::routing::post;
    use axum::Router;
    use tower::ServiceExt; // oneshot
    use tower_http::cors::CorsLayer;

    use crate::controllers::mcp_controller::{mcp_jsonrpc, mcp_session_delete, mcp_sse_stream};
    use crate::state::{AppState, McpSession};
    use crate::store::{DataStore, FileStore};

    // -----------------------------------------------------------------------
    // Hilfsfunktionen
    // -----------------------------------------------------------------------

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

    /// Baut einen minimalen Router nur mit den /mcp-Endpunkten + CORS.
    fn build_mcp_router(state: AppState) -> Router {
        Router::new()
            .route(
                "/mcp",
                post(mcp_jsonrpc)
                    .get(mcp_sse_stream)
                    .delete(mcp_session_delete),
            )
            .layer(CorsLayer::permissive())
            .with_state(state)
    }

    /// Erstellt einen gültigen Agent-Token im Store und gibt dessen Klartext-Secret zurück.
    async fn create_test_token(state: &AppState, name: &str, role: &str) -> String {
        use crate::models::auth::{hash_token_secret, AgentToken, TokenScope};
        use uuid::Uuid;
        let token_value = format!("plk_{}", Uuid::new_v4().simple());
        let token = AgentToken {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            token_hash: hash_token_secret(&token_value),
            role: role.to_string(),
            active: true,
            created_at: chrono::Utc::now().to_rfc3339(),
            description: String::new(),
            creator: "test".to_string(),
            last_used: None,
            scope: TokenScope::default(),
            expires_at: None,
        };
        state.store.create_token(token).await.expect("create_token");
        token_value
    }

    /// Liest den Response-Body als String.
    async fn body_to_string(body: axum::body::Body) -> String {
        use http_body_util::BodyExt;
        let bytes = body.collect().await.expect("body collect").to_bytes();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    // -----------------------------------------------------------------------
    // Test 1: initialize-Handshake
    // -----------------------------------------------------------------------

    /// POST /mcp mit `initialize`-Request → 200, Body enthält
    /// `protocolVersion`, `capabilities.tools`, `serverInfo.name`.
    /// Response-Header `mcp-session-id` muss gesetzt sein.
    #[tokio::test]
    async fn test_initialize_handshake() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "test-agent", "admin").await;
        let app = build_mcp_router(state);

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0.1" }
            },
            "id": 1
        });

        let req = Request::builder()
            .method(Method::POST)
            .uri("/mcp")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.expect("oneshot");
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "initialize muss 200 zurückgeben"
        );

        // mcp-session-id Header muss gesetzt sein
        assert!(
            resp.headers().contains_key("mcp-session-id"),
            "mcp-session-id Header fehlt in initialize-Antwort"
        );

        let text = body_to_string(resp.into_body()).await;
        let json: serde_json::Value = serde_json::from_str(&text).expect("JSON parse");

        assert_eq!(json["jsonrpc"], "2.0", "jsonrpc Feld muss '2.0' sein");
        assert!(
            json["error"].is_null(),
            "initialize darf keinen error zurückgeben: {}",
            text
        );
        assert_eq!(
            json["result"]["protocolVersion"], "2024-11-05",
            "protocolVersion muss mit Client übereinstimmen"
        );
        assert!(
            !json["result"]["capabilities"].is_null(),
            "capabilities Feld fehlt"
        );
        assert!(
            !json["result"]["capabilities"]["tools"].is_null(),
            "capabilities.tools Feld fehlt"
        );
        assert_eq!(
            json["result"]["serverInfo"]["name"], "plankton",
            "serverInfo.name muss 'plankton' sein"
        );
        assert_eq!(json["id"], 1, "id muss mit Request-id übereinstimmen");
    }

    // -----------------------------------------------------------------------
    // Test 2: initialize mit protocolVersion 2025-03-26
    // -----------------------------------------------------------------------

    /// Server muss auch das neuere Protokoll 2025-03-26 widerspiegeln.
    #[tokio::test]
    async fn test_initialize_protocol_2025() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "test-agent", "admin").await;
        let app = build_mcp_router(state);

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": { "protocolVersion": "2025-03-26", "capabilities": {} },
            "id": "init-1"
        });

        let req = Request::builder()
            .method(Method::POST)
            .uri("/mcp")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.expect("oneshot");
        let text = body_to_string(resp.into_body()).await;
        let json: serde_json::Value = serde_json::from_str(&text).expect("JSON parse");

        assert_eq!(json["result"]["protocolVersion"], "2025-03-26");
    }

    // -----------------------------------------------------------------------
    // Test 3: tools/list – vollständige Schemas
    // -----------------------------------------------------------------------

    /// `tools/list` via POST /mcp → alle Tools haben `name`, `description`, `inputSchema`.
    #[tokio::test]
    async fn test_tools_list_complete_schemas() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "test-agent", "admin").await;

        // Erst initialize um Session zu erhalten
        let app = build_mcp_router(state);

        let init_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": { "protocolVersion": "2024-11-05", "capabilities": {} },
            "id": 1
        });

        let init_req = Request::builder()
            .method(Method::POST)
            .uri("/mcp")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(serde_json::to_vec(&init_body).unwrap()))
            .unwrap();

        let init_resp = app.clone().oneshot(init_req).await.expect("init oneshot");
        let session_id = init_resp
            .headers()
            .get("mcp-session-id")
            .and_then(|v| v.to_str().ok())
            .expect("session_id Header")
            .to_string();

        // Jetzt tools/list
        let list_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "params": {},
            "id": 2
        });

        let list_req = Request::builder()
            .method(Method::POST)
            .uri("/mcp")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .header("mcp-session-id", &session_id)
            .body(Body::from(serde_json::to_vec(&list_body).unwrap()))
            .unwrap();

        let list_resp = app.oneshot(list_req).await.expect("list oneshot");
        assert_eq!(list_resp.status(), StatusCode::OK);

        let text = body_to_string(list_resp.into_body()).await;
        let json: serde_json::Value = serde_json::from_str(&text).expect("JSON parse");

        assert_eq!(json["jsonrpc"], "2.0");
        assert!(
            json["error"].is_null(),
            "tools/list darf keinen Fehler zurückgeben: {}",
            text
        );

        let tools = json["result"]["tools"]
            .as_array()
            .expect("tools Array fehlt");
        assert!(!tools.is_empty(), "tools Liste darf nicht leer sein");

        // Jedes Tool muss name, description und inputSchema haben
        for tool in tools {
            let name = tool["name"].as_str().unwrap_or("<unnamed>");
            assert!(tool["name"].is_string(), "Tool fehlt 'name'-Feld: {tool}");
            assert!(
                tool["description"].is_string(),
                "Tool '{name}' fehlt 'description'"
            );
            assert!(
                !tool["inputSchema"].is_null(),
                "Tool '{name}' fehlt 'inputSchema'"
            );
            // inputSchema muss ein Objekt mit "type" sein
            assert_eq!(
                tool["inputSchema"]["type"], "object",
                "Tool '{name}' inputSchema.type muss 'object' sein"
            );
        }

        // Kerntools müssen vorhanden sein
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"list_projects"), "list_projects Tool fehlt");
        assert!(names.contains(&"get_project"), "get_project Tool fehlt");
        assert!(names.contains(&"get_task"), "get_task Tool fehlt");
        assert!(names.contains(&"create_task"), "create_task Tool fehlt");
        assert!(names.contains(&"add_comment"), "add_comment Tool fehlt");
        assert!(names.contains(&"move_task"), "move_task Tool fehlt");
    }

    // -----------------------------------------------------------------------
    // Test 4: JSON-RPC 2.0 Fehlerformat bei unbekannter Methode
    // -----------------------------------------------------------------------

    /// Unbekannte Methode → Fehler mit `code: -32601`, `message` enthält Methodenname,
    /// `id` entspricht dem Request-id, `jsonrpc` ist "2.0".
    #[tokio::test]
    async fn test_jsonrpc_error_format_unknown_method() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "test-agent", "admin").await;
        let app = build_mcp_router(state.clone());

        // Erst initialize
        let init_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": { "protocolVersion": "2024-11-05", "capabilities": {} },
            "id": 1
        });
        let init_req = Request::builder()
            .method(Method::POST)
            .uri("/mcp")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(serde_json::to_vec(&init_body).unwrap()))
            .unwrap();
        let init_resp = app.clone().oneshot(init_req).await.unwrap();
        let session_id = init_resp
            .headers()
            .get("mcp-session-id")
            .and_then(|v| v.to_str().ok())
            .unwrap()
            .to_string();

        // Unbekannte Methode aufrufen
        let bad_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "nonexistent/method",
            "params": {},
            "id": 42
        });
        let bad_req = Request::builder()
            .method(Method::POST)
            .uri("/mcp")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .header("mcp-session-id", &session_id)
            .body(Body::from(serde_json::to_vec(&bad_body).unwrap()))
            .unwrap();
        let bad_resp = app.oneshot(bad_req).await.unwrap();
        let text = body_to_string(bad_resp.into_body()).await;
        let json: serde_json::Value = serde_json::from_str(&text).expect("JSON parse");

        // JSON-RPC 2.0 Fehlerformat prüfen
        assert_eq!(json["jsonrpc"], "2.0", "jsonrpc muss '2.0' sein");
        assert!(
            json["result"].is_null(),
            "result muss null sein bei Fehler: {}",
            text
        );
        assert!(
            !json["error"].is_null(),
            "error Feld fehlt bei unbekannter Methode: {}",
            text
        );
        assert_eq!(
            json["error"]["code"], -32601,
            "error.code muss -32601 (Method not found) sein"
        );
        assert!(
            json["error"]["message"].is_string(),
            "error.message muss ein String sein"
        );
        assert!(
            json["error"]["message"]
                .as_str()
                .unwrap()
                .contains("nonexistent/method"),
            "error.message muss den Methodennamen enthalten: {}",
            json["error"]["message"]
        );
        assert_eq!(json["id"], 42, "id muss mit Request-id übereinstimmen");
    }

    // -----------------------------------------------------------------------
    // Test 5: JSON-RPC 2.0 Parse-Fehler (-32700)
    // -----------------------------------------------------------------------

    /// Ungültiger JSON-Body → `code: -32700` (Parse error).
    #[tokio::test]
    async fn test_jsonrpc_parse_error() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "test-agent", "admin").await;
        let app = build_mcp_router(state);

        let req = Request::builder()
            .method(Method::POST)
            .uri("/mcp")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(b"{ not valid json".as_ref()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        // 401 ist akzeptabel wenn Auth vor Parsing geprüft wird, aber nach Auth-Erfolg muss
        // ein Parse-Error als JSON-RPC-Fehler zurückkommen.
        // Mit gültigem Token: Body wird geparst → Parse-Fehler
        let text = body_to_string(resp.into_body()).await;
        let json: serde_json::Value = serde_json::from_str(&text).expect("Antwort muss JSON sein");
        assert_eq!(
            json["error"]["code"], -32700,
            "Parse-Fehler muss code -32700 haben"
        );
    }

    // -----------------------------------------------------------------------
    // Test 6: CORS-Header vorhanden
    // -----------------------------------------------------------------------

    /// OPTIONS-Preflight auf /mcp → Access-Control-Allow-Origin Header muss gesetzt sein.
    #[tokio::test]
    async fn test_cors_preflight_headers() {
        let (state, _dir) = make_test_state().await;
        let app = build_mcp_router(state);

        let req = Request::builder()
            .method(Method::OPTIONS)
            .uri("/mcp")
            .header("origin", "https://claude.ai")
            .header("access-control-request-method", "POST")
            .header(
                "access-control-request-headers",
                "content-type, authorization",
            )
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();

        // CorsLayer::permissive() antwortet auf OPTIONS mit 200 und CORS-Headern
        assert!(
            resp.status().is_success(),
            "OPTIONS preflight muss 2xx zurückgeben, war: {}",
            resp.status()
        );
        assert!(
            resp.headers().contains_key("access-control-allow-origin"),
            "access-control-allow-origin Header fehlt bei Preflight"
        );
    }

    /// POST /mcp → Access-Control-Allow-Origin muss auch bei normalen Requests gesetzt sein.
    #[tokio::test]
    async fn test_cors_header_on_post_response() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "test-agent", "admin").await;
        let app = build_mcp_router(state);

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": { "protocolVersion": "2024-11-05", "capabilities": {} },
            "id": 1
        });

        let req = Request::builder()
            .method(Method::POST)
            .uri("/mcp")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .header("origin", "https://claude.ai")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert!(
            resp.headers().contains_key("access-control-allow-origin"),
            "access-control-allow-origin fehlt in normaler POST-Antwort"
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: SSE-Stream GET /mcp – bekannte Session
    // -----------------------------------------------------------------------

    /// GET /mcp mit gültigem Token und bekannter Session-ID → 200 (SSE-Stream beginnt).
    #[tokio::test]
    async fn test_sse_stream_known_session() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "test-agent", "admin").await;

        // Session manuell einfügen
        let session_id = "test-session-sse-123".to_string();
        let (tx, _) = broadcast::channel::<String>(10);
        state.mcp_sessions.lock().await.insert(
            session_id.clone(),
            McpSession {
                caller: "test-agent".to_string(),
                role: "admin".to_string(),
                created_at: chrono::Utc::now(),
                tx,
            },
        );

        let app = build_mcp_router(state);

        let req = Request::builder()
            .method(Method::GET)
            .uri("/mcp")
            .header("authorization", format!("Bearer {token}"))
            .header("mcp-session-id", &session_id)
            .header("accept", "text/event-stream")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "GET /mcp mit bekannter Session muss 200 zurückgeben"
        );
        // Content-Type muss text/event-stream sein
        let ct = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            ct.contains("text/event-stream"),
            "Content-Type muss text/event-stream sein, war: {ct}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 8: SSE-Stream GET /mcp – ohne Session-ID → 400
    // -----------------------------------------------------------------------

    /// GET /mcp ohne `mcp-session-id` Header → 400 Bad Request.
    #[tokio::test]
    async fn test_sse_stream_missing_session_id() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "test-agent", "admin").await;
        let app = build_mcp_router(state);

        let req = Request::builder()
            .method(Method::GET)
            .uri("/mcp")
            .header("authorization", format!("Bearer {token}"))
            .header("accept", "text/event-stream")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "GET /mcp ohne Session-ID muss 400 zurückgeben"
        );
    }

    // -----------------------------------------------------------------------
    // Test 9: SSE-Stream GET /mcp – unbekannte Session → 404
    // -----------------------------------------------------------------------

    /// GET /mcp mit unbekannter `mcp-session-id` → 404 Not Found.
    #[tokio::test]
    async fn test_sse_stream_unknown_session() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "test-agent", "admin").await;
        let app = build_mcp_router(state);

        let req = Request::builder()
            .method(Method::GET)
            .uri("/mcp")
            .header("authorization", format!("Bearer {token}"))
            .header("mcp-session-id", "does-not-exist-xyz")
            .header("accept", "text/event-stream")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "GET /mcp mit unbekannter Session muss 404 zurückgeben"
        );
    }

    // -----------------------------------------------------------------------
    // Test 10: DELETE /mcp – Session beenden
    // -----------------------------------------------------------------------

    /// DELETE /mcp mit bekannter Session → 200.
    #[tokio::test]
    async fn test_delete_session_known() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "test-agent", "admin").await;

        let session_id = "delete-test-session-456".to_string();
        let (tx, _) = broadcast::channel::<String>(10);
        state.mcp_sessions.lock().await.insert(
            session_id.clone(),
            McpSession {
                caller: "test-agent".to_string(),
                role: "admin".to_string(),
                created_at: chrono::Utc::now(),
                tx,
            },
        );

        let app = build_mcp_router(state);

        let req = Request::builder()
            .method(Method::DELETE)
            .uri("/mcp")
            .header("authorization", format!("Bearer {token}"))
            .header("mcp-session-id", &session_id)
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "DELETE bekannte Session muss 200 sein"
        );
    }

    /// DELETE /mcp mit unbekannter Session → 404.
    #[tokio::test]
    async fn test_delete_session_unknown() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "test-agent", "admin").await;
        let app = build_mcp_router(state);

        let req = Request::builder()
            .method(Method::DELETE)
            .uri("/mcp")
            .header("authorization", format!("Bearer {token}"))
            .header("mcp-session-id", "unknown-session-xyz")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "DELETE unbekannte Session muss 404 sein"
        );
    }

    // -----------------------------------------------------------------------
    // Test 11: initialize ohne Auth → 401 mit WWW-Authenticate
    // -----------------------------------------------------------------------

    /// POST /mcp ohne Token → 401 mit korrektem WWW-Authenticate Header.
    #[tokio::test]
    async fn test_initialize_without_auth_returns_401() {
        let (state, _dir) = make_test_state().await;
        let app = build_mcp_router(state);

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": { "protocolVersion": "2024-11-05", "capabilities": {} },
            "id": 1
        });

        let req = Request::builder()
            .method(Method::POST)
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "initialize ohne Auth muss 401 zurückgeben"
        );
        assert!(
            resp.headers().contains_key("www-authenticate"),
            "WWW-Authenticate Header muss bei 401 gesetzt sein"
        );
    }

    // -----------------------------------------------------------------------
    // Test 12: Notification liefert 202 Accepted (kein Body)
    // -----------------------------------------------------------------------

    /// `notifications/initialized` ohne `id` → 202 Accepted, kein Response-Body nötig.
    #[tokio::test]
    async fn test_notification_returns_202() {
        let (state, _dir) = make_test_state().await;
        let token = create_test_token(&state, "test-agent", "admin").await;
        let app = build_mcp_router(state.clone());

        // Erst initialize
        let init_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": { "protocolVersion": "2024-11-05", "capabilities": {} },
            "id": 1
        });
        let init_req = Request::builder()
            .method(Method::POST)
            .uri("/mcp")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(serde_json::to_vec(&init_body).unwrap()))
            .unwrap();
        let init_resp = app.clone().oneshot(init_req).await.unwrap();
        let session_id = init_resp
            .headers()
            .get("mcp-session-id")
            .and_then(|v| v.to_str().ok())
            .unwrap()
            .to_string();

        // Notification senden (kein id-Feld)
        let notif_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
            // Kein "id" → Notification
        });
        let notif_req = Request::builder()
            .method(Method::POST)
            .uri("/mcp")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .header("mcp-session-id", &session_id)
            .body(Body::from(serde_json::to_vec(&notif_body).unwrap()))
            .unwrap();

        let notif_resp = app.oneshot(notif_req).await.unwrap();
        assert_eq!(
            notif_resp.status(),
            StatusCode::ACCEPTED,
            "Notification muss 202 Accepted zurückgeben"
        );
    }
}
