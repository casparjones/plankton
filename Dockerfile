# ============================================================
# Plankton – Multi-Stage Dockerfile
#
# Stage 1 (frontend-builder): Node.js baut das Webpack-Bundle
# Stage 2 (backend-builder):  Rust baut das Binary
# Stage 3 (runtime):          Minimales Debian-Image
# ============================================================

# ============================================================
# Stage 1: Frontend
# ============================================================
FROM node:22-alpine AS frontend-builder
WORKDIR /app

# package.json zuerst kopieren → npm install wird Docker-gecacht
# solange sich package.json / package-lock.json nicht ändern
COPY package.json package-lock.json ./
RUN npm ci

# Frontend-Quellen
COPY webpack.config.js tsconfig.json ./
COPY src/frontend ./src/frontend/
# Alle statischen Dateien kopieren (Icons, Styles, HTML, Splash, etc.)
COPY static ./static/

# Bundle bauen → erzeugt static/bundle.[hash].js + static/bundle.[hash].css + static/index.html
RUN npm run build

# ============================================================
# Stage 2: Backend
# Node.js wird hier nicht benötigt – build.rs erkennt dass
# bundle.js bereits existiert und überspringt npm automatisch
# ============================================================
FROM rust:1.94-slim AS backend-builder
WORKDIR /app

# System-Abhängigkeiten für reqwest (OpenSSL)
RUN apt-get update \
    && apt-get install -y pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Cargo-Dependency-Cache: erst Cargo.toml/Cargo.lock + Dummy-main
# → cargo build cached alle Abhängigkeiten in einem eigenen Layer
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main(){}' > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Echten Source + build.rs + fertiges Frontend-Bundle reinkopieren
COPY src ./src
COPY build.rs ./
# Bundle aus Stage 1 – build.rs überspringt npm da bundle.js existiert
COPY --from=frontend-builder /app/static ./static

# Timestamps aktualisieren damit Cargo den Source neu kompiliert
RUN touch src/main.rs
RUN cargo build --release

# ============================================================
# Stage 3: Runtime
# Nur das Binary + static/ – kein Rust, kein Node, kein npm
# ============================================================
FROM debian:bookworm-slim
WORKDIR /app

# Laufzeit-Abhängigkeiten für OpenSSL
RUN apt-get update \
    && apt-get install -y libssl3 ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -sf http://localhost:3000/healthz || exit 1

COPY --from=backend-builder /app/target/release/plankton /usr/local/bin/plankton
COPY --from=backend-builder /app/static ./static

ENV PORT=3000
EXPOSE 3000
CMD ["/usr/local/bin/plankton"]