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
echo "    plankton remote add origin ${{PLANKTON_URL}}"
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
INSTALLED_FROM="{default_url}"
CONFIG_DIR="${{HOME}}/.config/plankton"
CONFIG_FILE="${{CONFIG_DIR}}/config"
DEFAULT_SERVER="{default_url}"

# ─── Konfiguration (Multi-Remote INI-Format) ────────────────

load_config() {{
    PLANKTON_SERVER=""
    PLANKTON_TOKEN=""
    CURRENT_REMOTE=""
    if [[ ! -f "$CONFIG_FILE" ]]; then
        return
    fi

    # Legacy-Format erkennen (flaches PLANKTON_SERVER=... ohne Sektionen)
    if grep -q '^PLANKTON_SERVER=' "$CONFIG_FILE" 2>/dev/null && ! grep -q '^\[' "$CONFIG_FILE" 2>/dev/null; then
        migrate_legacy_config
        return
    fi

    # INI-Format lesen
    CURRENT_REMOTE=$(grep '^CURRENT_REMOTE=' "$CONFIG_FILE" 2>/dev/null | head -1 | cut -d= -f2-)
    if [[ -z "$CURRENT_REMOTE" ]]; then
        return
    fi

    # Aktiven Remote laden
    local in_section=false
    while IFS= read -r line; do
        [[ -z "$line" || "$line" =~ ^# ]] && continue
        if [[ "$line" == "[$CURRENT_REMOTE]" ]]; then
            in_section=true
            continue
        fi
        if [[ "$line" =~ ^\[ ]]; then
            $in_section && break
            continue
        fi
        if $in_section; then
            case "$line" in
                URL=*) PLANKTON_SERVER="${{line#URL=}}" ;;
                PLANKTON_TOKEN=*) PLANKTON_TOKEN="${{line#PLANKTON_TOKEN=}}" ;;
            esac
        fi
    done < "$CONFIG_FILE"
}}

migrate_legacy_config() {{
    local old_server="" old_token=""
    source "$CONFIG_FILE"
    old_server="${{PLANKTON_SERVER:-}}"
    old_token="${{PLANKTON_TOKEN:-}}"

    if [[ -n "$old_server" ]]; then
        # Name aus URL ableiten
        local name="default"
        CURRENT_REMOTE="$name"
        PLANKTON_SERVER="$old_server"
        PLANKTON_TOKEN="$old_token"
        save_config_remote "$name" "$old_server" "$old_token"
    fi
}}

# Einen einzelnen Remote in die Config schreiben (upsert)
save_config_remote() {{
    local name="$1" url="$2" token="$3"
    mkdir -p "$CONFIG_DIR"

    if [[ ! -f "$CONFIG_FILE" ]] || ! grep -q '^\[' "$CONFIG_FILE" 2>/dev/null; then
        # Neue Config anlegen
        cat > "$CONFIG_FILE" <<CONF
CURRENT_REMOTE=$CURRENT_REMOTE

[$name]
URL=$url
PLANKTON_TOKEN=$token
CONF
        chmod 600 "$CONFIG_FILE"
        return
    fi

    # CURRENT_REMOTE aktualisieren
    if grep -q '^CURRENT_REMOTE=' "$CONFIG_FILE"; then
        sed -i "s|^CURRENT_REMOTE=.*|CURRENT_REMOTE=$CURRENT_REMOTE|" "$CONFIG_FILE"
    else
        sed -i "1i CURRENT_REMOTE=$CURRENT_REMOTE" "$CONFIG_FILE"
    fi

    # Sektion ersetzen oder hinzufügen
    local tmpfile
    tmpfile=$(mktemp)
    local in_section=false replaced=false
    while IFS= read -r line; do
        if [[ "$line" == "[$name]" ]]; then
            in_section=true
            replaced=true
            echo "[$name]" >> "$tmpfile"
            echo "URL=$url" >> "$tmpfile"
            echo "PLANKTON_TOKEN=$token" >> "$tmpfile"
            continue
        fi
        if [[ "$line" =~ ^\[ ]]; then
            in_section=false
        fi
        if ! $in_section; then
            echo "$line" >> "$tmpfile"
        fi
    done < "$CONFIG_FILE"

    if ! $replaced; then
        echo "" >> "$tmpfile"
        echo "[$name]" >> "$tmpfile"
        echo "URL=$url" >> "$tmpfile"
        echo "PLANKTON_TOKEN=$token" >> "$tmpfile"
    fi

    mv "$tmpfile" "$CONFIG_FILE"
    chmod 600 "$CONFIG_FILE"
}}

# Remote aus Config entfernen
remove_config_remote() {{
    local name="$1"
    if [[ ! -f "$CONFIG_FILE" ]]; then return; fi

    local tmpfile
    tmpfile=$(mktemp)
    local in_section=false
    while IFS= read -r line; do
        if [[ "$line" == "[$name]" ]]; then
            in_section=true
            continue
        fi
        if [[ "$line" =~ ^\[ ]]; then
            in_section=false
        fi
        if ! $in_section; then
            echo "$line" >> "$tmpfile"
        fi
    done < "$CONFIG_FILE"

    # Falls gelöschter Remote der aktive war, CURRENT_REMOTE leeren
    if [[ "$CURRENT_REMOTE" == "$name" ]]; then
        sed -i "s|^CURRENT_REMOTE=.*|CURRENT_REMOTE=|" "$tmpfile"
    fi

    mv "$tmpfile" "$CONFIG_FILE"
    chmod 600 "$CONFIG_FILE"
}}

# Alle Remote-Namen auflisten
list_remotes() {{
    if [[ ! -f "$CONFIG_FILE" ]]; then return; fi
    grep '^\[' "$CONFIG_FILE" | tr -d '[]'
}}

# plankton_secrets.md generieren
update_secrets_md() {{
    local secrets_dir="${{HOME}}/.claude"
    local secrets_file="${{secrets_dir}}/plankton_secrets.md"
    mkdir -p "$secrets_dir"

    if [[ ! -f "$CONFIG_FILE" ]]; then
        rm -f "$secrets_file"
        return
    fi

    local content="# Plankton Server Tokens"
    content+=$'\n'

    local current_section="" current_url="" current_token=""
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        [[ "$line" =~ ^CURRENT_REMOTE= ]] && continue
        if [[ "$line" =~ ^\[(.+)\]$ ]]; then
            # Vorherige Sektion schreiben
            if [[ -n "$current_section" && -n "$current_url" && -n "$current_token" ]]; then
                local host
                host=$(echo "$current_url" | sed 's|https\?://||;s|/$||')
                content+=$'\n'"[$host]"$'\n'"URL=$current_url"$'\n'"PLANKTON_TOKEN=$current_token"$'\n'
            fi
            current_section="${{BASH_REMATCH[1]}}"
            current_url=""
            current_token=""
            continue
        fi
        case "$line" in
            URL=*) current_url="${{line#URL=}}" ;;
            PLANKTON_TOKEN=*) current_token="${{line#PLANKTON_TOKEN=}}" ;;
        esac
    done < "$CONFIG_FILE"

    # Letzte Sektion schreiben
    if [[ -n "$current_section" && -n "$current_url" && -n "$current_token" ]]; then
        local host
        host=$(echo "$current_url" | sed 's|https\?://||;s|/$||')
        content+=$'\n'"[$host]"$'\n'"URL=$current_url"$'\n'"PLANKTON_TOKEN=$current_token"$'\n'
    fi

    echo "$content" > "$secrets_file"
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
    load_config
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

            # Remote-Name bestimmen: aktueller Remote oder "default"
            local remote_name="${{CURRENT_REMOTE:-default}}"
            CURRENT_REMOTE="$remote_name"
            save_config_remote "$remote_name" "$server" "$token"
            update_secrets_md
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
            echo "  Remote: $remote_name"
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
    load_config
    local global=false
    local target_dir=".claude/skills/plankton"
    local server_url=""

    for arg in "$@"; do
        case "$arg" in
            --global|-g) global=true ;;
            https://*|http://*) server_url="${{arg%/}}" ;;
        esac
    done

    if $global; then
        target_dir="${{HOME}}/.claude/skills/plankton"
    fi

    # Server-URL bestimmen: Argument > aktiver Remote > Frage
    if [[ -z "$server_url" ]]; then
        if [[ -n "$PLANKTON_SERVER" ]]; then
            server_url="$PLANKTON_SERVER"
        else
            echo ""
            echo "  🪼 Plankton Skill Setup"
            echo "  ━━━━━━━━━━━━━━━━━━━━━━━"
            echo ""
            echo "  No server configured. Please provide the server URL."
            echo "  Example: plankton skill install https://plankton.tiny-dev.de"
            echo ""
            exit 1
        fi
    fi

    # Remote anlegen falls noch nicht vorhanden
    local remote_exists=false
    for r in $(list_remotes); do
        # Check ob dieser Remote die gleiche URL hat
        local r_url=""
        local in_section=false
        while IFS= read -r line; do
            if [[ "$line" == "[$r]" ]]; then in_section=true; continue; fi
            if [[ "$line" =~ ^\[ ]]; then $in_section && break; continue; fi
            if $in_section && [[ "$line" == URL=* ]]; then r_url="${{line#URL=}}"; fi
        done < "$CONFIG_FILE" 2>/dev/null
        if [[ "$r_url" == "$server_url" ]]; then
            remote_exists=true
            CURRENT_REMOTE="$r"
            break
        fi
    done

    if ! $remote_exists; then
        # Remote-Name aus URL ableiten (hostname ohne Punkte → Kurzform)
        local remote_name
        remote_name=$(echo "$server_url" | sed 's|https\?://||;s|/$||;s|\..*||')
        [[ -z "$remote_name" ]] && remote_name="default"
        CURRENT_REMOTE="$remote_name"
        save_config_remote "$remote_name" "$server_url" ""
        echo ""
        echo "  ✓ Remote '$remote_name' added: $server_url"
    fi

    # Login falls noch kein Token vorhanden
    load_config
    if [[ -z "$PLANKTON_TOKEN" ]]; then
        echo ""
        echo "  🪼 Plankton Skill Setup"
        echo "  ━━━━━━━━━━━━━━━━━━━━━━━"
        echo ""
        echo "  Logging in to $server_url ..."
        cmd_login "$server_url"
        load_config
        if [[ -z "$PLANKTON_TOKEN" ]]; then
            echo "  ✗ Login failed. Skill installed without authentication."
            echo "  Run: plankton login $server_url"
            echo ""
        fi
    fi

    # SKILL.md herunterladen
    mkdir -p "$target_dir"

    echo ""
    echo "  ↓ Downloading SKILL.md from $server_url ..."
    curl -fsSL "${{server_url}}/skill.md" -o "${{target_dir}}/SKILL.md"
    echo "  ✓ Installed to ${{target_dir}}/SKILL.md"

    # Secrets generieren
    update_secrets_md
    local secrets_file="${{HOME}}/.claude/plankton_secrets.md"
    if [[ -f "$secrets_file" ]]; then
        echo "  ✓ Secrets written to $secrets_file"
    fi
    echo ""

    echo "  Done! The /plankton skill is now available in Claude Code."
    echo "  Ticket URLs determine which server to use automatically."
    echo ""
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

    echo "  Remote: ${{CURRENT_REMOTE:-default}}"
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
    if [[ -n "$CURRENT_REMOTE" ]]; then
        save_config_remote "$CURRENT_REMOTE" "$PLANKTON_SERVER" ""
    fi
    update_secrets_md
    echo ""
    echo "  ✓ Logged out from ${{CURRENT_REMOTE:-default}}."
    echo ""
}}

# ─── Version & Info ──────────────────────────────────────────

cmd_version() {{
    echo "plankton $VERSION"
}}

cmd_info() {{
    load_config

    echo ""
    echo "  🪼 Plankton CLI"
    echo "  ━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "  Version:        $VERSION"
    echo "  Installed from: $INSTALLED_FROM"
    echo "  Config:         $CONFIG_FILE"
    echo "  Active remote:  ${{CURRENT_REMOTE:-(none)}}"
    echo ""

    if [[ -z "$PLANKTON_SERVER" ]]; then
        echo "  Server:         (not configured)"
        echo "  Auth:           ✗ not logged in"
    else
        echo "  Server:         $PLANKTON_SERVER"
        local me
        me=$(api GET /auth/me 2>/dev/null)
        if [[ $? -eq 0 && -n "$me" ]]; then
            local name role
            name=$(echo "$me" | jq -r '.display_name // .username // "unknown"')
            role=$(echo "$me" | jq -r '.role // "unknown"')
            echo "  Auth:           ✓ $name ($role)"
        else
            echo "  Auth:           ✗ token expired or invalid"
        fi
    fi
    echo ""
}}

# ─── Init (.vibe Struktur) ───────────────────────────────────

cmd_init() {{
    echo ""
    echo "  🪼 Plankton Init"
    echo "  ━━━━━━━━━━━━━━━━━"
    echo ""

    local dirs=(".vibe" ".vibe/ideas" ".vibe/epics" ".vibe/tasks" ".vibe/done" ".vibe/done/ideas" ".vibe/done/epics" ".vibe/done/tasks" ".vibe/log")

    for d in "${{dirs[@]}}"; do
        if [[ -d "$d" ]]; then
            echo "  · $d (exists)"
        else
            mkdir -p "$d"
            echo "  ✓ $d"
        fi
    done

    # README anlegen wenn nicht vorhanden.
    if [[ ! -f ".vibe/readme.md" ]]; then
        cat > ".vibe/readme.md" <<'VIBEEOF'
# Vibe – KI-Agenten Workflow

Dieses Verzeichnis wird von Plankton-KI-Agenten genutzt.

## Struktur

- `ideas/`      – Neue Ideen (Markdown-Dateien)
- `epics/`      – Aktive Epics
- `tasks/`      – Aktive Tasks
- `done/`       – Abgeschlossene Items
- `log/`        – Session-Logs

## Workflow

1. Idee als Markdown in `ideas/` anlegen
2. Supervisor/Architect erstellt daraus Epics und Tasks
3. Developer implementiert Tasks
4. Tester prüft und genehmigt
5. Erledigte Items werden nach `done/` verschoben
VIBEEOF
        echo "  ✓ .vibe/readme.md"
    else
        echo "  · .vibe/readme.md (exists)"
    fi

    echo ""
    echo "  Projekt-Struktur angelegt."
    echo "  Lege Ideen als Markdown-Dateien in .vibe/ideas/ ab."
    echo ""
}}

# ─── Projects & Tasks ─────────────────────────────────────────

cmd_projects() {{
    load_config
    if [ -z "$PLANKTON_TOKEN" ]; then echo "Not logged in. Run: plankton login <url>"; exit 1; fi
    local resp md=false
    for arg in "$@"; do [[ "$arg" == "--md" ]] && md=true; done
    resp=$(curl -sf -H "Authorization: Bearer $PLANKTON_TOKEN" "$PLANKTON_SERVER/api/projects") || {{ echo "Error fetching projects"; exit 1; }}
    if $md; then
        echo "# Projects"
        echo ""
        echo "| Slug | Title | Tasks |"
        echo "|------|-------|-------|"
        echo "$resp" | jq -r '.[] | "| \(.slug // ._id) | \(.title) | \(.tasks | length) |"'
    else
        echo ""
        echo "  Projects:"
        echo "  ━━━━━━━━━"
        echo "$resp" | jq -r '.[] | "  \(.slug // ._id)  \(.title)  (\(.tasks | length) tasks)"'
        echo ""
    fi
}}

cmd_view_project() {{
    load_config
    if [ -z "$PLANKTON_TOKEN" ]; then echo "Not logged in."; exit 1; fi
    local pid="" md=false
    for arg in "$@"; do
        case "$arg" in
            --md) md=true ;;
            *) [ -z "$pid" ] && pid="$arg" ;;
        esac
    done
    if [ -z "$pid" ]; then echo "Usage: plankton view <slug> [--md]"; exit 1; fi
    local resp
    resp=$(curl -sf -H "Authorization: Bearer $PLANKTON_TOKEN" "$PLANKTON_SERVER/api/projects/$pid?sort=order&group_epics=true") || {{ echo "Error fetching project"; exit 1; }}
    if $md; then
        local title
        title=$(echo "$resp" | jq -r '.title')
        echo "# $title"
        echo ""
        local hdr='#''#'
        echo "$resp" | jq -r --arg hdr "$hdr" '
            .columns[] as $col |
            if ($col.hidden != true) then
                .tasks as $tasks |
                ($hdr + " " + $col.title),
                "",
                ([$tasks[] | select(.column_id == $col.id) | select(.parent_id == "" or .parent_id == null)] | sort_by(.order) | if length > 0 then .[] |
                    . as $t |
                    "- **" + .title + "**" +
                    (if .task_type == "epic" then " [epic]" elif .task_type == "job" then " [job]" else "" end) +
                    (if .points > 0 then " (" + (.points|tostring) + "pt)" else "" end) +
                    (if .worker != "" then " @" + .worker else "" end) +
                    (if (.labels | length) > 0 then " `" + (.labels | join("` `")) + "`" else "" end),
                    ([$tasks[] | select(.parent_id == $t.id)] | sort_by(.order) | .[] |
                        "  - " + .title +
                        (if .points > 0 then " (" + (.points|tostring) + "pt)" else "" end) +
                        (if .worker != "" then " @" + .worker else "" end) +
                        (if (.labels | length) > 0 then " `" + (.labels | join("` `")) + "`" else "" end)
                    )
                else "_(empty)_" end),
                ""
            else empty end
        '
    else
        echo ""
        echo "  $(echo "$resp" | jq -r '.title')"
        echo "  ━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "$resp" | jq -r '.columns[] | select(.hidden != true) | "  \(.title): \(.id)"'
        echo ""
        echo "  Tasks by column:"
        echo "$resp" | jq -r '
            .columns[] as $col |
            if ($col.hidden != true) then
                "\n  ── \($col.title) ──",
                ([.tasks[] | select(.column_id == $col.id) | select(.parent_id == "" or .parent_id == null)] | sort_by(.order) | if length > 0 then .[] |
                    . as $t |
                    "    [\(.task_type // "task")] \(.title) (\(.worker // "-"))",
                    ([.tasks[] | select(.parent_id == $t.id)] | sort_by(.order) | .[] |
                        "      └─ \(.title) (\(.worker // "-"))"
                    )
                else "    (empty)" end)
            else empty end
        ' 2>/dev/null || echo "$resp" | jq -r '
            .columns[] as $col |
            if ($col.hidden != true) then
                "\n  ── \($col.title) ──",
                ([.tasks[] | select(.column_id == $col.id)] | sort_by(.order) | if length > 0 then .[] | "    [\(.task_type // "task")] \(.title) (\(.worker // "-"))" else "    (empty)" end)
            else empty end
        '
        echo ""
    fi
}}

