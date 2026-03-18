#!/bin/bash

PROMPT="Lies /vibe/readme.md vollständig durch, falls es keine gibt les die /vibe/init.md und erstelle daraus eine /vibe/readme.md. Analysiere danach die gesamte Codebasis (src/main.rs, static/main.js, Cargo.toml, package.json). Prüfe /vibe/ideas/ auf neue Ideen. Starte den Agenten-Workflow als Supervisor - vollständig autonom ohne Rückfragen."

/home/frank/.local/bin/claude --verbose --dangerously-skip-permissions "$PROMPT"