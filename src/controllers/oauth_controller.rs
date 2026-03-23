// OAuth 2.0 Authorization Code Flow für externe Clients (z.B. claude.ai).

use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect},
    Json,
};
use chrono::Utc;

use crate::error::ApiError;
use crate::models::*;
use crate::services::*;
use crate::state::AppState;

/// GET /oauth/authorize – Authorization Endpoint.
/// Zeigt Login-Formular oder leitet mit Code zurück.
pub async fn oauth_authorize(
    State(state): State<AppState>,
    Query(params): Query<OAuthAuthorizeRequest>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    // Validierung
    if params.response_type != "code" {
        return Ok(Redirect::to(&format!(
            "{}?error=unsupported_response_type&state={}",
            params.redirect_uri, params.state
        ))
        .into_response());
    }

    // Client prüfen: registrierte Clients ODER Public Clients mit PKCE
    let clients = state.oauth_clients.lock().await;
    let registered_client = clients
        .iter()
        .find(|c| c.client_id == params.client_id && c.active)
        .cloned();
    drop(clients);

    let client_name = if let Some(ref c) = registered_client {
        // Registrierter Client: Redirect-URI muss übereinstimmen
        if !c.redirect_uris.iter().any(|u| u == &params.redirect_uri) {
            return Err(ApiError::BadRequest("Invalid redirect_uri".into()));
        }
        c.name.clone()
    } else {
        // Public Client (unregistriert): PKCE ist Pflicht
        if params.code_challenge.is_none() {
            return Ok(Redirect::to(&format!(
                "{}?error=invalid_request&error_description=PKCE+required+for+public+clients&state={}",
                params.redirect_uri, params.state
            ))
            .into_response());
        }
        // Client-Name aus der Redirect-URI ableiten
        params.redirect_uri
            .split("//")
            .nth(1)
            .and_then(|s| s.split('/').next())
            .unwrap_or(&params.client_id)
            .to_string()
    };

    // Prüfe ob User schon eingeloggt (Cookie)
    if let Some(t) = extract_token_from_headers(&headers) {
        if let Ok(claims) = validate_jwt(&t, &state.jwt_secret) {
            // User ist eingeloggt → Code generieren und zurückleiten
            let code = generate_oauth_code();
            let auth_code = OAuthAuthCode {
                code: code.clone(),
                client_id: params.client_id.clone(),
                user_id: claims.sub.clone(),
                redirect_uri: params.redirect_uri.clone(),
                scope: params.scope.clone(),
                created_at: Utc::now(),
                code_challenge: params.code_challenge.clone(),
            };
            state
                .oauth_codes
                .lock()
                .await
                .insert(code.clone(), auth_code);

            let redirect = format!(
                "{}?code={}&state={}",
                params.redirect_uri, code, params.state
            );
            return Ok(Redirect::to(&redirect).into_response());
        }
    }

    // Nicht eingeloggt → Login-Seite mit Consent anzeigen
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");
    let host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost");
    let base_url = format!("{scheme}://{host}");

    let html = format!(
        r##"<!DOCTYPE html>
<html lang="de">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Plankton – Autorisierung</title>
<style>
  :root {{
    --bg: #1a1a2e; --surface: #16213e; --input-bg: #0f3460;
    --text: #e0e0e0; --text-dim: #aaa; --border: #333;
    --accent: #64ffda; --accent-text: #1a1a2e;
    --shadow: rgba(0,0,0,0.3);
    color-scheme: dark;
  }}
  @media (prefers-color-scheme: light) {{
    :root {{
      --bg: #f5f5f7; --surface: #ffffff; --input-bg: #eeeef2;
      --text: #1a1a2e; --text-dim: #6e6e82; --border: #d0d0d8;
      --accent: #6b5ce7; --accent-text: #ffffff;
      --shadow: rgba(0,0,0,0.08);
      color-scheme: light;
    }}
  }}
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: var(--bg); color: var(--text);
    display: flex; justify-content: center; align-items: center;
    min-height: 100vh;
  }}
  .card {{
    background: var(--surface); border-radius: 12px; padding: 40px;
    max-width: 420px; width: 100%; box-shadow: 0 8px 32px var(--shadow);
  }}
  h1 {{ font-size: 24px; margin-bottom: 8px; }}
  .subtitle {{ color: var(--text-dim); margin-bottom: 24px; }}
  .client-name {{ color: var(--accent); font-weight: 600; }}
  label {{ display: block; margin-bottom: 4px; font-size: 14px; color: var(--text-dim); }}
  input {{ width: 100%; padding: 10px 12px; border: 1px solid var(--border);
    border-radius: 6px; background: var(--input-bg); color: var(--text);
    font-size: 14px; margin-bottom: 12px; outline: none; }}
  input:focus {{ border-color: var(--accent); }}
  button {{
    width: 100%; padding: 12px; border: none; border-radius: 6px;
    background: var(--accent); color: var(--accent-text); font-size: 16px;
    font-weight: 600; cursor: pointer; transition: opacity 0.2s;
  }}
  button:hover {{ opacity: 0.9; }}
  button:disabled {{ opacity: 0.5; cursor: default; }}
  .msg {{ text-align: center; padding: 12px; border-radius: 6px;
    margin-top: 16px; font-size: 14px; background: rgba(255,82,82,0.1); color: #ff5252; }}
</style>
</head>
<body>
<div class="card">
  <h1><img src="/icons/favicon-32.png" alt="" style="vertical-align:middle;margin-right:8px" />Autorisierung</h1>
  <p class="subtitle"><span class="client-name">{client_name}</span> m&ouml;chte auf Plankton zugreifen.</p>
  <form id="auth-form">
    <label for="username">Username</label>
    <input id="username" type="text" autocomplete="username" required autofocus>
    <label for="password">Passwort</label>
    <input id="password" type="password" autocomplete="current-password" required>
    <button type="submit">Anmelden &amp; Autorisieren</button>
  </form>
  <div id="error-msg" style="display:none" class="msg"></div>
</div>
<script>
document.getElementById('auth-form').addEventListener('submit', async (e) => {{
  e.preventDefault();
  const btn = e.target.querySelector('button');
  btn.disabled = true;
  const msg = document.getElementById('error-msg');
  msg.style.display = 'none';
  try {{
    const r = await fetch('{base_url}/auth/login', {{
      method: 'POST',
      headers: {{ 'Content-Type': 'application/json' }},
      credentials: 'include',
      body: JSON.stringify({{
        username: document.getElementById('username').value,
        password: document.getElementById('password').value,
      }}),
    }});
    if (!r.ok) {{
      const err = await r.json().catch(() => ({{}}));
      throw new Error(err.error || 'Login fehlgeschlagen');
    }}
    // Erneut authorize aufrufen – jetzt mit Cookie
    window.location.href = '{base_url}/authorize?{query}';
  }} catch (err) {{
    msg.textContent = err.message;
    msg.style.display = '';
    btn.disabled = false;
  }}
}});
</script>
</body></html>"##,
        client_name = html_escape(&client_name),
        base_url = base_url,
        query = build_authorize_query(&params),
    );

    Ok(Html(html).into_response())
}