cmd_tasks() {{
    load_config
    if [ -z "$PLANKTON_TOKEN" ]; then echo "Not logged in."; exit 1; fi
    local pid="" md=false
    for arg in "$@"; do
        case "$arg" in
            --md) md=true ;;
            *) [ -z "$pid" ] && pid="$arg" ;;
        esac
    done
    if [ -z "$pid" ]; then echo "Usage: plankton tasks <slug> [--md]"; exit 1; fi
    local resp
    resp=$(curl -sf -H "Authorization: Bearer $PLANKTON_TOKEN" "$PLANKTON_SERVER/api/projects/$pid?sort=order&group_epics=true") || {{ echo "Error fetching project"; exit 1; }}
    if $md; then
        echo "# Tasks – $(echo "$resp" | jq -r '.title')"
        echo ""
        echo "| # | Type | Title | Column | Pts | Worker | Labels |"
        echo "|---|------|-------|--------|-----|--------|--------|"
        echo "$resp" | jq -r '
            .columns as $cols |
            .tasks | to_entries[] |
            .key as $idx | .value as $t |
            ($cols[] | select(.id == $t.column_id) | .title) as $col |
            "| \($idx + 1) | \($t.task_type // "task") | \(if $t.parent_id != "" and $t.parent_id != null then "  -> " else "" end)\($t.title) | \($col) | \($t.points) | \($t.worker // "-") | \($t.labels | join(", ")) |"
        '
    else
        echo ""
        printf "  %-4s %-6s %-40s %-12s %-4s %s\n" "#" "TYPE" "TITLE" "COLUMN" "PTS" "WORKER"
        echo "  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "$resp" | jq -r '
            .columns as $cols |
            .tasks | to_entries[] |
            .key as $idx | .value as $t |
            ($cols[] | select(.id == $t.column_id) | .title) as $col |
            "  \($idx + 1 | tostring | .[0:4]) \($t.task_type // "task" | .[0:6]) \(if $t.parent_id != "" and $t.parent_id != null then "  └ " else "" end)\($t.title[:40]) \($col[:12]) \($t.points) \($t.worker // "-")"
        '
        echo ""
    fi
}}

