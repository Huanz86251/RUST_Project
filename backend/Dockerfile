# ---------- Build stage ----------
FROM rust:bookworm AS builder
WORKDIR /app

# Install native dependencies commonly needed by Rust crates (e.g., OpenSSL)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Copy manifest files first to leverage Docker layer caching
COPY Cargo.toml Cargo.lock ./

# Copy the source code
COPY src ./src

# If you use sqlx::query! macros, you may want offline mode:
# ENV SQLX_OFFLINE=true
# COPY sqlx-data.json ./sqlx-data.json

# Build the release binary
RUN cargo build --release

# ---------- Runtime stage ----------
FROM debian:bookworm-slim
WORKDIR /app

# Install CA certificates for HTTPS/TLS
RUN apt-get update && apt-get install -y ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Run as a non-root user for better security
RUN useradd -m appuser
USER appuser

# Copy the compiled binary from the build stage
COPY --from=builder /app/target/release/backend /app/backend

# Expose the port your Axum app listens on
EXPOSE 8080

# Start the server
CMD ["/app/backend"]
