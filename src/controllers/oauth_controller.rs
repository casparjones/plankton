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
    let logged_in_user = extract_token_from_headers(&headers)
        .and_then(|t| validate_jwt(&t, &state.jwt_secret).ok());

    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");
    let host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost");
    let base_url = format!("{scheme}://{host}");

    // Consent wurde bestätigt (POST von der Consent-Seite)
    // → wird via query param "consent=granted" signalisiert
    if let Some(ref user) = logged_in_user {
        if params.state.ends_with("_consent") {
            let real_state = params.state.trim_end_matches("_consent");
            let code = generate_oauth_code();
            let auth_code = OAuthAuthCode {
                code: code.clone(),
                client_id: params.client_id.clone(),
                user_id: user.sub.clone(),
                redirect_uri: params.redirect_uri.clone(),
                scope: params.scope.clone(),
                created_at: Utc::now(),
                code_challenge: params.code_challenge.clone(),
            };
            state.oauth_codes.lock().await.insert(code.clone(), auth_code);
            let redirect = format!("{}?code={}&state={}", params.redirect_uri, code, real_state);
            return Ok(Redirect::to(&redirect).into_response());
        }
    }

    // Consent-Screen anzeigen (eingeloggt oder nicht)
    let show_login = logged_in_user.is_none();
    let user_display = logged_in_user
        .as_ref()
        .map(|u| format!("{} ({})", u.display_name, u.role))
        .unwrap_or_default();

    let original_state = params.state.clone();
    let redirect_uri_raw = params.redirect_uri.clone();
    let consent_state = format!("{}_consent", params.state);
    let consent_query = {
        let mut p = params;
        p.state = consent_state;
        build_authorize_query(&p)
    };

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
    --shadow: rgba(0,0,0,0.3); --danger: #ff5252;
    color-scheme: dark;
  }}
  @media (prefers-color-scheme: light) {{
    :root {{
      --bg: #f5f5f7; --surface: #ffffff; --input-bg: #eeeef2;
      --text: #1a1a2e; --text-dim: #6e6e82; --border: #d0d0d8;
      --accent: #6b5ce7; --accent-text: #ffffff;
      --shadow: rgba(0,0,0,0.08); --danger: #d94452;
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
  .subtitle {{ color: var(--text-dim); margin-bottom: 20px; }}
  .client-name {{ color: var(--accent); font-weight: 600; }}
  .user-info {{ background: var(--input-bg); border: 1px solid var(--border);
    border-radius: 6px; padding: 10px 14px; margin-bottom: 16px; font-size: 13px; }}
  .user-label {{ color: var(--text-dim); font-size: 11px; text-transform: uppercase;
    letter-spacing: 0.05em; margin-bottom: 2px; }}
  .permissions {{ margin: 16px 0; font-size: 13px; color: var(--text-dim); }}
  .permissions li {{ margin: 4px 0; padding-left: 4px; }}
  label {{ display: block; margin-bottom: 4px; font-size: 14px; color: var(--text-dim); }}
  input {{ width: 100%; padding: 10px 12px; border: 1px solid var(--border);
    border-radius: 6px; background: var(--input-bg); color: var(--text);
    font-size: 14px; margin-bottom: 12px; outline: none; }}
  input:focus {{ border-color: var(--accent); }}
  .btn-row {{ display: flex; gap: 10px; }}
  .btn-row button {{ flex: 1; }}
  button {{
    padding: 12px; border: none; border-radius: 6px;
    font-size: 15px; font-weight: 600; cursor: pointer; transition: opacity 0.2s;
  }}
  .btn-allow {{ background: var(--accent); color: var(--accent-text); }}
  .btn-deny {{ background: var(--input-bg); color: var(--text-dim); border: 1px solid var(--border); }}
  button:hover {{ opacity: 0.9; }}
  button:disabled {{ opacity: 0.5; cursor: default; }}
  .msg {{ text-align: center; padding: 12px; border-radius: 6px;
    margin-top: 16px; font-size: 14px; background: rgba(255,82,82,0.1); color: var(--danger); }}
  .hidden {{ display: none; }}
</style>
</head>
<body>
<div class="card">
  <h1><img src="/icons/favicon-32.png" alt="" style="vertical-align:middle;margin-right:8px" />Autorisierung</h1>
  <p class="subtitle"><span class="client-name">{client_name}</span> m&ouml;chte auf dein Plankton-Konto zugreifen.</p>

  <!-- Login-Formular (nur wenn nicht eingeloggt) -->
  <div id="step-login" class="{login_class}">
    <form id="login-form">
      <label for="username">Username</label>
      <input id="username" type="text" autocomplete="username" required autofocus>
      <label for="password">Passwort</label>
      <input id="password" type="password" autocomplete="current-password" required>
      <button type="submit" class="btn-allow" style="width:100%">Anmelden</button>
    </form>
    <div id="login-msg"></div>
  </div>

  <!-- Consent-Screen -->
  <div id="step-consent" class="{consent_class}">
    <div class="user-info">
      <div class="user-label">Eingeloggt als</div>
      <div id="user-display">{user_display}</div>
    </div>
    <ul class="permissions">
      <li>Projekte und Tasks lesen</li>
      <li>Tasks erstellen, bearbeiten und verschieben</li>
      <li>Kommentare und Logs schreiben</li>
    </ul>
    <div class="btn-row">
      <button class="btn-deny" id="deny-btn">Ablehnen</button>
      <button class="btn-allow" id="allow-btn">Zugriff erlauben</button>
    </div>
  </div>

  <!-- Abgelehnt -->
  <div id="step-denied" class="hidden">
    <div class="msg">Zugriff verweigert. Du kannst dieses Fenster schlie&szlig;en.</div>
  </div>
</div>
<script>
(function() {{
  const consentUrl = '{base_url}/oauth/authorize?{consent_query}';
  const redirectUri = '{redirect_uri}';
  const state = '{original_state}';

  // Login-Formular
  const loginForm = document.getElementById('login-form');
  if (loginForm) {{
    loginForm.addEventListener('submit', async (e) => {{
      e.preventDefault();
      const btn = e.target.querySelector('button');
      btn.disabled = true;
      const msg = document.getElementById('login-msg');
      msg.innerHTML = '';
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
        const data = await r.json();
        // Login OK → Consent anzeigen
        document.getElementById('step-login').classList.add('hidden');
        document.getElementById('step-consent').classList.remove('hidden');
        document.getElementById('user-display').textContent =
          (data.display_name || data.username) + ' (' + (data.role || 'user') + ')';
      }} catch (err) {{
        msg.innerHTML = '<div class="msg">' + err.message + '</div>';
        btn.disabled = false;
      }}
    }});
  }}

  // Consent: Erlauben
  document.getElementById('allow-btn').addEventListener('click', () => {{
    window.location.href = consentUrl;
  }});

  // Consent: Ablehnen
  document.getElementById('deny-btn').addEventListener('click', () => {{
    document.getElementById('step-consent').classList.add('hidden');
    document.getElementById('step-denied').classList.remove('hidden');
    // Redirect mit error=access_denied
    setTimeout(() => {{
      window.location.href = redirectUri + '?error=access_denied&state=' + state;
    }}, 1500);
  }});
}})();
</script>
</body></html>"##,
        client_name = html_escape(&client_name),
        base_url = base_url,
        consent_query = consent_query,
        redirect_uri = html_escape(&redirect_uri_raw),
        original_state = html_escape(&original_state),
        login_class = if show_login { "" } else { "hidden" },
        consent_class = if show_login { "hidden" } else { "" },
        user_display = html_escape(&user_display),
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

            // PKCE Verification (Pflicht für public clients mit auth_method "none")
            if let Some(ref challenge) = auth_code.code_challenge {
                let verifier = params
                    .code_verifier
                    .as_deref()
                    .ok_or(ApiError::BadRequest("Missing code_verifier".into()))?;
                let computed = base64url_sha256(verifier);
                if &computed != challenge {
                    return Err(ApiError::BadRequest("Invalid code_verifier".into()));
                }
            } else {
                // Prüfe ob der Client PKCE erfordert (auth_method "none")
                let clients = state.oauth_clients.lock().await;
                let is_public = clients.iter()
                    .find(|c| c.client_id == auth_code.client_id)
                    .map(|c| c.auth_method == "none")
                    .unwrap_or(false);
                if is_public {
                    return Err(ApiError::BadRequest("PKCE required for public clients".into()));
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

/// GET /.well-known/oauth-protected-resource – Protected Resource Metadata (RFC 9728).
pub async fn oauth_protected_resource(
    axum::extract::Host(host): axum::extract::Host,
    headers: axum::http::HeaderMap,
) -> Json<serde_json::Value> {
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");
    let resource = format!("{scheme}://{host}");

    Json(serde_json::json!({
        "resource": resource,
        "authorization_servers": [resource],
        "scopes_supported": ["default"],
        "bearer_methods_supported": ["header"],
        "resource_name": "Plankton MCP",
    }))
}

/// GET /.well-known/oauth-authorization-server – OAuth 2.0 Server Metadata (RFC 8414).
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
        "authorization_endpoint": format!("{issuer}/oauth/authorize"),
        "token_endpoint": format!("{issuer}/oauth/token"),
        "registration_endpoint": format!("{issuer}/oauth/register"),
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

    // token_endpoint_auth_method aus dem Request lesen (default: "client_secret_post")
    let auth_method = payload["token_endpoint_auth_method"]
        .as_str()
        .unwrap_or("client_secret_post")
        .to_string();

    let client_id = generate_oauth_code();
    // Nur bei "client_secret_post" ein Secret generieren
    let client_secret = if auth_method == "none" {
        String::new()
    } else {
        generate_oauth_code()
    };

    let client = OAuthClient {
        client_id: client_id.clone(),
        client_secret: client_secret.clone(),
        name: client_name.clone(),
        redirect_uris: redirect_uris.clone(),
        auth_method: auth_method.clone(),
        active: true,
        created_at: Utc::now().to_rfc3339(),
    };

    state.oauth_clients.lock().await.push(client);

    // Response: kein client_secret bei auth_method "none"
    let mut resp = serde_json::json!({
        "client_id": client_id,
        "client_name": client_name,
        "redirect_uris": redirect_uris,
        "token_endpoint_auth_method": auth_method,
    });
    if auth_method != "none" {
        resp["client_secret"] = serde_json::Value::String(client_secret);
    }

    Ok((axum::http::StatusCode::CREATED, Json(resp)))
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
        auth_method: "client_secret_post".to_string(),
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
