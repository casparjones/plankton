// Handler für CLI-Installation, Device-Auth-Flow und CLI-Login-Seite.

use axum::{
    extract::{Path, State},
    response::Html,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::*;
use crate::services::*;
use crate::state::AppState;

/// POST /auth/cli-init – Startet eine CLI-Login-Session (Device Flow).
pub async fn cli_init(
    State(state): State<AppState>,
    axum::extract::Host(host): axum::extract::Host,
    headers: axum::http::HeaderMap,
) -> Json<serde_json::Value> {
    let session_id = Uuid::new_v4().to_string();

    // 6-stelliger Code zur Anzeige im Terminal.
    let code: String = {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..6)
            .map(|_| {
                let idx = rng.gen_range(0..36);
                if idx < 10 {
                    (b'0' + idx) as char
                } else {
                    (b'A' + idx - 10) as char
                }
            })
            .collect()
    };

    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");
    let login_url = format!("{scheme}://{host}/cli-login?session={session_id}");

    let session = CliSession {
        session_id: session_id.clone(),
        code: code.clone(),
        status: CliSessionStatus::Pending,
        token: None,
        created_at: Utc::now(),
    };

    state
        .cli_sessions
        .lock()
        .await
        .insert(session_id.clone(), session);

    Json(serde_json::json!({
        "session_id": session_id,
        "code": code,
        "login_url": login_url,
    }))
}

/// GET /auth/cli-poll/:session_id – Pollt den Status einer CLI-Session.
pub async fn cli_poll(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut sessions = state.cli_sessions.lock().await;
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| ApiError::NotFound("Session not found".into()))?;

    // Abgelaufen nach 5 Minuten.
    let age = Utc::now() - session.created_at;
    if age.num_seconds() > 300 {
        sessions.remove(&session_id);
        return Ok(Json(serde_json::json!({ "status": "expired" })));
    }

    match session.status {
        CliSessionStatus::Pending => Ok(Json(serde_json::json!({
            "status": "pending",
            "code": session.code,
        }))),
        CliSessionStatus::Approved => {
            let token = session.token.clone();
            sessions.remove(&session_id);
            Ok(Json(serde_json::json!({
                "status": "approved",
                "token": token,
            })))
        }
        CliSessionStatus::Expired => {
            sessions.remove(&session_id);
            Ok(Json(serde_json::json!({ "status": "expired" })))
        }
    }
}

/// POST /auth/cli-approve – Genehmigt eine CLI-Session (aufgerufen vom Browser).
pub async fn cli_approve(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<CliApproveRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // JWT des eingeloggten Users validieren.
    let jwt = extract_token_from_headers(&headers)
        .ok_or(ApiError::Unauthorized("Not authenticated".into()))?;
    let claims = validate_jwt(&jwt, &state.jwt_secret)?;

    // User laden und langlebigen CLI-Token erstellen (30 Tage).
    let user = state.store.get_user(&claims.sub).await?;
    let cli_token =
        create_jwt_with_duration(&user, &state.jwt_secret, false, chrono::Duration::days(30))?;

    // Session aktualisieren.
    let mut sessions = state.cli_sessions.lock().await;
    let session = sessions
        .get_mut(&payload.session_id)
        .ok_or_else(|| ApiError::NotFound("Session not found or expired".into()))?;

    if session.status != CliSessionStatus::Pending {
        return Err(ApiError::BadRequest("Session already processed".into()));
    }

    session.status = CliSessionStatus::Approved;
    session.token = Some(cli_token);

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /install – Installer-Script für `curl -fsSL .../install | bash`.
pub async fn serve_installer(
    axum::extract::Host(host): axum::extract::Host,
    headers: axum::http::HeaderMap,
) -> impl axum::response::IntoResponse {
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");
    let base_url = format!("{scheme}://{host}");

    let script = format!(
        r##"#!/bin/bash
set -e

PLANKTON_URL="{base_url}"
INSTALL_DIR="${{HOME}}/.local/bin"

echo ""
echo "  🪼 Plankton CLI Installer"
echo "  ━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Abhängigkeiten prüfen.
for cmd in curl jq; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "  ✗ $cmd is required but not installed."
        exit 1
    fi
done

# Installationsverzeichnis erstellen.
mkdir -p "$INSTALL_DIR"

# CLI herunterladen.
echo "  ↓ Downloading plankton CLI..."
curl -fsSL "${{PLANKTON_URL}}/cli/plankton" -o "${{INSTALL_DIR}}/plankton"
chmod +x "${{INSTALL_DIR}}/plankton"

echo "  ✓ Installed to ${{INSTALL_DIR}}/plankton"
echo ""

# PATH prüfen.
if [[ ":$PATH:" != *":${{INSTALL_DIR}}:"* ]]; then
    echo "  ⚠ ${{INSTALL_DIR}} is not in your PATH."
    echo "  Add this to your shell config:"
    echo ""
    echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
fi

echo "  Get started:"
echo ""
echo "    plankton login ${{PLANKTON_URL}}"
echo ""
"##,
        base_url = base_url,
    );

    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        script,
    )
}

