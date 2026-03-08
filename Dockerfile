# ============================================================
# Stage 1: Frontend – Node.js baut das Webpack-Bundle
# ============================================================
FROM node:22-alpine AS frontend-builder
WORKDIR /app

# Erst nur package.json kopieren → npm install wird gecacht
# solange sich package.json nicht ändert
COPY package.json package-lock.json ./
RUN npm ci

# Jetzt erst die Frontend-Quellen
COPY webpack.config.js ./
COPY static/main.js static/styles.css static/index.html ./static/
RUN npm run build

# ============================================================
# Stage 2: Backend – Rust baut das Binary
# Kein Node.js nötig, build.rs überspringt npm da bundle
# bereits von Stage 1 gebaut wurde
# ============================================================
FROM rust:1.78-slim AS backend-builder
WORKDIR /app

# System-Deps für reqwest (TLS)
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Cargo-Cache-Layer: erst nur Cargo.toml, dann Dummy-Build
# → Abhängigkeiten werden gecacht solange Cargo.toml gleich bleibt
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main(){}' > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Jetzt echten Source-Code und das fertige Frontend-Bundle reinkopieren
COPY src ./src
# Bundle aus Stage 1 holen – build.rs muss npm NICHT nochmal laufen
COPY --from=frontend-builder /app/static ./static
# build.rs braucht keine node_modules wenn bundle.js schon existiert
COPY build.rs ./
# Touch damit Cargo den Source als neuer erkennt
RUN touch src/main.rs && cargo build --release

# ============================================================
# Stage 3: Runtime – minimales Image, nur das Binary + static
# ============================================================
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=backend-builder /app/target/release/plankton /usr/local/bin/plankton
COPY --from=backend-builder /app/static ./static

ENV PORT=3000
EXPOSE 3000
CMD ["/usr/local/bin/plankton"]