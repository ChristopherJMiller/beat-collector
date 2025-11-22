# Frontend build stage - Build TailwindCSS
FROM node:20-alpine as frontend-builder
WORKDIR /app
COPY package.json package-lock.json* ./
RUN npm install --production=false
COPY tailwind.config.js ./
COPY styles ./styles
COPY src ./src
RUN npm run css:prod

# Build stage - Chef prepare
FROM rust:1.75-slim as chef
WORKDIR /app
RUN cargo install cargo-chef
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Chef planner
FROM chef as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Chef cook (build dependencies)
FROM chef as cacher
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
FROM chef as backend-builder
COPY . .
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN cargo build --release --bin beat-collector

# Runtime stage
FROM debian:bookworm-slim
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=backend-builder /app/target/release/beat-collector /app/beat-collector

# Copy static assets (CSS and directory structure)
COPY --from=frontend-builder /app/static /app/static

# Create directories for covers and music
RUN mkdir -p /app/static/covers /music

# Expose port
EXPOSE 3000

# Run the application
CMD ["/app/beat-collector"]
