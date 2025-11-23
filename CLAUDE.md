# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Beat Collector is a self-hosted music library management system built with the MASH stack (Maud + Axum + SeaORM + HTMX). It helps users transition from Spotify to self-hosted solutions by tracking music ownership status, discovering albums, and automating downloads via Lidarr integration.

**Tech Stack:**
- Backend: Rust with Axum web framework, SeaORM for database access
- Templating: Maud (compile-time HTML templates)
- Frontend: HTMX for dynamic interactions, TailwindCSS for styling
- Database: PostgreSQL with SeaORM migrations
- Cache: Redis for API response caching
- Task Scheduling: tokio-cron-scheduler for background jobs

## Development Commands

### Initial Setup
```bash
# Copy environment variables and configure
cp .env.example .env
# Edit .env with your Spotify client ID and other settings

# Start development environment (starts PostgreSQL/Redis, builds CSS, runs app)
./dev.sh
```

### Building
```bash
# Build for production
./build.sh

# Build Rust only
cargo build --release

# Build TailwindCSS
npm run css:build
npm run css:watch  # Watch mode for development
```

### Database Migrations
```bash
# Run pending migrations (automatically done on app startup)
cargo run -- migrate up

# Create a new migration
cd migration
cargo run -- generate MIGRATION_NAME

# Migration files are in migration/src/
# Migrations run automatically on application startup via main.rs:55
```

### Running Tests
```bash
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### Development Workflow
```bash
# Development mode with auto-reload
cargo watch -x run

# Format code
cargo fmt

