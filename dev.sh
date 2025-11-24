#!/usr/bin/env bash
set -e

echo "Starting Beat Collector in development mode..."

# Check if .env exists
if [ ! -f ".env" ]; then
    echo ".env file not found. Copying from .env.example..."
    cp .env.example .env
    echo "Please edit .env with your configuration"
    exit 1
fi

# Start infrastructure services
echo "Starting PostgreSQL and Redis..."
docker compose up -d postgres redis

# Wait for services to be healthy
echo "Waiting for services to be ready..."
sleep 3

# Install npm dependencies if needed
if [ ! -d "node_modules" ]; then
    echo "Installing npm dependencies..."
    npm install
fi

# Build CSS in watch mode (background)
echo "Starting TailwindCSS watch mode..."
npm run css:watch &
CSS_PID=$!

# Trap to kill the CSS watcher on script exit
trap "kill $CSS_PID 2>/dev/null" EXIT

# Run the application with auto-reload
echo "Application will be available at http://localhost:3000"
echo ""

cargo watch -x run

# This will run when cargo watch exits
kill $CSS_PID 2>/dev/null || true