/// GET /cli/plankton – Das CLI-Script selbst.
pub async fn serve_cli_script(
    axum::extract::Host(host): axum::extract::Host,
    headers: axum::http::HeaderMap,
) -> impl axum::response::IntoResponse {
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");
    let default_url = format!("{scheme}://{host}");

    let script = format!(
        r##"#!/bin/bash
# Plankton CLI – Kanban-Board für KI-Agenten
# Installiert via: curl -fsSL <server>/install | bash

set -e

VERSION="0.1.0"
CONFIG_DIR="${{HOME}}/.config/plankton"
CONFIG_FILE="${{CONFIG_DIR}}/config"
DEFAULT_SERVER="{default_url}"

# ─── Konfiguration ──────────────────────────────────────────

load_config() {{
    PLANKTON_SERVER=""
    PLANKTON_TOKEN=""
    if [[ -f "$CONFIG_FILE" ]]; then
        source "$CONFIG_FILE"
    fi
}}

save_config() {{
    mkdir -p "$CONFIG_DIR"
    cat > "$CONFIG_FILE" <<CONF
PLANKTON_SERVER=$PLANKTON_SERVER
PLANKTON_TOKEN=$PLANKTON_TOKEN
CONF
    chmod 600 "$CONFIG_FILE"
}}

need_auth() {{
    load_config
    if [[ -z "$PLANKTON_SERVER" || -z "$PLANKTON_TOKEN" ]]; then
        echo "Not logged in. Run: plankton login <url>"
        exit 1
    fi
}}

api() {{
    local method="$1" path="$2" data="$3"
    local args=(-fsSL -X "$method" -H "Content-Type: application/json" -H "Authorization: Bearer $PLANKTON_TOKEN")
    if [[ -n "$data" ]]; then
        args+=(-d "$data")
    fi
    curl "${{args[@]}}" "${{PLANKTON_SERVER}}${{path}}"
}}

# ─── Login (Device Flow) ────────────────────────────────────

cmd_login() {{
    local server="${{1:-$DEFAULT_SERVER}}"
    server="${{server%/}}"

    echo ""
    echo "  🪼 Plankton Login"
    echo "  ━━━━━━━━━━━━━━━━━"
    echo ""

    # Session starten.
    local resp
    resp=$(curl -fsSL -X POST -H "Content-Type: application/json" "${{server}}/auth/cli-init")
    local session_id code login_url
    session_id=$(echo "$resp" | jq -r '.session_id')
    code=$(echo "$resp" | jq -r '.code')
    login_url=$(echo "$resp" | jq -r '.login_url')

    echo "  Open this URL in your browser:"
    echo ""
    echo "    $login_url"
    echo ""
    echo "  Verification code: $code"
    echo ""
    echo "  Waiting for approval..."

    # Polling (max 5 Minuten).
    local status=""
    for i in $(seq 1 150); do
        sleep 2
        resp=$(curl -fsSL "${{server}}/auth/cli-poll/${{session_id}}" 2>/dev/null || echo '{{"status":"error"}}')
        status=$(echo "$resp" | jq -r '.status')

        if [[ "$status" == "approved" ]]; then
            local token
            token=$(echo "$resp" | jq -r '.token')
            PLANKTON_SERVER="$server"
            PLANKTON_TOKEN="$token"
            save_config
            echo "  ✓ Login successful!"
            echo ""

            # User-Info anzeigen.
            local me
            me=$(api GET /auth/me 2>/dev/null || echo '{{}}')
            local name role
            name=$(echo "$me" | jq -r '.display_name // .username // "unknown"')
            role=$(echo "$me" | jq -r '.role // "unknown"')
            echo "  Logged in as: $name ($role)"
            echo "  Server: $PLANKTON_SERVER"
            echo ""
            return 0
        fi

        if [[ "$status" == "expired" || "$status" == "error" ]]; then
            echo "  ✗ Login failed or session expired."
            return 1
        fi
    done

    echo "  ✗ Timeout – session expired."
    return 1
}}

# ─── Skill Install / Update ─────────────────────────────────

cmd_skill_install() {{
    need_auth
    local global=false
    local target_dir=".claude/skills/plankton"

    for arg in "$@"; do
        case "$arg" in
            --global|-g) global=true ;;
        esac
    done

    if $global; then
        target_dir="${{HOME}}/.claude/skills/plankton"
    fi

    mkdir -p "$target_dir"

    echo ""
    echo "  ↓ Downloading SKILL.md from $PLANKTON_SERVER ..."
    curl -fsSL "${{PLANKTON_SERVER}}/skill.md" -o "${{target_dir}}/SKILL.md"
    echo "  ✓ Installed to ${{target_dir}}/SKILL.md"
    echo ""

    # Secrets-Datei prüfen.
    local secrets_found=false
    if [[ -f "${{HOME}}/.claude/plankton-secrets.md" ]]; then
        secrets_found=true
    elif [[ -f ".claude/plankton-secrets.md" ]]; then
        secrets_found=true
    fi

    if ! $secrets_found; then
        echo "  ⚠ No plankton-secrets.md found."
        echo "  Create one at ~/.claude/plankton-secrets.md"
        echo "  You can generate it in the Plankton UI:"
        echo "  Project Menu → Prompts → Claude Code Skill → Secrets"
        echo ""
    fi
}}

cmd_skill_update() {{
    cmd_skill_install "$@"
}}

# ─── Tokens ──────────────────────────────────────────────────

cmd_tokens() {{
    need_auth
    local resp
    resp=$(api GET /api/admin/tokens 2>/dev/null)
    if [[ $? -ne 0 ]]; then
        echo "  ✗ Failed to list tokens (admin required)."
        return 1
    fi

    echo ""
    echo "  Agent Tokens"
    echo "  ━━━━━━━━━━━━"
    echo ""
    echo "$resp" | jq -r '.[] | "  \(.name)\t\(.role)\t\(if .active then "active" else "inactive" end)\t\(.token)"' | column -t -s $'\t'
    echo ""
}}

# ─── Status ──────────────────────────────────────────────────

cmd_status() {{
    load_config

    echo ""
    echo "  🪼 Plankton CLI v$VERSION"
    echo "  ━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""

    if [[ -z "$PLANKTON_SERVER" ]]; then
        echo "  Not logged in."
        echo "  Run: plankton login <url>"
        echo ""
        return
    fi

    echo "  Server: $PLANKTON_SERVER"

    local me
    me=$(api GET /auth/me 2>/dev/null)
    if [[ $? -eq 0 && -n "$me" ]]; then
        local name role
        name=$(echo "$me" | jq -r '.display_name // .username // "unknown"')
        role=$(echo "$me" | jq -r '.role // "unknown"')
        echo "  User:   $name ($role)"
        echo "  Status: ✓ authenticated"
    else
        echo "  Status: ✗ token expired or invalid"
        echo "  Run: plankton login $PLANKTON_SERVER"
    fi
    echo ""
}}

# ─── Logout ──────────────────────────────────────────────────

cmd_logout() {{
    load_config
    PLANKTON_SERVER=""
    PLANKTON_TOKEN=""
    save_config
    echo ""
    echo "  ✓ Logged out."
    echo ""
}}

# ─── Help ────────────────────────────────────────────────────

cmd_help() {{
    echo ""
    echo "  🪼 Plankton CLI v$VERSION"
    echo "  ━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "  Usage: plankton <command> [options]"
    echo ""
    echo "  Commands:"
    echo "    login <url>          Login to a Plankton server (device flow)"
    echo "    logout               Clear stored credentials"
    echo "    status               Show connection info"
    echo "    skill install [-g]   Download & install SKILL.md"
    echo "    skill update  [-g]   Update installed SKILL.md"
    echo "    tokens               List agent tokens (admin)"
    echo "    help                 Show this help"
    echo ""
    echo "  Options:"
    echo "    -g, --global         Install skill to ~/.claude/ (default: .claude/)"
    echo ""
    echo "  Install:"
    echo "    curl -fsSL $DEFAULT_SERVER/install | bash"
    echo ""
}}

# ─── Main ────────────────────────────────────────────────────

case "${{1:-help}}" in
    login)   shift; cmd_login "$@" ;;
    logout)  cmd_logout ;;
    status)  cmd_status ;;
    skill)
        shift
        case "${{1:-install}}" in
            install) shift; cmd_skill_install "$@" ;;
            update)  shift; cmd_skill_update "$@" ;;
            *)       echo "Unknown skill command: $1"; cmd_help ;;
        esac
        ;;
    tokens)  cmd_tokens ;;
    help|--help|-h) cmd_help ;;
    *)       echo "Unknown command: $1"; cmd_help ;;
