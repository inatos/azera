# --- Stage 1: The Forge (Builder) ---
FROM rust:latest as builder

# 1. Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 2. Cache Dependencies Strategy
RUN mkdir src && echo "fn main() {}" > src/main.rs
# Only copy the manifest, not the lockfile (it doesn't exist yet)
COPY Cargo.toml ./
# Copy build.rs so the build script compiles
COPY build.rs ./

# Build dependencies only (this generates a fresh Cargo.lock inside)
RUN cargo build --release
RUN rm src/main.rs

# 4. Build the Actual Application
COPY src ./src
# Touch the main file to force a rebuild of the application logic
RUN touch src/main.rs
RUN cargo build --release

# --- Stage 2: The Vessel (Runtime) ---
FROM debian:bookworm-slim

# Install runtime libs (SSL, CA certs, curl for healthchecks)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 5. Copy the Artifacts
COPY --from=builder /app/target/release/azera_core .
COPY entrypoint.sh .

# Make entrypoint executable
RUN chmod +x entrypoint.sh

# Create directory for logs and artifacts if they don't exist
RUN mkdir -p athanor logs atelier

# 6. Environment & Entry
ENV RUST_LOG=info
EXPOSE 3000

ENTRYPOINT ["./entrypoint.sh"]