ARG RUST_VERSION=1.85.0
ARG APP_NAME=agent

# Build Stage
FROM rust:${RUST_VERSION}-slim-bookworm AS build

ARG APP_NAME
WORKDIR /app

# Install build dependencies for Debian
RUN apt-get update && apt-get install -y \
    pkg-config \
    libx11-dev \
    libxtst-dev \
    libxext-dev \
    libxi-dev \
    libxinerama-dev \
    libxfixes-dev \
    libxkbcommon-dev \
    libxcb1-dev \
    libxcb-shm0-dev \
    libxcb-randr0-dev \
    libxdo-dev \
    libxau-dev \
    libxdmcp-dev \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy local source instead of cloning (to avoid private repo issues)
COPY . .

WORKDIR /app/$APP_NAME

RUN cargo build --locked --release && \
    cp /app/target/release/$APP_NAME /usr/local/bin/server

# Final Stage
FROM debian:bookworm-slim AS final

# Re-add user for security
RUN useradd -m -u 1000 user

# Install runtime dependencies for Debian + Xvfb
RUN apt-get update && apt-get install -y \
    xvfb \
    xauth \
    x11-utils \
    libx11-6 \
    libxtst6 \
    libxext6 \
    libxi6 \
    libxinerama1 \
    libxfixes3 \
    libxkbcommon0 \
    libxcb1 \
    libxcb-shm0 \
    libxcb-randr0 \
    libxdo3 \
    libxau6 \
    libxdmcp6 \
    ca-certificates \
    openssl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=build /usr/local/bin/server /usr/local/bin/server

# Copy entrypoint script
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Run as non-privileged user
USER user

# Virtual display setup
ENV DISPLAY=:99

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
