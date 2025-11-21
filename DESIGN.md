# Beat Collector - Design Document

## Executive Summary

Beat Collector is a self-hosted music library management system that helps users transition from Spotify to self-hosted solutions (Gonic/Subsonic). It provides a unified interface to track music ownership status, discover albums on Bandcamp, and automate downloads via Lidarr.

**Core Features:**
- Import entire Spotify library via OAuth
- Match albums to canonical MusicBrainz metadata
- Visual grid interface showing ownership status
- Lidarr integration for automated downloads
- Filesystem monitoring for local music collection
- Track acquisition sources (Bandcamp, physical media, Lidarr)

---

## Architecture Overview

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Client Layer                             │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  React SPA + TailwindCSS                             │   │
│  │  - Album Grid View                                   │   │
│  │  - Search & Filters                                  │   │
│  │  - Acquisition Management                            │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            │ REST API
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   API Layer (Axum)                           │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  HTTP Handlers                                       │   │
│  │  - Album CRUD                                        │   │
│  │  - Spotify OAuth Flow                                │   │
│  │  - Search & Filters                                  │   │
│  │  - Job Management                                    │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   Service Layer                              │
│  ┌──────────────┬──────────────┬──────────────┬─────────┐   │
│  │ Spotify      │ MusicBrainz  │ Lidarr       │ File    │   │
│  │ Service      │ Service      │ Service      │ Monitor │   │
│  └──────────────┴──────────────┴──────────────┴─────────┘   │
└─────────────────────────────────────────────────────────────┘
                            │
          ┌─────────────────┼─────────────────┐
          ▼                 ▼                 ▼
