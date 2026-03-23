#!/usr/bin/env fish

# ============================================
# Plankton MCP OAuth Flow Tester (Fish Shell)
# ============================================

set BASE_URL "https://plankton.tiny-dev.de"
set REDIRECT_URI "https://claude.ai/oauth/callback"

function pass
    echo -e "\033[0;32m✅ PASS\033[0m $argv"
end
function fail
    echo -e "\033[0;31m❌ FAIL\033[0m $argv"
end
function info
    echo -e "\033[0;34mℹ️  INFO\033[0m $argv"
end
function warn
    echo -e "\033[1;33m⚠️  WARN\033[0m $argv"
end
function header
    echo ""
    echo -e "\033[0;34m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m"
    echo -e "\033[0;34m  $argv\033[0m"
    echo -e "\033[0;34m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m"
end

# ============================================
# STEP 1: Well-Known Endpoint
# ============================================
header "STEP 1: /.well-known/oauth-authorization-server"

set WELL_KNOWN (curl -s "$BASE_URL/.well-known/oauth-authorization-server")
echo $WELL_KNOWN | python3 -m json.tool

for field in issuer authorization_endpoint token_endpoint response_types_supported code_challenge_methods_supported
    if echo $WELL_KNOWN | grep -q "\"$field\""
        pass "Feld vorhanden: $field"
    else
        fail "Feld fehlt: $field"
    end
end

if echo $WELL_KNOWN | grep -q "\"registration_endpoint\""
    pass "Dynamic Client Registration unterstützt"
else
    warn "registration_endpoint fehlt – Claude.ai kann sich nicht registrieren"
end

# ============================================
# STEP 2: Dynamic Client Registration
# ============================================
header "STEP 2: /register (token_endpoint_auth_method: none)"

set REGISTER_RESPONSE (curl -s -X POST "$BASE_URL/oauth/register" \
    -H "Content-Type: application/json" \
    -d '{
        "client_name": "Claude-Test",
        "redirect_uris": ["'$REDIRECT_URI'"],
        "grant_types": ["authorization_code"],
        "response_types": ["code"],
        "token_endpoint_auth_method": "none"
    }')

echo $REGISTER_RESPONSE | python3 -m json.tool

set CLIENT_ID (echo $REGISTER_RESPONSE | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('client_id',''))")
set CLIENT_SECRET (echo $REGISTER_RESPONSE | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('client_secret','NONE'))")
set AUTH_METHOD (echo $REGISTER_RESPONSE | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('token_endpoint_auth_method',''))")

if test -n "$CLIENT_ID"
    pass "client_id erhalten: $CLIENT_ID"
else
    fail "client_id fehlt – Abbruch"
    exit 1
end

if test "$AUTH_METHOD" = "none"
    pass "token_endpoint_auth_method ist 'none' ✓ (Claude.ai kompatibel)"
else
    fail "token_endpoint_auth_method ist '$AUTH_METHOD' – erwartet 'none' (Claude.ai wird scheitern!)"
end

if test "$CLIENT_SECRET" = "NONE" -o -z "$CLIENT_SECRET"
    pass "Kein client_secret zurückgegeben (korrekt für PKCE-only)"
else
    fail "client_secret wurde zurückgegeben: $CLIENT_SECRET (sollte bei 'none' fehlen!)"
end

# ============================================
# STEP 3: PKCE generieren
# ============================================
header "STEP 3: PKCE Code Verifier & Challenge generieren"

