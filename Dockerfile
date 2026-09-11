# renewal-server (PropertyPilot API + scheduler) — for Lightsail / any Linux host.
#   docker build -t propertypilot-server .
#   docker run -d --name propertypilot --env-file .env -p 8787:8787 propertypilot-server
# The client apps only need the HTTP(S) port; PostgreSQL is reached via DATABASE_URL.

FROM rust:1-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rustfmt.toml ./
COPY crates ./crates
COPY apps/server ./apps/server
# The desktop shell is a workspace member but not needed here: build only the server.
RUN sed -i 's|"apps/desktop/src-tauri",||; s|"apps/desktop/src-tauri"||' Cargo.toml \
 && cargo build --release -p renewal-server

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates tzdata curl \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --home /var/lib/propertypilot --create-home propertypilot
COPY --from=build /src/target/release/renewal-server /usr/local/bin/renewal-server
USER propertypilot
WORKDIR /var/lib/propertypilot
ENV BIND_ADDR=0.0.0.0:8787
EXPOSE 8787
HEALTHCHECK --interval=30s --timeout=5s --retries=3 CMD curl -fsS http://127.0.0.1:8787/api/health || exit 1
ENTRYPOINT ["renewal-server"]
