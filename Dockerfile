FROM rust:1.78 as builder
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
COPY static ./static
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/plankton /usr/local/bin/plankton
COPY --from=builder /app/static ./static
ENV PORT=3000
EXPOSE 3000
CMD ["/usr/local/bin/plankton"]