┌──────────────────┐ ┌──────────────┐ ┌──────────────────┐
│  Task Queue      │ │   Cache      │ │   Database       │
│  (tokio-cron)    │ │   (Redis)    │ │  (PostgreSQL)    │
│                  │ │              │ │   + SeaORM       │
└──────────────────┘ └──────────────┘ └──────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────┐
│               Background Workers                             │
│  - Spotify Library Sync                                      │
│  - MusicBrainz Matching (rate-limited)                       │
│  - Lidarr Status Polling                                     │
│  - Filesystem Watcher                                        │
│  - Cover Art Fetching                                        │
└─────────────────────────────────────────────────────────────┘
```

### Layered Architecture Pattern

**1. Presentation Layer (React + TailwindCSS)**
- Single Page Application
- Component-based UI
- State management via React hooks/context
- Responsive grid layout for albums

**2. API Layer (Axum)**
- RESTful endpoints
- JWT authentication
- Request validation
- Error handling middleware
- CORS configuration

**3. Service Layer (Business Logic)**
- External API clients (Spotify, MusicBrainz, Lidarr)
- Rate limiting enforcement
- Data transformation
- Caching strategies

**4. Data Access Layer (SeaORM)**
- Repository pattern
- Query builders
- Migration management
- Connection pooling

**5. Infrastructure Layer**
- Task scheduling (tokio-cron-scheduler)
- Redis caching
- File system monitoring (notify)
- Logging (tracing)

---

## Technology Stack

### Backend
- **Language**: Rust 1.75+
- **Web Framework**: Axum 0.7
- **ORM**: SeaORM 0.12 with sea-orm-migration
- **Database**: PostgreSQL 15+
- **Cache**: Redis 7+
- **Task Scheduling**: tokio-cron-scheduler 0.10
- **File Watching**: notify 6.1
- **HTTP Client**: reqwest 0.11
- **Serialization**: serde + serde_json
- **Logging**: tracing + tracing-subscriber

### Frontend
- **Framework**: React 18
- **CSS**: TailwindCSS 3.4
- **Build Tool**: Vite 5
- **HTTP Client**: axios
- **State Management**: React Query + Context API
- **Icons**: lucide-react

### External APIs
- **Spotify Web API**: OAuth 2.0 with PKCE
- **MusicBrainz API**: v2 with rate limiting (1 req/sec)
- **Cover Art Archive**: Unlimited
- **Lidarr API**: v1 with webhooks

### Infrastructure
- **Containerization**: Docker + docker-compose
- **Orchestration**: Kubernetes-ready
- **Reverse Proxy**: Configurable (Traefik/Nginx)

---

## Database Schema (SeaORM Entities)

### Core Tables

#### `artists`
```sql
CREATE TABLE artists (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(500) NOT NULL,
    spotify_id VARCHAR(100) UNIQUE,
    musicbrainz_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_artists_spotify_id ON artists(spotify_id);
CREATE INDEX idx_artists_musicbrainz_id ON artists(musicbrainz_id);
```

#### `albums`
```sql
CREATE TABLE albums (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(500) NOT NULL,
    artist_id UUID NOT NULL REFERENCES artists(id) ON DELETE CASCADE,

    -- External IDs
    spotify_id VARCHAR(100) UNIQUE,
    musicbrainz_release_group_id UUID,

    -- Metadata
    release_date DATE,
    total_tracks INTEGER,
    cover_art_url TEXT,
    genres TEXT[], -- Array of genre strings

    -- Ownership status
    ownership_status VARCHAR(20) NOT NULL DEFAULT 'not_owned',
        -- 'not_owned', 'owned', 'downloading'
    acquisition_source VARCHAR(20),
        -- 'bandcamp', 'physical', 'lidarr', 'unknown'
    local_path TEXT, -- File system path if owned

    -- Match confidence
    match_score INTEGER, -- 0-100 from MusicBrainz search
    match_status VARCHAR(20) DEFAULT 'pending',
        -- 'pending', 'matched', 'manual_review', 'no_match'

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_synced_at TIMESTAMPTZ
);

CREATE INDEX idx_albums_artist_id ON albums(artist_id);
CREATE INDEX idx_albums_spotify_id ON albums(spotify_id);
CREATE INDEX idx_albums_musicbrainz_id ON albums(musicbrainz_release_group_id);
CREATE INDEX idx_albums_ownership_status ON albums(ownership_status);
CREATE INDEX idx_albums_match_status ON albums(match_status);
```

#### `tracks`
```sql
CREATE TABLE tracks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    album_id UUID NOT NULL REFERENCES albums(id) ON DELETE CASCADE,

    title VARCHAR(500) NOT NULL,
    track_number INTEGER,
    disc_number INTEGER DEFAULT 1,
    duration_ms INTEGER,

    spotify_id VARCHAR(100),
    musicbrainz_id UUID,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tracks_album_id ON tracks(album_id);
CREATE INDEX idx_tracks_spotify_id ON tracks(spotify_id);
```

#### `user_settings`
```sql
CREATE TABLE user_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Spotify OAuth
    spotify_access_token TEXT,
    spotify_refresh_token TEXT,
    spotify_token_expires_at TIMESTAMPTZ,

    -- Lidarr Configuration
    lidarr_url VARCHAR(500),
    lidarr_api_key VARCHAR(100),

    -- Local Music Directory
    music_folder_path TEXT,

    -- Preferences
    auto_sync_enabled BOOLEAN DEFAULT false,
    sync_interval_hours INTEGER DEFAULT 24,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

#### `jobs`
```sql
CREATE TABLE jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_type VARCHAR(50) NOT NULL,
        -- 'spotify_sync', 'musicbrainz_match', 'lidarr_search',
        -- 'cover_art_fetch', 'filesystem_scan'

    status VARCHAR(20) NOT NULL DEFAULT 'pending',
        -- 'pending', 'running', 'completed', 'failed'

    entity_id UUID, -- Related album/artist ID

    progress INTEGER DEFAULT 0, -- 0-100
    total_items INTEGER,
    processed_items INTEGER DEFAULT 0,

    error_message TEXT,

    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_jobs_status ON jobs(status);
CREATE INDEX idx_jobs_type ON jobs(job_type);
CREATE INDEX idx_jobs_created_at ON jobs(created_at DESC);
```

#### `lidarr_downloads`
```sql
CREATE TABLE lidarr_downloads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    album_id UUID NOT NULL REFERENCES albums(id) ON DELETE CASCADE,

    lidarr_album_id INTEGER,
    download_id VARCHAR(100),

    status VARCHAR(20) NOT NULL DEFAULT 'pending',
        -- 'pending', 'searching', 'downloading', 'completed', 'failed'

    quality_profile VARCHAR(50),
    estimated_completion_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_lidarr_downloads_album_id ON lidarr_downloads(album_id);
CREATE INDEX idx_lidarr_downloads_status ON lidarr_downloads(status);
```

### Migration Strategy

Use SeaORM's migration system with versioned migrations:
- `m20240101_000001_create_artists_table.rs`
- `m20240101_000002_create_albums_table.rs`
- `m20240101_000003_create_tracks_table.rs`
- `m20240101_000004_create_user_settings_table.rs`
- `m20240101_000005_create_jobs_table.rs`
- `m20240101_000006_create_lidarr_downloads_table.rs`

---

## API Design

### Authentication Endpoints

#### `POST /api/auth/spotify/authorize`
Initiate Spotify OAuth flow
```json
Response:
{
  "authorization_url": "https://accounts.spotify.com/authorize?...",
  "code_verifier": "stored_in_session"
}
```

#### `POST /api/auth/spotify/callback`
Handle OAuth callback
```json
Request:
{
  "code": "authorization_code",
  "state": "csrf_token"
}

Response:
{
  "success": true,
  "expires_at": "2024-11-21T23:00:00Z"
}
```

### Album Management

#### `GET /api/albums`
List all albums with filters
```
Query params:
- ownership_status: not_owned|owned|downloading
- match_status: pending|matched|manual_review|no_match
- artist_id: UUID
- search: string (search title/artist)
- page: integer (default 1)
- page_size: integer (default 50, max 200)

Response:
{
  "albums": [
    {
      "id": "uuid",
      "title": "OK Computer",
      "artist": {
        "id": "uuid",
        "name": "Radiohead"
      },
      "cover_art_url": "https://...",
      "release_date": "1997-05-21",
      "ownership_status": "not_owned",
      "match_score": 95,
      "genres": ["Alternative Rock", "Art Rock"]
    }
  ],
  "pagination": {
    "page": 1,
    "page_size": 50,
    "total_items": 523,
    "total_pages": 11
  }
}
```

#### `GET /api/albums/:id`
Get album details

#### `PATCH /api/albums/:id`
Update album (ownership status, local path, manual match)
```json
Request:
{
  "ownership_status": "owned",
  "acquisition_source": "bandcamp",
  "local_path": "/music/Radiohead/OK Computer"
}
```

#### `POST /api/albums/:id/match`
Manually trigger MusicBrainz matching

#### `POST /api/albums/:id/search-lidarr`
Trigger Lidarr search for album

### Job Management

#### `GET /api/jobs`
List recent jobs with status

#### `POST /api/jobs/spotify-sync`
Trigger full Spotify library sync
```json
Response:
{
  "job_id": "uuid",
  "status": "pending"
}
```

#### `POST /api/jobs/musicbrainz-match-all`
Match all unmatched albums (rate-limited)

#### `GET /api/jobs/:id/status`
Poll job status
```json
Response:
{
  "id": "uuid",
  "job_type": "spotify_sync",
  "status": "running",
  "progress": 67,
  "processed_items": 234,
  "total_items": 350,
  "started_at": "2024-11-21T22:00:00Z"
}
```

### Settings

#### `GET /api/settings`
Get user settings

#### `PUT /api/settings`
Update settings
```json
Request:
{
  "lidarr_url": "http://localhost:8686",
  "lidarr_api_key": "abc123",
  "music_folder_path": "/music",
  "auto_sync_enabled": true,
  "sync_interval_hours": 12
}
```

#### `POST /api/settings/test-lidarr`
Test Lidarr connection

### Statistics

#### `GET /api/stats`
Get library statistics
```json
Response:
{
  "total_albums": 523,
  "owned_albums": 178,
  "not_owned_albums": 340,
  "downloading_albums": 5,
  "matched_albums": 500,
  "unmatched_albums": 23,
  "total_artists": 142
}
```

---

## Service Layer Details

### Spotify Service

**Responsibilities:**
- OAuth 2.0 flow implementation (PKCE)
- Token management (refresh before expiry)
- Rate limiting (2 req/sec with governor crate)
- Fetch saved albums, tracks, playlists
- Pagination handling

**Key Methods:**
```rust
async fn authorize_url() -> Result<(String, String)>
async fn exchange_code(code: &str, verifier: &str) -> Result<TokenResponse>
async fn refresh_token(refresh_token: &str) -> Result<TokenResponse>
async fn fetch_saved_albums(access_token: &str) -> Result<Vec<SpotifyAlbum>>
async fn fetch_saved_tracks(access_token: &str) -> Result<Vec<SpotifyTrack>>
async fn fetch_playlists(access_token: &str) -> Result<Vec<SpotifyPlaylist>>
```

**Caching Strategy:**
- Cache album/track data in Redis (TTL: 24 hours)
- Store tokens encrypted in database
- Use `keyring` crate for sensitive token storage in production

### MusicBrainz Service

**Responsibilities:**
- Rate limiting (1 req/sec strict enforcement)
- Lucene query construction
- Progressive refinement matching
- Score-based filtering (≥80 threshold)
- Cover art fetching from Cover Art Archive

**Key Methods:**
```rust
async fn search_release_group(artist: &str, album: &str) -> Result<Vec<MBMatch>>
async fn get_release_group_details(mbid: Uuid) -> Result<ReleaseGroup>
async fn fetch_cover_art(mbid: Uuid, size: CoverSize) -> Result<Vec<u8>>
```

**Matching Algorithm:**
1. Exact phrase match: `artist:"Name" AND releasegroup:"Title" AND primarytype:album`
2. If score < 80, try fuzzy: `artist:Name~ AND releasegroup:Title~`
3. Normalize artist names (remove "The", featuring artists)
4. Handle special characters (& vs and)
5. Return highest scoring match

**Caching:**
- Cache successful matches indefinitely in Redis
- Cache "no match" results for 7 days
- Cache cover art as files (avoid repeated downloads)

### Lidarr Service

**Responsibilities:**
- API key authentication
- Artist/album management
- Search triggering
- Queue monitoring
- Webhook registration and handling

**Key Methods:**
```rust
async fn test_connection(url: &str, api_key: &str) -> Result<bool>
async fn search_album(album_id: i32) -> Result<CommandResponse>
async fn get_queue() -> Result<Vec<QueueItem>>
async fn handle_webhook(event: LidarrWebhook) -> Result<()>
```

**Webhook Handling:**
- On Import: Update album ownership_status to "owned", set local_path
- On Grab: Update lidarr_downloads status to "downloading"
- On Failure: Update status, log error_message
- On Delete: Update ownership_status back to "not_owned"

### File Monitor Service

**Responsibilities:**
- Watch music directory for new files
- Parse folder structure (<Artist>/<Album>)
- Match files to existing albums
- Update ownership status automatically

**Key Methods:**
```rust
async fn watch_directory(path: &Path) -> Result<()>
async fn handle_new_file(path: &Path) -> Result<()>
async fn parse_music_metadata(path: &Path) -> Result<MusicMetadata>
async fn match_to_album(metadata: MusicMetadata) -> Result<Option<Uuid>>
```

**Implementation:**
- Use `notify` crate with debouncing (wait 5 seconds after last change)
- Parse ID3 tags with `id3` crate
- Fuzzy match artist/album names to database
- Create manual review job if no match found

---

## Task Queue System

### Architecture

Use **tokio-cron-scheduler** for background job processing with these job types:

#### Job Types

**1. Spotify Sync Job**
- Triggered: Manually or on schedule (configurable interval)
- Duration: ~2-5 minutes for 500 albums
- Process:
  1. Fetch all saved albums/tracks/playlists
  2. Create/update artist records
  3. Create/update album records
  4. Create/update track records
  5. Queue MusicBrainz matching jobs

**2. MusicBrainz Match Job**
- Triggered: After Spotify sync, manually, or for new albums
- Duration: Rate-limited to 1/sec = ~8 minutes for 500 albums
- Process:
  1. Select unmatched albums (match_status = 'pending')
  2. For each album:
     - Search MusicBrainz API
     - Update match_score and musicbrainz_release_group_id
     - Update match_status based on score threshold
     - Queue cover art fetch if successful
  3. Respect rate limit strictly

**3. Cover Art Fetch Job**
- Triggered: After successful MusicBrainz match
- Duration: Fast (no rate limit)
- Process:
  1. Fetch 500px cover art from Cover Art Archive
  2. Store in local filesystem or object storage
  3. Update cover_art_url in database

**4. Lidarr Status Poll Job**
- Triggered: Every 30 seconds when downloads active
- Duration: <1 second
- Process:
  1. Fetch queue from Lidarr API
  2. Update lidarr_downloads table
  3. Update album ownership_status if completed

**5. Filesystem Scan Job**
- Triggered: On startup, manually, or on schedule (daily)
- Duration: Varies by library size
- Process:
  1. Recursively scan music directory
  2. Parse folder structure
  3. Match to albums in database
  4. Update ownership_status and local_path

### Job State Management

Store job state in `jobs` table:
- Update progress percentage in real-time
- Log errors for failed jobs
- Implement retry logic (3 attempts with exponential backoff)
- Prevent duplicate jobs (check for running jobs of same type)

### Job Scheduling

```rust
// Example cron expressions
// Spotify sync: every 12 hours (if auto_sync_enabled)
scheduler.add(Job::new_async("0 0 */12 * * *", |_uuid, _lock| {
    Box::pin(async move {
        spotify_sync_job().await
    })
})?);

// Filesystem scan: daily at 2 AM
scheduler.add(Job::new_async("0 0 2 * * *", |_uuid, _lock| {
    Box::pin(async move {
        filesystem_scan_job().await
    })
})?);

// Lidarr poll: every 30 seconds if downloads active
scheduler.add(Job::new_async("*/30 * * * * *", |_uuid, _lock| {
    Box::pin(async move {
        lidarr_poll_job().await
    })
})?);
```

---

## Frontend Architecture

### Component Structure

```
frontend/
├── src/
│   ├── components/
│   │   ├── AlbumGrid.tsx          # Main grid view
│   │   ├── AlbumCard.tsx          # Individual album card
│   │   ├── AlbumDetail.tsx        # Detail modal
│   │   ├── FilterBar.tsx          # Status/genre filters
│   │   ├── SearchBar.tsx          # Search input
│   │   ├── JobStatus.tsx          # Job progress indicator
│   │   ├── Settings.tsx           # Settings page
│   │   └── Layout.tsx             # App layout
│   ├── api/
│   │   └── client.ts              # Axios API client
│   ├── hooks/
│   │   ├── useAlbums.ts           # React Query hook
│   │   ├── useJobs.ts             # Job polling hook
│   │   └── useSettings.ts         # Settings hook
│   ├── types/
│   │   └── api.ts                 # TypeScript types
│   ├── App.tsx
│   └── main.tsx
├── tailwind.config.js
├── vite.config.ts
└── package.json
```

### Album Card States

Visual representation:
- **Not Owned**: Greyscale cover art, dim opacity
- **Owned**: Full color, bright
- **Downloading**: Pulsing border, progress indicator
- **Needs Manual Match**: Yellow border indicator

### TailwindCSS Configuration

```javascript
// tailwind.config.js
module.exports = {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        primary: '#1db954',    // Spotify green
        owned: '#22c55e',      // Green
        notOwned: '#6b7280',   // Gray
        downloading: '#3b82f6', // Blue
        warning: '#eab308'     // Yellow
      }
    }
  }
}
```

---

## Deployment Strategy

### Docker Configuration

#### Multi-stage Dockerfile

```dockerfile
# Build stage
FROM rust:1.75-slim as builder
WORKDIR /app
RUN cargo install cargo-chef

# Chef prepare
FROM builder as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Chef cook (dependencies)
FROM builder as cacher
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
FROM builder as backend-builder
COPY . .
COPY --from=cacher /app/target target
RUN cargo build --release --bin beat-collector

# Frontend build
FROM node:20-alpine as frontend-builder
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ .
RUN npm run build

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=backend-builder /app/target/release/beat-collector /app/
COPY --from=frontend-builder /app/frontend/dist /app/static

EXPOSE 3000
CMD ["/app/beat-collector"]
```

#### docker-compose.yml

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:15-alpine
    environment:
      POSTGRES_DB: beat_collector
      POSTGRES_USER: beat_user
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U beat_user"]
      interval: 10s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    command: redis-server --appendonly yes
    volumes:
      - redis_data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 5s
      retries: 5

  app:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "3000:3000"
    environment:
      DATABASE_URL: postgresql://beat_user:${DB_PASSWORD}@postgres/beat_collector
      REDIS_URL: redis://redis:6379
      RUST_LOG: info
    volumes:
      - ${MUSIC_FOLDER}:/music:ro
      - cover_art:/app/cover_art
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy

volumes:
  postgres_data:
  redis_data:
  cover_art:
```

### Environment Configuration

```.env
# Database
DATABASE_URL=postgresql://beat_user:changeme@localhost/beat_collector

# Redis
REDIS_URL=redis://localhost:6379

# Spotify OAuth
SPOTIFY_CLIENT_ID=your_client_id
SPOTIFY_REDIRECT_URI=http://localhost:3000/auth/callback

# Lidarr (optional, can be set via UI)
LIDARR_URL=http://localhost:8686
LIDARR_API_KEY=your_api_key

# Music folder
MUSIC_FOLDER=/path/to/music

# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=3000

# Logging
RUST_LOG=beat_collector=debug,info
```

---

## Implementation Phases

### Phase 1: Foundation (Week 1)
- ✅ Project structure and workspace setup
- ✅ Database schema and migrations
- ✅ Axum server with basic routing
- ✅ SeaORM configuration
- ✅ Redis connection
- ✅ Basic React frontend with Vite

### Phase 2: Core Services (Week 2)
- ✅ Spotify OAuth flow
- ✅ Spotify library import
- ✅ MusicBrainz search and matching
- ✅ Cover Art Archive integration
- ✅ Task queue system

### Phase 3: Lidarr & Filesystem (Week 3)
- ✅ Lidarr API client
- ✅ Webhook handling
- ✅ Filesystem monitoring
- ✅ Automatic ownership detection

### Phase 4: Frontend (Week 4)
- ✅ Album grid component
- ✅ Filters and search
- ✅ Album detail modal
- ✅ Settings page
- ✅ Job status indicators

### Phase 5: Polish (Week 5)
- ✅ Docker configuration
- ✅ Documentation
- ✅ Error handling improvements
- ✅ Performance optimization
- ✅ Testing

---

## Security Considerations

1. **Token Storage**: Encrypt refresh tokens at rest
2. **API Keys**: Store in environment variables, never commit
3. **CORS**: Restrict to known origins in production
4. **Rate Limiting**: Implement per-IP rate limiting on API
5. **Input Validation**: Sanitize all user inputs
6. **SQL Injection**: SeaORM provides protection via parameterized queries
7. **File Access**: Validate paths to prevent directory traversal
8. **HTTPS**: Require HTTPS in production (reverse proxy)

---

## Performance Optimization

1. **Database Indexing**: Strategic indexes on foreign keys and query columns
2. **Connection Pooling**: SeaORM's built-in connection pool
3. **Redis Caching**: Cache expensive API calls (MusicBrainz, Spotify)
4. **Pagination**: Limit result sets to prevent memory issues
5. **Lazy Loading**: Load cover art on demand in frontend
6. **CDN**: Serve static assets via CDN in production
7. **Compression**: Gzip/Brotli compression for API responses

---

## Monitoring & Observability

1. **Logging**: Structured logging with `tracing`
2. **Metrics**: Track API response times, job durations
3. **Health Checks**: `/health` endpoint for container orchestration
4. **Job Monitoring**: Dashboard showing active/failed jobs
5. **Error Tracking**: Integrate Sentry or similar (optional)

---

## Future Enhancements

- **Multi-user Support**: Add user accounts and authentication
- **Playlist Management**: Import and track Spotify playlists
- **Mobile App**: React Native app
- **Advanced Matching**: ML-based album matching for difficult cases
- **Listening Statistics**: Track play counts from Gonic
- **Bandcamp Integration**: Unofficial search (user-directed)
- **Export/Import**: Backup library data
- **Recommendation Engine**: Suggest new music based on owned albums

---

## Appendix: Crate Dependencies

### Backend (Cargo.toml)

```toml
[workspace]
members = [".", "migration"]

[package]
name = "beat-collector"
version = "0.1.0"
edition = "2021"

[dependencies]
# Web framework
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "cors", "trace"] }
tokio = { version = "1", features = ["full"] }

