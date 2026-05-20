# Authentifizierung & OAuth 2.0

## Übersicht der Auth-Mechanismen

| Mechanismus | Verwendung |
|------------|-----------|
| JWT (Cookie) | Browser-Session nach Login |
| JWT (Bearer) | API-Aufrufe von externen Clients |
| Agent-Token (`plk_`) | KI-Agenten, CI/CD, Skripte |
| OAuth 2.0 Authorization Code | Externe Apps (z.B. claude.ai) |
| Device Flow | CLI (`plankton remote add`) |

---

## JWT-Session-Auth

### Token-Erstellung

Nach erfolgreichen Login (`POST /auth/login`) wird ein JWT-Cookie gesetzt:

```
Set-Cookie: plankton_token=<jwt>; HttpOnly; SameSite=Strict
```

**Claims:**
```rust
Claims {
    sub: String,              // User-UUID
    username: String,
    display_name: String,
    role: String,             // "admin" | "manager" | "developer" | "tester" | "user"
    exp: usize,               // Unix-Timestamp (Browser: 8h, CLI: 30 Tage)
    must_change_password: bool,
}
```

### Token-Extraktion (Priorität im `auth_guard`)

1. Cookie `plankton_token=<jwt>`
2. Header `Authorization: Bearer <jwt>`
3. Header `Authorization: Bearer plk_<hex>` → Agent-Token-Lookup

### Passwort-Hashing

- Algorithmus: **Argon2id** (via `argon2` crate)
- Salt: OsRng-generiert
- Format: PHC-String (`$argon2id$v=19$m=...`)

### Öffentliche Pfade (auth_guard übersprungen)

```
/auth/*
/authorize  /oauth/*
/token  /register
/.well-known/*
/cli-login  /auth/cli-*
/install  /cli/*
/healthz  /docs  /skill.md
Alle statischen Dateien (nicht /api/, nicht /mcp)
```

---

## Agent-Tokens

Format: `plk_` + 48 Hex-Zeichen

Erstellt im Admin-Panel oder via `POST /api/admin/tokens`. Werden direkt im `Authorization`-Header übergeben:

```
Authorization: Bearer plk_a1b2c3...
```

Tokens haben eine Rolle (`developer`, `tester`, etc.) und können deaktiviert werden ohne das Passwort zu kennen.

---

## OAuth 2.0 Authorization Code Flow

Für externe Apps (claude.ai, eigene Clients).

### Flow

```
Client                    Plankton                    Browser
  │                           │                           │
  │── GET /authorize ─────────▶                           │
  │   ?client_id=...          │                           │
  │   &redirect_uri=...       │── Zeigt Consent-Screen ──▶│
  │   &response_type=code     │                           │
  │   &state=...              │◀── User genehmigt ────────│
  │   &code_challenge=...     │                           │
  │                           │── Redirect mit code= ─────▶
  │◀── ?code=...&state=... ───│                           │
  │                           │                           │
  │── POST /token ────────────▶                           │
  │   code + code_verifier    │                           │
  │◀── access_token ──────────│                           │
```

### PKCE (für Public Clients)

- `code_challenge_method`: nur `S256` (kein `plain`)
- Challenge: `BASE64URL(SHA256(code_verifier))`
- Token-Request muss `code_verifier` mitschicken

### Client-Registrierung

```
POST /register
{
  "client_name": "Meine App",
  "redirect_uris": ["https://example.com/callback"],
  "token_endpoint_auth_method": "none"  // oder "client_secret_post"
}
```

### Authorization Code

- Kurzlebig (~10 Min), danach gelöscht
- Einmalig: nach Einlösung sofort ungültig
- Gespeichert in `data/oauth/codes/<hex>.json`

### Refresh Token

- Gültig für wiederholte Token-Erneuerung ohne Re-Login
- `grant_type=refresh_token` im Token-Request
- Gespeichert in `data/oauth/refresh/<hex>.json`

### Discovery Endpoints

| Endpoint | RFC |
|---------|-----|
| `GET /.well-known/oauth-authorization-server` | RFC 8414 |
| `GET /.well-known/oauth-protected-resource` | RFC 9728 |

---

## CLI Device Flow

Für `plankton remote add <url>` im Terminal.

```
1. CLI → POST /auth/cli-init
         ← {session_id, code: "ABCD12", login_url}

2. CLI zeigt: "Öffne http://host/cli-login?session=..."
              "Code: ABCD12"

3. Browser → GET /cli-login?session=<id>
             ← HTML-Formular (Login + Approve-Button)

4. Browser → POST /auth/cli-approve
   (mit gültigem JWT-Cookie)

5. CLI pollt → GET /auth/cli-poll/<session_id>
              ← {status: "approved", token: "<jwt>"}

6. CLI speichert Token (30 Tage gültig)
```

### Session-Cleanup

Ein Background-Task läuft jede 60 Sekunden und löscht Sessions, die älter als 5 Minuten sind.
