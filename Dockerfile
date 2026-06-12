# syntax=docker/dockerfile:1

# ---- build stage -----------------------------------------------------------
FROM rust:1.86-bookworm AS builder
WORKDIR /app

# Prime dependency cache: copy manifests, build dummy target, then real sources.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && echo 'fn main() {}' > src/main.rs \
    && echo '' > src/lib.rs \
    && cargo build --release --locked --quiet; \
       rm -rf src

COPY . .
RUN touch src/main.rs src/lib.rs \
    && cargo build --release --locked \
    && strip target/release/lineagent

# ---- runtime stage ---------------------------------------------------------
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home --home-dir /home/lineagent lineagent \
    && mkdir -p /data && chown lineagent:lineagent /data

COPY --from=builder /app/target/release/lineagent /usr/local/bin/lineagent

USER lineagent
ENV LINEAGENT_HOST=0.0.0.0 \
    LINEAGENT_PORT=3000 \
    LINEAGENT_DATA_DIR=/data \
    LINEAGENT_LOG=info
VOLUME ["/data"]
EXPOSE 3000

HEALTHCHECK --interval=15s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -fsS http://127.0.0.1:${LINEAGENT_PORT:-3000}/healthz || exit 1

ENTRYPOINT ["lineagent"]
CMD ["serve"]
