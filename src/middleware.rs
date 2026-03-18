// Middleware: Auth-Guard, Request-Logger, Startup-Banner.

use std::time::Instant;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Local;

use crate::models::Claims;
use crate::services::{extract_token_from_headers, validate_jwt};
use crate::state::AppState;

// ANSI-Farbcodes für Terminal-Ausgabe.
const RESET: &str = "\x1b[0m";
const GREEN: &str = "\x1b[32m";
const BLUE: &str = "\x1b[34m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";

/// Gibt die passende ANSI-Farbe für eine HTTP-Methode zurück.
fn method_color(method: &str) -> &'static str {
    match method {
        "GET" => GREEN,
        "POST" => BLUE,
        "PUT" => YELLOW,
        "DELETE" => RED,
        _ => RESET,
    }
}

/// Gibt die passende ANSI-Farbe für einen HTTP-Status-Code zurück.
fn status_color(status: u16) -> &'static str {
    match status {
        200..=299 => GREEN,
        400..=499 => YELLOW,
        500..=599 => RED,
        _ => RESET,
    }
}

/// Middleware: Loggt jeden Request mit Methode, Pfad, Status und Dauer.
pub async fn request_logger(req: Request, next: Next) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let start = Instant::now();

    let response = next.run(req).await;

    let status = response.status().as_u16();
    let duration = start.elapsed();
    let ms = duration.as_secs_f64() * 1000.0;
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");

    println!(
        "{DIM}[{timestamp}]{RESET} {BOLD}{mc}{method:<7}{RESET} {path:<40} {sc}{status}{RESET}  {DIM}{ms:.1}ms{RESET}",
        mc = method_color(&method),
        sc = status_color(status),
    );

    response
}

/// Gibt das Startup-Banner und die Routen-Tabelle im Terminal aus.
pub fn print_startup_banner(port: &str) {
    let routes: &[(&str, &str)] = &[
        ("POST",   "/auth/login"),
        ("POST",   "/auth/logout"),
        ("GET",    "/auth/me"),
        ("POST",   "/auth/change-password"),
        ("GET",    "/api/projects"),
        ("POST",   "/api/projects"),
        ("GET",    "/api/projects/:id"),
        ("PUT",    "/api/projects/:id"),
        ("DELETE", "/api/projects/:id"),
        ("POST",   "/api/projects/:id/tasks"),
        ("PUT",    "/api/projects/:id/tasks/:task_id"),
        ("DELETE", "/api/projects/:id/tasks/:task_id"),
        ("POST",   "/api/projects/:id/tasks/:task_id/move"),
        ("POST",   "/api/projects/:id/columns"),
        ("PUT",    "/api/projects/:id/columns/:column_id"),
        ("DELETE", "/api/projects/:id/columns/:column_id"),
        ("POST",   "/api/projects/:id/users"),
        ("PUT",    "/api/projects/:id/users/:user_id"),
        ("DELETE", "/api/projects/:id/users/:user_id"),
        ("GET",    "/api/projects/:id/events"),
        ("GET",    "/api/admin/users"),
        ("POST",   "/api/admin/users"),
        ("PUT",    "/api/admin/users/:id"),
        ("DELETE", "/api/admin/users/:id"),
        ("PUT",    "/api/admin/users/:id/password"),
        ("GET",    "/api/admin/tokens"),
        ("POST",   "/api/admin/tokens"),
        ("PUT",    "/api/admin/tokens/:id"),
        ("DELETE", "/api/admin/tokens/:id"),
        ("GET",    "/mcp/tools"),
        ("POST",   "/mcp/call"),
        ("POST",   "/mcp (JSON-RPC 2.0)"),
        ("GET",    "/docs"),
        ("GET",    "/skill.md"),
        ("GET",    "/install"),
        ("GET",    "/cli/plankton"),
        ("GET",    "/cli-login"),
        ("POST",   "/auth/cli-init"),
        ("GET",    "/auth/cli-poll/:id"),
        ("POST",   "/auth/cli-approve"),
    ];

    println!();
    println!("  {BOLD}🪼 Plankton v0.1.0{RESET}");
    println!("  {DIM}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{RESET}");
    for (method, path) in routes {
        let mc = method_color(method);
        println!("  {mc}{BOLD}{method:<7}{RESET} {path}");
    }
    println!("  {DIM}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{RESET}");
    println!("  {GREEN}listening on 0.0.0.0:{port}{RESET}");
    println!();
}

/// Auth-Guard Middleware: Prüft JWT oder Agent-Token für geschützte Routen.
pub async fn auth_guard(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();

    // Öffentliche Pfade: /auth/*, /docs und statische Dateien.
    if path.starts_with("/auth/")
        || path == "/docs"
        || (!path.starts_with("/api/") && !path.starts_with("/mcp/"))
    {
        return next.run(req).await;
    }

    // 1) JWT-Token versuchen (Cookie oder Bearer).
    let jwt_token = extract_token_from_headers(req.headers());
    if let Some(ref t) = jwt_token {
        if let Ok(claims) = validate_jwt(t, &state.jwt_secret) {
            if path.starts_with("/api/admin/") && claims.role != "admin" {
                return (
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({"error": "Admin required"})),
                )
                    .into_response();
            }
            req.extensions_mut().insert(claims);
            return next.run(req).await;
        }
    }

    // 2) Agent-Token versuchen (nur Bearer-Header).
    let bearer = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    if let Some(bearer_val) = bearer {
        if let Ok(agent_token) = state.store.get_token_by_value(&bearer_val).await {
            if path.starts_with("/api/admin/") {
                return (
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({"error": "Admin required"})),
                )
                    .into_response();
            }
            let claims = Claims {
                sub: agent_token.id.clone(),
                username: agent_token.name.clone(),
                display_name: agent_token.name.clone(),
                role: agent_token.role.clone(),
                exp: usize::MAX,
                must_change_password: false,
            };
            req.extensions_mut().insert(claims);
            return next.run(req).await;
        }
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error": "Not authenticated"})),
    )
        .into_response()
}