/// POST /token – Token Endpoint (akzeptiert form-urlencoded und JSON).
pub async fn oauth_token(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let params: OAuthTokenRequest = if content_type.contains("json") {
        serde_json::from_slice(&body)
            .map_err(|e| ApiError::BadRequest(format!("Invalid JSON: {e}")))?
    } else {
        serde_urlencoded::from_bytes(&body)
            .map_err(|e| ApiError::BadRequest(format!("Invalid form data: {e}")))?
    };
    match params.grant_type.as_str() {
        "authorization_code" => {
            let code_str = params
                .code
                .as_deref()
                .ok_or(ApiError::BadRequest("Missing code".into()))?;

            // Code einlösen (einmalig)
            let auth_code = state
                .oauth_codes
                .lock()
                .await
                .remove(code_str)
                .ok_or(ApiError::BadRequest("Invalid or expired code".into()))?;

            // Code ist max 5 Minuten gültig
            let age = Utc::now() - auth_code.created_at;
            if age.num_seconds() > 300 {
                return Err(ApiError::BadRequest("Code expired".into()));
            }

            // Client validieren
            let client_id = params
                .client_id
                .as_deref()
                .unwrap_or("");
            if client_id != auth_code.client_id {
                return Err(ApiError::BadRequest("Client mismatch".into()));
            }

            // Redirect-URI muss übereinstimmen
            if let Some(ref uri) = params.redirect_uri {
                if uri != &auth_code.redirect_uri {
                    return Err(ApiError::BadRequest("Redirect URI mismatch".into()));
                }
            }

            // PKCE Verification
            if let Some(ref challenge) = auth_code.code_challenge {
                let verifier = params
                    .code_verifier
                    .as_deref()
                    .ok_or(ApiError::BadRequest("Missing code_verifier".into()))?;
                let computed = base64url_sha256(verifier);
                if &computed != challenge {
                    return Err(ApiError::BadRequest("Invalid code_verifier".into()));
                }
            }

            // User laden und Access Token erstellen
            let user = state.store.get_user(&auth_code.user_id).await?;
            let access_token =
                create_jwt_with_duration(&user, &state.jwt_secret, false, chrono::Duration::hours(1))?;

            // Refresh Token generieren
            let refresh_token_str = generate_oauth_code();
            let refresh = OAuthRefreshToken {
                token: refresh_token_str.clone(),
                client_id: auth_code.client_id.clone(),
                user_id: auth_code.user_id.clone(),
                scope: auth_code.scope.clone(),
                created_at: Utc::now(),
                active: true,
            };
            state
                .oauth_refresh_tokens
                .lock()
                .await
                .insert(refresh_token_str.clone(), refresh);

            Ok(Json(serde_json::json!({
                "access_token": access_token,
                "token_type": "Bearer",
                "expires_in": 3600,
                "refresh_token": refresh_token_str,
                "scope": auth_code.scope,
            })))
        }
        "refresh_token" => {
            let refresh_str = params
                .refresh_token
                .as_deref()
                .ok_or(ApiError::BadRequest("Missing refresh_token".into()))?;

            // Alten Refresh Token einlösen + rotieren
            let old_refresh = state
                .oauth_refresh_tokens
                .lock()
                .await
                .remove(refresh_str)
                .ok_or(ApiError::BadRequest("Invalid refresh token".into()))?;

            if !old_refresh.active {
                return Err(ApiError::BadRequest("Refresh token revoked".into()));
            }

            // User laden und neuen Access Token erstellen
            let user = state.store.get_user(&old_refresh.user_id).await?;
            let access_token =
                create_jwt_with_duration(&user, &state.jwt_secret, false, chrono::Duration::hours(1))?;

            // Neuen Refresh Token (Rotation)
            let new_refresh_str = generate_oauth_code();
            let new_refresh = OAuthRefreshToken {
                token: new_refresh_str.clone(),
                client_id: old_refresh.client_id.clone(),
                user_id: old_refresh.user_id.clone(),
                scope: old_refresh.scope.clone(),
                created_at: Utc::now(),
                active: true,
            };
            state
                .oauth_refresh_tokens
                .lock()
                .await
                .insert(new_refresh_str.clone(), new_refresh);

            Ok(Json(serde_json::json!({
                "access_token": access_token,
                "token_type": "Bearer",
                "expires_in": 3600,
                "refresh_token": new_refresh_str,
                "scope": old_refresh.scope,
            })))
        }
        _ => Err(ApiError::BadRequest("Unsupported grant_type".into())),
    }
}