# ─── Export / Import ──────────────────────────────────────────

cmd_export() {{
    load_config
    if [ -z "$PLANKTON_TOKEN" ]; then echo "Not logged in. Run: plankton login <url>"; exit 1; fi
    local force=0
    local target_dir="."
    local filter_slug=""
    while [ $# -gt 0 ]; do
        case "$1" in
            -f|--force) force=1; shift ;;
            -d|--dir) shift; target_dir="${{1:-.}}"; shift ;;
            -p|--project) shift; filter_slug="${{1:-}}"; shift ;;
            *) shift ;;
        esac
    done
    mkdir -p "$target_dir"
    local resp
    resp=$(curl -sf -H "Authorization: Bearer $PLANKTON_TOKEN" "$PLANKTON_SERVER/api/projects") || {{ echo "Error fetching projects"; exit 1; }}

    # Einzelnes Projekt per Slug filtern
    if [ -n "$filter_slug" ]; then
        local match=$(echo "$resp" | jq -r ".[] | select(.slug == \"$filter_slug\") | ._id")
        if [ -z "$match" ]; then
            echo "  Error: project '$filter_slug' not found on server"
            exit 1
        fi
    fi

    local count=0
    local skipped=0
    echo "$resp" | jq -c '.[]' | while IFS= read -r entry; do
        local id=$(echo "$entry" | jq -r '._id')
        local slug=$(echo "$entry" | jq -r '.slug // empty')
        local title=$(echo "$entry" | jq -r '.title')
        [ -z "$slug" ] && slug="$id"

        # Filter: nur das angegebene Projekt
        if [ -n "$filter_slug" ] && [ "$slug" != "$filter_slug" ]; then
            continue
        fi

        local file="$target_dir/$slug.json"
        if [ -f "$file" ] && [ "$force" -eq 0 ]; then
            echo "  skip  $title ($file exists, use -f to overwrite)"
            continue
        fi
        local project
        project=$(curl -sf -H "Authorization: Bearer $PLANKTON_TOKEN" "$PLANKTON_SERVER/api/projects/$id?include_archived=true") || {{ echo "  error fetching $title"; continue; }}
        echo "$project" | jq '.' > "$file"
        echo "  saved $title → $file"
    done
    echo ""
}}

