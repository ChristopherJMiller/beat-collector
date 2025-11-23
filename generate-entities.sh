#!/bin/bash
set -e

echo "=== Beat Collector Entity Generator ==="
echo ""

# Check if .env exists
if [ ! -f ".env" ]; then
    echo ".env file not found. Copying from .env.example..."
    cp .env.example .env
    echo "Please edit .env with your configuration"
    exit 1
fi

# Source .env to get DATABASE_URL
source .env

# Ensure database services are running
echo "Starting PostgreSQL and Redis..."
docker compose up -d postgres redis

# Wait for PostgreSQL to be ready
echo "Waiting for PostgreSQL to be ready..."
sleep 5

# Run migrations to create the schema
echo "Running database migrations..."
cargo run -- migrate up || {
    echo "Migration failed - building first..."
    cargo build
    cargo run -- migrate up
}

# Backup existing entities
echo "Backing up existing entities..."
if [ -d "src/db/entities" ]; then
    timestamp=$(date +%Y%m%d_%H%M%S)
    cp -r src/db/entities "src/db/entities.backup.$timestamp"
    echo "Backup created at: src/db/entities.backup.$timestamp"
fi

# Generate entities from database
echo ""
echo "Generating entities from database schema..."
sea-orm-cli generate entity \
    --database-url "$DATABASE_URL" \
    --output-dir ./src/db/entities \
    --with-serde both \
    --date-time-crate chrono \
    --with-prelude all \
    --impl-active-model-behavior

echo ""
echo "✓ Entity generation complete!"
echo ""
echo "Generated files are in: src/db/entities/"
echo "Old entities backed up to: src/db/entities.backup.$timestamp"
echo ""
echo "Next steps:"
echo "  1. Review the generated entities"
echo "  2. Run 'cargo check' to verify compilation"
echo "  3. Run './dev.sh' to start the development server"