# Database
sea-orm = { version = "0.12", features = ["sqlx-postgres", "runtime-tokio-native-tls", "macros"] }

# Redis
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }

# HTTP client
reqwest = { version = "0.11", features = ["json"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Rate limiting
governor = "0.6"

# Scheduling
tokio-cron-scheduler = "0.10"

# File watching
notify = "6"
notify-debouncer-full = "0.3"

# Music metadata
id3 = "1.13"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Error handling
anyhow = "1"
thiserror = "1"

# Utilities
uuid = { version = "1", features = ["serde", "v4"] }
chrono = { version = "0.4", features = ["serde"] }
dotenv = "0.15"

# Crypto
sha2 = "0.10"
base64 = "0.21"
rand = "0.8"

# Optional: Token encryption
keyring = { version = "2", optional = true }

[features]
default = []
secure-tokens = ["keyring"]
```

### Frontend (package.json)

```json
{
  "name": "beat-collector-frontend",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "axios": "^1.6.0",
    "@tanstack/react-query": "^5.0.0",
    "react-router-dom": "^6.20.0",
    "lucide-react": "^0.294.0"
  },
  "devDependencies": {
    "@types/react": "^18.2.0",
    "@types/react-dom": "^18.2.0",
    "@vitejs/plugin-react": "^4.2.0",
    "typescript": "^5.3.0",
    "vite": "^5.0.0",
    "tailwindcss": "^3.4.0",
    "autoprefixer": "^10.4.16",
    "postcss": "^8.4.32"
  }
}
```

---

**Document Version**: 1.0
**Last Updated**: 2024-11-21
**Author**: System Architecture Team
