#!/usr/bin/env bash
set -e

echo "Building Beat Collector..."

# Check if npm is installed
if ! command -v npm &> /dev/null; then
    echo "npm is required but not installed. Please install Node.js first."
    exit 1
fi

# Install npm dependencies if needed
if [ ! -d "node_modules" ]; then
    echo "Installing npm dependencies..."
    npm install
fi

# Build CSS
echo "Building TailwindCSS..."
npm run css:build

# Create static directories if they don't exist
mkdir -p static/css
mkdir -p static/covers

# Build Rust application
echo "Building Rust application..."
cargo build --release

echo "Build complete!"