set CODE_VERIFIER (python3 -c "
import base64, os
verifier = base64.urlsafe_b64encode(os.urandom(32)).rstrip(b'=').decode()
print(verifier)
")

set CODE_CHALLENGE (python3 -c "
import base64, hashlib
verifier = '$CODE_VERIFIER'
digest = hashlib.sha256(verifier.encode()).digest()
challenge = base64.urlsafe_b64encode(digest).rstrip(b'=').decode()
print(challenge)
")

set STATE (python3 -c "import base64,os; print(base64.urlsafe_b64encode(os.urandom(16)).rstrip(b'=').decode())")

info "code_verifier:  $CODE_VERIFIER"
info "code_challenge: $CODE_CHALLENGE"
info "state:          $STATE"
pass "PKCE Parameter generiert"

# ============================================
# STEP 4: Authorization URL
# ============================================
header "STEP 4: /authorize URL"

set ENCODED_REDIRECT (python3 -c "import urllib.parse; print(urllib.parse.quote('$REDIRECT_URI'))")
set AUTH_URL "$BASE_URL/authorize?response_type=code&client_id=$CLIENT_ID&redirect_uri=$ENCODED_REDIRECT&code_challenge=$CODE_CHALLENGE&code_challenge_method=S256&state=$STATE"

info "Authorization URL:"
echo ""
echo $AUTH_URL
echo ""

set AUTH_STATUS (curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/authorize?response_type=code&client_id=$CLIENT_ID&redirect_uri=$REDIRECT_URI&code_challenge=$CODE_CHALLENGE&code_challenge_method=S256&state=$STATE" \
    --max-redirs 0)

if test "$AUTH_STATUS" = "200" -o "$AUTH_STATUS" = "302" -o "$AUTH_STATUS" = "301"
    pass "/authorize antwortet mit HTTP $AUTH_STATUS"
else
    fail "/authorize antwortet mit HTTP $AUTH_STATUS (erwartet 200 oder 302)"
end

warn "Öffne die URL oben im Browser, logge dich ein"
warn "Kopiere den 'code' Parameter aus der Redirect-URL:"
warn "  → https://claude.ai/oauth/callback?code=XXXXX&state=..."

# ============================================
# STEP 5: Token Exchange
# ============================================
header "STEP 5: /token - Code gegen Token tauschen"

echo ""
read --prompt "echo -e '\033[1;33mcode aus Redirect-URL eingeben (Enter zum Überspringen): \033[0m'" AUTH_CODE

if test -n "$AUTH_CODE"
    set TOKEN_RESPONSE (curl -s -X POST "$BASE_URL/oauth/token" \
        -H "Content-Type: application/x-www-form-urlencoded" \
        -d "grant_type=authorization_code&code=$AUTH_CODE&redirect_uri=$REDIRECT_URI&client_id=$CLIENT_ID&code_verifier=$CODE_VERIFIER")

    echo $TOKEN_RESPONSE | python3 -m json.tool

    set ACCESS_TOKEN (echo $TOKEN_RESPONSE | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('access_token',''))" 2>/dev/null)

    if test -n "$ACCESS_TOKEN"
        pass "Access Token erhalten!"

        # ============================================
        # STEP 6: MCP initialize + tools/list
        # ============================================
        header "STEP 6a: MCP initialize mit Bearer Token"

        set INIT_RESPONSE (curl -s -D /tmp/mcp_test_headers -X POST "$BASE_URL/mcp" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $ACCESS_TOKEN" \
            -d '{
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {"name": "test-script", "version": "1.0"}
                }
            }')

        echo $INIT_RESPONSE | python3 -m json.tool

        set SESSION_ID (grep -i mcp-session-id /tmp/mcp_test_headers | tr -d '\r\n' | awk '{print $2}')

        if test -n "$SESSION_ID"
            pass "Session-ID erhalten: $SESSION_ID"
        else
            fail "Keine Session-ID im Response-Header"
        end

        set PROTO_VER (echo $INIT_RESPONSE | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('result',{}).get('protocolVersion',''))" 2>/dev/null)
        if test "$PROTO_VER" = "2024-11-05"
            pass "protocolVersion: $PROTO_VER ✓"
        else
            fail "protocolVersion: $PROTO_VER (erwartet: 2024-11-05)"
        end

        header "STEP 6b: MCP tools/list mit Session-ID"

        set TOOLS_RESPONSE (curl -s -X POST "$BASE_URL/mcp" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $ACCESS_TOKEN" \
            -H "Mcp-Session-Id: $SESSION_ID" \
            -d '{
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": {}
            }')

        echo $TOOLS_RESPONSE | python3 -m json.tool

        set TOOL_COUNT (echo $TOOLS_RESPONSE | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d.get('result',{}).get('tools',[])))" 2>/dev/null)

        if test -n "$TOOL_COUNT" -a "$TOOL_COUNT" -gt 0
            pass "$TOOL_COUNT Tool(s) gefunden!"

            # Erste 5 Tools anzeigen
            echo $TOOLS_RESPONSE | python3 -c "
import sys,json
d=json.load(sys.stdin)
tools=d.get('result',{}).get('tools',[])
for t in tools[:5]:
    props=list(t.get('inputSchema',{}).get('properties',{}).keys())
    print(f'  - {t[\"name\"]}: {t.get(\"description\",\"\")} (params: {props})')
if len(tools)>5: print(f'  ... und {len(tools)-5} weitere')
"
        else
            fail "Keine Tools zurückgegeben (tools: [])"
        end

        # ============================================
        # STEP 7: MCP tools/call testen
        # ============================================
        header "STEP 7: MCP tools/call (list_projects)"

        set CALL_RESPONSE (curl -s -X POST "$BASE_URL/mcp" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $ACCESS_TOKEN" \
            -H "Mcp-Session-Id: $SESSION_ID" \
            -d '{
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "list_projects",
                    "arguments": {}
                }
            }')

        set HAS_CONTENT (echo $CALL_RESPONSE | python3 -c "import sys,json; d=json.load(sys.stdin); print('yes' if 'result' in d else 'no')" 2>/dev/null)

        if test "$HAS_CONTENT" = "yes"
            pass "tools/call erfolgreich!"
            echo $CALL_RESPONSE | python3 -c "import sys,json; d=json.load(sys.stdin); print(json.dumps(d,indent=2)[:500])"
        else
            fail "tools/call fehlgeschlagen"
            echo $CALL_RESPONSE | python3 -m json.tool
        end

    else
        fail "Kein Access Token erhalten"
        set ERROR (echo $TOKEN_RESPONSE | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('error',''), '-', d.get('error_description',''))" 2>/dev/null)
        fail "Fehler: $ERROR"
    end
else
    warn "Token-Test übersprungen"
end

# ============================================
# ZUSAMMENFASSUNG
# ============================================
header "ZUSAMMENFASSUNG"
echo ""
info "Server:       $BASE_URL"
info "Client ID:    $CLIENT_ID"
info "Auth Method:  $AUTH_METHOD (erwartet: none)"
echo ""