/// GET /oauth/metadata – OAuth 2.0 Server Metadata (RFC 8414).
pub async fn oauth_metadata(
    axum::extract::Host(host): axum::extract::Host,
    headers: axum::http::HeaderMap,
) -> Json<serde_json::Value> {
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");
    let issuer = format!("{scheme}://{host}");

    Json(serde_json::json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/authorize"),
        "token_endpoint": format!("{issuer}/token"),
        "registration_endpoint": format!("{issuer}/register"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["client_secret_post", "none"],
    }))
}

/// POST /register – Dynamic Client Registration (RFC 7591).
pub async fn oauth_register(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), ApiError> {
    let client_name = payload["client_name"]
        .as_str()
        .unwrap_or("Unknown Client")
        .to_string();
    let redirect_uris: Vec<String> = payload["redirect_uris"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    if redirect_uris.is_empty() {
        return Err(ApiError::BadRequest("redirect_uris required".into()));
    }

    let client_id = generate_oauth_code();
    let client_secret = generate_oauth_code();

    let client = OAuthClient {
        client_id: client_id.clone(),
        client_secret: client_secret.clone(),
        name: client_name.clone(),
        redirect_uris: redirect_uris.clone(),
        active: true,
        created_at: Utc::now().to_rfc3339(),
    };

    state.oauth_clients.lock().await.push(client);

    Ok((
        axum::http::StatusCode::CREATED,
        Json(serde_json::json!({
            "client_id": client_id,
            "client_secret": client_secret,
            "client_name": client_name,
            "redirect_uris": redirect_uris,
            "token_endpoint_auth_method": "client_secret_post",
        })),
    ))
}

/// GET /api/admin/oauth-clients – OAuth Clients auflisten.
pub async fn admin_list_oauth_clients(
    State(state): State<AppState>,
) -> Json<Vec<serde_json::Value>> {
    let clients = state.oauth_clients.lock().await;
    Json(
        clients
            .iter()
            .map(|c| {
                serde_json::json!({
                    "client_id": c.client_id,
                    "name": c.name,
                    "redirect_uris": c.redirect_uris,
                    "active": c.active,
                    "created_at": c.created_at,
                })
            })
            .collect(),
    )
}

/// POST /api/admin/oauth-clients – Neuen OAuth Client erstellen.
pub async fn admin_create_oauth_client(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let name = payload["name"]
        .as_str()
        .ok_or(ApiError::BadRequest("Missing name".into()))?
        .to_string();
    let redirect_uris: Vec<String> = payload["redirect_uris"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let client_id = generate_oauth_code();
    let client_secret = generate_oauth_code();

    let client = OAuthClient {
        client_id: client_id.clone(),
        client_secret: client_secret.clone(),
        name,
        redirect_uris,
        active: true,
        created_at: Utc::now().to_rfc3339(),
    };

    state.oauth_clients.lock().await.push(client.clone());

    // Secret nur einmalig anzeigen
    Ok(Json(serde_json::json!({
        "client_id": client_id,
        "client_secret": client_secret,
        "name": client.name,
        "redirect_uris": client.redirect_uris,
    })))
}

// ─── Hilfsfunktionen ─────────────────────────────────────────

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn build_authorize_query(params: &OAuthAuthorizeRequest) -> String {
    let mut q = format!(
        "response_type={}&client_id={}&redirect_uri={}&state={}",
        urlencoding::encode(&params.response_type),
        urlencoding::encode(&params.client_id),
        urlencoding::encode(&params.redirect_uri),
        urlencoding::encode(&params.state),
    );
    if !params.scope.is_empty() {
        q.push_str(&format!("&scope={}", urlencoding::encode(&params.scope)));
    }
    if let Some(ref cc) = params.code_challenge {
        q.push_str(&format!("&code_challenge={}", urlencoding::encode(cc)));
        if let Some(ref method) = params.code_challenge_method {
            q.push_str(&format!(
                "&code_challenge_method={}",
                urlencoding::encode(method)
            ));
        }
    }
    q
}

fn base64url_sha256(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(input.as_bytes());
    base64url_encode(&hash)
}

fn base64url_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}