esac
"##,
        default_url = default_url,
    );

    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        script,
    )
}

/// GET /cli-login – Browser-Seite für den Device-Auth-Flow.
pub async fn cli_login_page(
    State(state): State<AppState>,
    axum::extract::Host(host): axum::extract::Host,
    headers: axum::http::HeaderMap,
) -> Html<String> {
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");
    let base_url = format!("{scheme}://{host}");

    let _ = &state;

    Html(format!(
        r##"<!DOCTYPE html>
<html lang="de">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Plankton – CLI Login</title>
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: #1a1a2e; color: #e0e0e0;
    display: flex; justify-content: center; align-items: center;
    min-height: 100vh;
  }}
  .card {{
    background: #16213e; border-radius: 12px; padding: 40px;
    max-width: 420px; width: 100%; box-shadow: 0 8px 32px rgba(0,0,0,0.3);
  }}
  h1 {{ font-size: 24px; margin-bottom: 8px; }}
  .subtitle {{ color: #888; margin-bottom: 24px; }}
  .code {{ font-family: monospace; font-size: 28px; letter-spacing: 4px;
    color: #64ffda; text-align: center; padding: 16px;
    background: #0f3460; border-radius: 8px; margin: 16px 0; }}
  label {{ display: block; margin-bottom: 4px; font-size: 14px; color: #aaa; }}
  input {{ width: 100%; padding: 10px 12px; border: 1px solid #333;
    border-radius: 6px; background: #0f3460; color: #e0e0e0;
    font-size: 14px; margin-bottom: 12px; outline: none; }}
  input:focus {{ border-color: #64ffda; }}
  button {{
    width: 100%; padding: 12px; border: none; border-radius: 6px;
    background: #64ffda; color: #1a1a2e; font-size: 16px;
    font-weight: 600; cursor: pointer; transition: opacity 0.2s;
  }}
  button:hover {{ opacity: 0.9; }}
  button:disabled {{ opacity: 0.5; cursor: default; }}
  .msg {{ text-align: center; padding: 12px; border-radius: 6px;
    margin-top: 16px; font-size: 14px; }}
  .msg.ok {{ background: rgba(100,255,218,0.1); color: #64ffda; }}
  .msg.err {{ background: rgba(255,82,82,0.1); color: #ff5252; }}
  .step {{ display: none; }}
  .step.active {{ display: block; }}
</style>
</head>
<body>
<div class="card">
  <h1>🪼 Plankton CLI Login</h1>
  <p class="subtitle">Authorize your terminal session</p>

  <div id="step-login" class="step active">
    <form id="login-form">
      <label for="username">Username</label>
      <input id="username" type="text" autocomplete="username" required autofocus>
      <label for="password">Password</label>
      <input id="password" type="password" autocomplete="current-password" required>
      <button type="submit" id="login-btn">Login & Authorize</button>
    </form>
    <div id="login-msg"></div>
  </div>

  <div id="step-approve" class="step">
    <p style="margin-bottom:12px">Verification code from your terminal:</p>
    <div class="code" id="session-code">------</div>
    <p style="margin-bottom:16px; font-size:13px; color:#888">
      Make sure this matches the code shown in your terminal.
    </p>
    <button id="approve-btn">Approve</button>
    <div id="approve-msg"></div>
  </div>

  <div id="step-done" class="step">
    <div class="msg ok">
      ✓ CLI session approved!<br>
      You can close this tab and return to your terminal.
    </div>
  </div>

  <div id="step-error" class="step">
    <div class="msg err" id="error-text">Session not found or expired.</div>
  </div>
</div>

<script>
(function() {{
  const params = new URLSearchParams(location.search);
  const sessionId = params.get('session');

  if (!sessionId) {{
    show('step-error');
    document.getElementById('error-text').textContent = 'No session ID provided.';
    return;
  }}

  // Prüfe ob schon eingeloggt (Cookie).
  fetch('{base_url}/auth/me', {{ credentials: 'include' }})
    .then(r => r.ok ? r.json() : null)
    .then(user => {{
      if (user && !user.must_change_password) {{
        // Schon eingeloggt: Session-Code laden und direkt Approve zeigen.
        loadSessionCode(sessionId);
        show('step-approve');
      }}
    }})
    .catch(() => {{}});

  // Login-Formular.
  document.getElementById('login-form').addEventListener('submit', async (e) => {{
    e.preventDefault();
    const btn = document.getElementById('login-btn');
    const msg = document.getElementById('login-msg');
    btn.disabled = true;
    msg.innerHTML = '';

    try {{
      const resp = await fetch('{base_url}/auth/login', {{
        method: 'POST',
        headers: {{ 'Content-Type': 'application/json' }},
        credentials: 'include',
        body: JSON.stringify({{
          username: document.getElementById('username').value,
          password: document.getElementById('password').value,
        }}),
      }});

      if (!resp.ok) {{
        const err = await resp.json().catch(() => ({{}}));
        throw new Error(err.error || 'Login failed');
      }}

      loadSessionCode(sessionId);
      show('step-approve');
    }} catch (err) {{
      msg.innerHTML = '<div class="msg err">' + err.message + '</div>';
      btn.disabled = false;
    }}
  }});

  // Approve-Button.
  document.getElementById('approve-btn').addEventListener('click', async () => {{
    const btn = document.getElementById('approve-btn');
    const msg = document.getElementById('approve-msg');
    btn.disabled = true;

    try {{
      const resp = await fetch('{base_url}/auth/cli-approve', {{
        method: 'POST',
        headers: {{ 'Content-Type': 'application/json' }},
        credentials: 'include',
        body: JSON.stringify({{ session_id: sessionId }}),
      }});

      if (!resp.ok) {{
        const err = await resp.json().catch(() => ({{}}));
        throw new Error(err.error || 'Approval failed');
      }}

      show('step-done');
    }} catch (err) {{
      msg.innerHTML = '<div class="msg err">' + err.message + '</div>';
      btn.disabled = false;
    }}
  }});

  function show(stepId) {{
    document.querySelectorAll('.step').forEach(s => s.classList.remove('active'));
    document.getElementById(stepId).classList.add('active');
  }}

  async function loadSessionCode(sid) {{
    try {{
      const resp = await fetch('{base_url}/auth/cli-poll/' + sid);
      const data = await resp.json();
      if (data.status === 'expired') {{
        show('step-error');
        return;
      }}
      if (data.code) {{
        document.getElementById('session-code').textContent = data.code;
      }}
    }} catch (e) {{}}
  }}
}})();
</script>
</body>
</html>"##,
        base_url = base_url,
    ))
}
