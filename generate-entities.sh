#!/usr/bin/env bash
set -e

echo "=== Beat Collector Entity Generator ==="
echo ""

# Use temporary SQLite database for entity generation
TEMP_DB="sqlite:./.temp_entity_gen.db"

echo "Creating temporary SQLite database..."
touch .temp_entity_gen.db

echo "Running migrations on temporary database..."
DATABASE_URL="$TEMP_DB" cargo run -p migration up

echo "Generating entities from database schema..."
sea-orm-cli generate entity \
    --database-url "$TEMP_DB" \
    --output-dir ./src/db/entities \
    --with-serde both \
    --date-time-crate chrono \
    --with-prelude all \
    --impl-active-model-behavior

echo "Cleaning up temporary database..."
rm -f .temp_entity_gen.db

echo ""
echo "✓ Entity generation complete!"
echo ""
echo "Next steps:"
echo "  1. Run 'cargo check' to verify compilation"
echo "  2. Run './dev.sh' to start the development server"