# Run linter
cargo clippy
```

## Architecture

### MASH Stack Pattern

This project uses the MASH stack (Maud + Axum + SeaORM + HTMX):

1. **Maud**: Compile-time HTML templating in Rust (see `src/templates/`)
2. **Axum**: Web framework for routing and handlers (see `src/handlers/`)
3. **SeaORM**: ORM for PostgreSQL with compile-time checked queries (see `src/db/`)
4. **HTMX**: Dynamic HTML updates without JavaScript framework

### Dual Route System

The application serves both HTML pages and JSON APIs:

**HTML Routes** (`src/handlers/html.rs`) - Serve Maud-rendered HTML with HTMX:
- `/` - Main album grid view
- `/settings` - Settings page
- `/jobs` - Job status page
- `/albums/:id` - Album detail (HTMX partial)

**API Routes** (`src/handlers/mod.rs`) - JSON API under `/api/`:
- `/api/albums` - CRUD operations for albums
- `/api/auth/spotify/*` - OAuth flow
- `/api/jobs/*` - Job management
- `/api/settings` - Settings management
- `/api/webhooks/lidarr` - Webhook endpoint

### State Management

`AppState` (defined in `src/state.rs`) is shared across all handlers:
- `db: DatabaseConnection` - SeaORM database connection pool
- `redis: ConnectionManager` - Redis connection for caching
- `config: Arc<Config>` - Application configuration from environment
- `job_queue: JobQueue` - Background job queue

Access state in handlers via Axum's state extractor:
```rust
async fn handler(State(state): State<AppState>) -> Result<...> {
    // Use state.db, state.redis, state.config, state.job_queue
}
```

### Service Layer Architecture

Services (in `src/services/`) handle external integrations:

- `spotify.rs` - Spotify OAuth and library import (rate-limited: 2 req/sec)
- `musicbrainz.rs` - Album matching and metadata (rate-limited: 1 req/sec)
- `lidarr.rs` - Lidarr API integration for downloads
- `cache.rs` - Redis caching helpers

### Background Job System

Jobs are managed via a queue-executor pattern:

1. **Job Queue** (`src/jobs/queue.rs`): Thread-safe job submission
2. **Job Executor** (`src/jobs/executor.rs`): Processes jobs from queue
3. **Task Modules** (`src/tasks/`):
   - `spotify_sync.rs` - Import entire Spotify library
   - `musicbrainz_match.rs` - Match albums to MusicBrainz (rate-limited)
   - `cover_art.rs` - Fetch album artwork
   - `filesystem_scan.rs` - Scan local music directory
   - `filesystem_watcher.rs` - Watch for new files

Jobs are spawned as background tasks (see `src/main.rs:72-74`).

### Database Pattern

**Entities** are defined in `src/db/entities/`:
- `artist.rs`, `album.rs`, `track.rs` - Core music metadata
- `user_settings.rs` - User configuration (Spotify tokens, Lidarr settings)
- `job.rs` - Background job tracking
- `lidarr_download.rs` - Download status tracking

**Repositories** (future pattern) go in `src/db/repositories/`:
- Create repository modules for complex queries
- Keep entity models clean and focused on schema definition

**Migrations** are in `migration/src/`:
- Versioned migrations: `m20240101_000001_create_artists_table.rs`, etc.
- New migrations: use SeaORM migration generator
- Migrations auto-run on startup

### Template System (Maud)

Templates are in `src/templates/`:
- `layout.rs` - Base HTML layout
- `pages.rs` - Full page components
- `components.rs` - Reusable UI components

Maud templates are compile-time checked Rust macros:
```rust
html! {
    div class="container" {
        h1 { "Hello, World!" }
        @if some_condition {
            p { "Conditional content" }
        }
    }
}
```

### Error Handling

Custom error type in `src/error.rs`:
- Implements `IntoResponse` for automatic HTTP error responses
- Use `anyhow::Result` for internal operations
- Convert to `AppError` at handler boundaries

## Configuration

All configuration is via environment variables (see `.env.example`):

**Required:**
- `DATABASE_URL` - PostgreSQL connection string
- `REDIS_URL` - Redis connection string
- `SPOTIFY_CLIENT_ID` - From Spotify Developer Dashboard

**Optional:**
- `LIDARR_URL`, `LIDARR_API_KEY` - Can also be set via UI
- `MUSIC_FOLDER` - Local music directory to monitor
- `RUST_LOG` - Logging level (default: info)

Config is loaded in `src/config.rs` via `dotenvy` and accessed through `AppState.config`.

## Key Patterns

### Adding a New Feature

1. **Define database schema** (if needed):
   - Create migration in `migration/src/`
   - Define entity in `src/db/entities/`

2. **Create service layer** (if external API):
   - Add service module in `src/services/`
   - Implement rate limiting if required

3. **Add handlers**:
   - JSON API: `src/handlers/[feature].rs`
   - HTML views: `src/handlers/html.rs`

4. **Update templates** (for UI):
   - Add/modify templates in `src/templates/`
   - Use HTMX attributes for interactivity

5. **Add background job** (if needed):
   - Create task in `src/tasks/`
   - Register with scheduler in `src/tasks/mod.rs`

### Rate Limiting Pattern

External APIs have strict rate limits:

**Spotify**: 2 requests/second (using `governor` crate)
**MusicBrainz**: 1 request/second (strict requirement)

Implement rate limiting in service layer:
```rust
use governor::{Quota, RateLimiter};
use nonzero_ext::nonzero;

let limiter = RateLimiter::direct(
    Quota::per_second(nonzero!(1u32))
);
limiter.until_ready().await;
// Make API call
```

### Caching Pattern

Use Redis for expensive API calls:

```rust
// Try cache first
let cache_key = format!("mb:album:{}", album_id);
if let Ok(cached) = redis.get(&cache_key).await {
    return Ok(cached);
}

// Fetch from API
let result = fetch_from_api().await?;

// Cache with TTL
redis.set_ex(&cache_key, &result, 86400).await?; // 24 hours
```

## Important Notes

- **Migrations run automatically** on app startup (main.rs:55)
- **Job executor starts on app startup** (main.rs:72-75)
- **TailwindCSS must be built** before running (or use dev.sh)
- **HTMX interactions** use `hx-get`, `hx-post` attributes in Maud templates
- **OAuth tokens** are stored encrypted in `user_settings` table
- **Cover art** is stored in `static/covers/` directory

## Docker Deployment

```bash
# Start all services
docker compose up -d

# View logs
docker compose logs -f app

# Rebuild after code changes
docker compose up -d --build app
```

The app runs on port 3000 by default.