cmd_import() {{
    load_config
    if [ -z "$PLANKTON_TOKEN" ]; then echo "Not logged in. Run: plankton login <url>"; exit 1; fi
    local force=0
    local target_dir="."
    local filter_slug=""
    while [ $# -gt 0 ]; do
        case "$1" in
            -f|--force) force=1; shift ;;
            -d|--dir) shift; target_dir="${{1:-.}}"; shift ;;
            -p|--project) shift; filter_slug="${{1:-}}"; shift ;;
            *) shift ;;
        esac
    done

    # Einzelnes Projekt: Datei muss existieren
    if [ -n "$filter_slug" ]; then
        local file="$target_dir/$filter_slug.json"
        if [ ! -f "$file" ]; then
            echo "  Error: file '$file' not found"
            exit 1
        fi
    fi

    # Bestehende Projekte auf dem Server laden
    local server_projects
    server_projects=$(curl -sf -H "Authorization: Bearer $PLANKTON_TOKEN" "$PLANKTON_SERVER/api/projects") || {{ echo "Error fetching projects"; exit 1; }}

    local count=0
    local skipped=0
    for file in "$target_dir"/*.json; do
        [ -f "$file" ] || continue
        local basename=$(basename "$file" .json)

        # Filter: nur das angegebene Projekt
        if [ -n "$filter_slug" ] && [ "$basename" != "$filter_slug" ]; then
            continue
        fi

        local id=$(jq -r '._id' "$file" 2>/dev/null)
        local title=$(jq -r '.title' "$file" 2>/dev/null)
        [ -z "$id" ] || [ "$id" = "null" ] && {{ echo "  skip  $file (no _id)"; continue; }}

        # Prüfen ob Projekt auf dem Server existiert (per ID oder Slug)
        local server_match=$(echo "$server_projects" | jq -r ".[] | select(._id == \"$id\" or .slug == \"$basename\") | ._id")
        if [ -n "$server_match" ]; then
            if [ "$force" -eq 0 ]; then
                echo "  skip  $title (exists on server, use -f to overwrite)"
                skipped=$((skipped + 1))
                continue
            fi
            # Force: überschreiben via PUT (aktuelle _rev vom Server holen)
            local rev
            rev=$(echo "$server_projects" | jq -r ".[] | select(._id == \"$server_match\") | ._rev")
            local data=$(jq --arg rev "$rev" --arg id "$server_match" '._rev = $rev | ._id = $id' "$file")
            curl -sf -X PUT -H "Authorization: Bearer $PLANKTON_TOKEN" -H "Content-Type: application/json" \
                "$PLANKTON_SERVER/api/projects/$server_match" -d "$data" > /dev/null || {{ echo "  error updating $title"; continue; }}
            echo "  updated $title → $server_match"
            count=$((count + 1))
        else
            # Neu: POST
            curl -sf -X POST -H "Authorization: Bearer $PLANKTON_TOKEN" -H "Content-Type: application/json" \
                "$PLANKTON_SERVER/api/projects" -d @"$file" > /dev/null || {{ echo "  error creating $title"; continue; }}
            echo "  created $title"
            count=$((count + 1))
        fi
    done
    echo ""
    echo "  Imported $count project(s), skipped $skipped."
    echo ""
}}

# ─── Remote ──────────────────────────────────────────────────

cmd_remote() {{
    load_config
    local subcmd="${{1:-list}}"
    shift 2>/dev/null || true

    case "$subcmd" in
        add)
            local name="$1" url="$2"
            if [[ -z "$name" || -z "$url" ]]; then
                echo "Usage: plankton remote add <name> <url>"
                exit 1
            fi
            url="${{url%/}}"
            CURRENT_REMOTE="$name"
            save_config_remote "$name" "$url" ""
            echo ""
            echo "  ✓ Remote '$name' added: $url"
            echo "  (set as active remote)"
            echo ""

            # Automatisch einloggen
            cmd_login "$url"
            ;;
        remove|rm)
            local name="$1"
            if [[ -z "$name" ]]; then
                echo "Usage: plankton remote remove <name>"
                exit 1
            fi
            # Token löschen (Logout) und Remote entfernen
            remove_config_remote "$name"
            load_config
            update_secrets_md
            echo ""
            echo "  ✓ Remote '$name' removed (logged out)."
            echo ""
            ;;
        switch)
            local name="$1"
            if [[ -z "$name" ]]; then
                echo "Usage: plankton remote switch <name>"
                exit 1
            fi
            local found=false
            for r in $(list_remotes); do
                [[ "$r" == "$name" ]] && found=true
            done
            if ! $found; then
                echo "  ✗ Remote '$name' not found."
                echo "  Available: $(list_remotes | tr '\n' ' ')"
                exit 1
            fi
            CURRENT_REMOTE="$name"
            # CURRENT_REMOTE in Config aktualisieren
            if grep -q '^CURRENT_REMOTE=' "$CONFIG_FILE" 2>/dev/null; then
                sed -i "s|^CURRENT_REMOTE=.*|CURRENT_REMOTE=$name|" "$CONFIG_FILE"
            fi
            load_config
            echo ""
            echo "  ✓ Switched to remote '$name' ($PLANKTON_SERVER)"
            echo ""
            ;;
        list|"")
            echo ""
            echo "  Remotes:"
            echo "  ━━━━━━━━"
            if [[ ! -f "$CONFIG_FILE" ]]; then
                echo "  (none configured)"
                echo ""
                return
            fi
            local current_section="" current_url=""
            while IFS= read -r line; do
                [[ -z "$line" ]] && continue
                [[ "$line" =~ ^CURRENT_REMOTE= ]] && continue
                if [[ "$line" =~ ^\[(.+)\]$ ]]; then
                    if [[ -n "$current_section" ]]; then
                        local marker="  "
                        [[ "$current_section" == "$CURRENT_REMOTE" ]] && marker="* "
                        echo "  $marker$current_section  $current_url"
                    fi
                    current_section="${{BASH_REMATCH[1]}}"
                    current_url=""
                    continue
                fi
                case "$line" in
                    URL=*) current_url="${{line#URL=}}" ;;
                esac
            done < "$CONFIG_FILE"
            if [[ -n "$current_section" ]]; then
                local marker="  "
                [[ "$current_section" == "$CURRENT_REMOTE" ]] && marker="* "
                echo "  $marker$current_section  $current_url"
            fi
            echo ""
            ;;
        *)
            echo "Unknown remote command: $subcmd"
            echo "Usage: plankton remote [add|remove|switch|list]"
            exit 1
            ;;
    esac
}}

cmd_use() {{
    cmd_remote switch "$@"
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
    echo "    remote               List configured remotes"
    echo "    remote add <n> <url> Add remote + login (like git remote)"
    echo "    remote remove <n>    Remove remote + logout"
    echo "    remote switch <n>    Switch active remote"
    echo "    use <name>           Shortcut for remote switch"
    echo "    login [url]          Re-login to current remote"
    echo "    status               Show connection info"
    echo "    projects [--md]      List all projects (--md for Markdown output)"
    echo "    view <slug> [--md]   View project with columns and tasks"
    echo "    tasks <slug> [--md]  List tasks in a project"
    echo "    export [-f] [-p slug] [-d dir]  Export projects as JSON"
    echo "    import [-f] [-p slug] [-d dir]  Import JSON to server"
    echo "    init                 Create .vibe/ project structure"
    echo "    skill install [-g]   Download & install SKILL.md"
    echo "    skill update  [-g]   Update installed SKILL.md"
    echo "    tokens               List agent tokens (admin)"
    echo "    help                 Show this help"
    echo ""
    echo "  Options:"
    echo "    -g, --global         Install skill to ~/.claude/ (default: .claude/)"
    echo "    --version            Show version number"
    echo "    --info               Show version, server, and auth status"
    echo ""
    echo "  Install / Update:"
    echo "    curl -fsSL $INSTALLED_FROM/install | bash"
    echo ""
}}

# ─── Main ────────────────────────────────────────────────────

case "${{1:-help}}" in
    login)      shift; cmd_login "$@" ;;
    logout)     cmd_logout ;;
    status)     cmd_status ;;
    remote)     shift; cmd_remote "$@" ;;
    use)        shift; cmd_use "$@" ;;
    projects)   shift; cmd_projects "$@" ;;
    view)       shift; cmd_view_project "$@" ;;
    tasks)      shift; cmd_tasks "$@" ;;
    export)     shift; cmd_export "$@" ;;
    import)     shift; cmd_import "$@" ;;
    init)       cmd_init ;;
    skill)
        shift
        case "${{1:-install}}" in
            install) shift; cmd_skill_install "$@" ;;
            update)  shift; cmd_skill_update "$@" ;;
            *)       echo "Unknown skill command: $1"; cmd_help ;;
        esac
        ;;
    tokens)     cmd_tokens ;;
    --version)  cmd_version ;;
    --info)     cmd_info ;;
    help|--help|-h) cmd_help ;;
    *)          echo "Unknown command: $1"; cmd_help ;;
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
  :root {{
    --cli-bg: #1a1a2e; --cli-surface: #16213e; --cli-input-bg: #0f3460;
    --cli-text: #e0e0e0; --cli-text-dim: #aaa; --cli-border: #333;
    --cli-accent: #64ffda; --cli-accent-text: #1a1a2e;
    --cli-shadow: rgba(0,0,0,0.3);
    --cli-ok-bg: rgba(100,255,218,0.1); --cli-err-bg: rgba(255,82,82,0.1); --cli-err: #ff5252;
    color-scheme: dark;
  }}
  @media (prefers-color-scheme: light) {{
    :root:not([data-theme="dark"]) {{
      --cli-bg: #f5f5f7; --cli-surface: #ffffff; --cli-input-bg: #eeeef2;
      --cli-text: #1a1a2e; --cli-text-dim: #6e6e82; --cli-border: #d0d0d8;
      --cli-accent: #6b5ce7; --cli-accent-text: #ffffff;
      --cli-shadow: rgba(0,0,0,0.08);
      --cli-ok-bg: rgba(107,92,231,0.1); --cli-err-bg: rgba(255,82,82,0.1); --cli-err: #d94452;
      color-scheme: light;
    }}
  }}
  [data-theme="light"] {{
    --cli-bg: #f5f5f7; --cli-surface: #ffffff; --cli-input-bg: #eeeef2;
    --cli-text: #1a1a2e; --cli-text-dim: #6e6e82; --cli-border: #d0d0d8;
    --cli-accent: #6b5ce7; --cli-accent-text: #ffffff;
    --cli-shadow: rgba(0,0,0,0.08);
    --cli-ok-bg: rgba(107,92,231,0.1); --cli-err-bg: rgba(255,82,82,0.1); --cli-err: #d94452;
    color-scheme: light;
  }}
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: var(--cli-bg); color: var(--cli-text);
    display: flex; justify-content: center; align-items: center;
    min-height: 100vh;
  }}
  .card {{
    background: var(--cli-surface); border-radius: 12px; padding: 40px;
    max-width: 420px; width: 100%; box-shadow: 0 8px 32px var(--cli-shadow);
  }}
  h1 {{ font-size: 24px; margin-bottom: 8px; }}
  .subtitle {{ color: var(--cli-text-dim); margin-bottom: 24px; }}
  .code {{ font-family: monospace; font-size: 28px; letter-spacing: 4px;
    color: var(--cli-accent); text-align: center; padding: 16px;
    background: var(--cli-input-bg); border-radius: 8px; margin: 16px 0; }}
  label {{ display: block; margin-bottom: 4px; font-size: 14px; color: var(--cli-text-dim); }}
  input {{ width: 100%; padding: 10px 12px; border: 1px solid var(--cli-border);
    border-radius: 6px; background: var(--cli-input-bg); color: var(--cli-text);
    font-size: 14px; margin-bottom: 12px; outline: none; }}
  input:focus {{ border-color: var(--cli-accent); }}
  button {{
    width: 100%; padding: 12px; border: none; border-radius: 6px;
    background: var(--cli-accent); color: var(--cli-accent-text); font-size: 16px;
    font-weight: 600; cursor: pointer; transition: opacity 0.2s;
  }}
  button:hover {{ opacity: 0.9; }}
  button:disabled {{ opacity: 0.5; cursor: default; }}
  .msg {{ text-align: center; padding: 12px; border-radius: 6px;
    margin-top: 16px; font-size: 14px; }}
  .msg.ok {{ background: var(--cli-ok-bg); color: var(--cli-accent); }}
  .msg.err {{ background: var(--cli-err-bg); color: var(--cli-err); }}
  .step {{ display: none; }}
  .step.active {{ display: block; }}
</style>
</head>
<body>
<script>
// Theme vom Plankton-Board uebernehmen (localStorage) oder System-Preference nutzen.
(function(){{var t=localStorage.getItem('plankton-theme');if(t)document.documentElement.setAttribute('data-theme',t)}})();
</script>
<div class="card">
  <h1><img src="/icons/favicon-32.png" alt="" style="vertical-align:middle;margin-right:8px" />Plankton CLI Login</h1>
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
