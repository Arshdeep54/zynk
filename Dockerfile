# ---- Build Stage ----
FROM rust:1.82 as builder

WORKDIR /app

# Needed for tonic-build/prost-build to run protoc during build
RUN apt-get update \
  && apt-get install -y --no-install-recommends protobuf-compiler \
  && rm -rf /var/lib/apt/lists/*

# Pre-copy manifests and proto to leverage Docker layer caching
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./build.rs
COPY proto ./proto

# Create a dummy src to cache dependency build
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo build --release || true

# Now copy full source
COPY src ./src
COPY benches ./benches

# Build all binaries (zynkd, zynk_lb, zynkcli)
RUN cargo build --release

# ---- Runtime Stage ----
FROM debian:bookworm-slim

# Install minimal certificates (if TLS is enabled in the future)
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binaries
COPY --from=builder /app/target/release/zynkd /usr/local/bin/zynkd
COPY --from=builder /app/target/release/zynk_lb /usr/local/bin/zynk_lb
COPY --from=builder /app/target/release/zynk /usr/local/bin/zynk

# Data dir for the LSM engine
VOLUME ["/data"]

# Defaults (override via env)
ENV PORT=50051
ENV BIND_IP=0.0.0.0
ENV DATA_DIR=/data
ENV NODE_ID=node-unknown
ENV LB_PORT=60051
ENV LB_BIND_IP=0.0.0.0
ENV PEERS=

EXPOSE 50051 60051

# Default entrypoint is the storage node; can be overridden in Kubernetes
ENTRYPOINT ["/usr/local/bin/zynkd"]
