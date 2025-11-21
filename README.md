# Beat Collector 🎵

A self-hosted music library management system to help you transition from Spotify to self-hosted solutions like Gonic/Subsonic. Track your music collection, discover albums on Bandcamp, and automate downloads via Lidarr.

## Features

- **Spotify Integration**: Import your entire Spotify library via OAuth 2.0
- **MusicBrainz Matching**: Automatically match albums to canonical metadata
- **Visual Library**: Grid interface showing ownership status (owned/not owned/downloading)
- **Lidarr Integration**: Automate music downloads with webhooks and API integration
- **Filesystem Monitoring**: Automatically detect new music in your local folder
- **Multi-Source Tracking**: Track acquisition via Bandcamp, physical media, or Lidarr
- **Cover Art**: Automatic album artwork from Cover Art Archive

## Architecture

Built with:
- **Backend**: Rust + Axum web framework
- **Database**: PostgreSQL with SeaORM
- **Cache**: Redis for API response caching
- **Frontend**: React + TailwindCSS (coming soon)
- **Task Queue**: tokio-cron-scheduler for background jobs

## Prerequisites

- Docker and Docker Compose (recommended)
- OR manually:
  - Rust 1.75+
  - PostgreSQL 15+
  - Redis 7+
  - Node.js 20+ (for frontend)

## Quick Start with Docker

1. **Clone the repository**
   ```bash
   git clone https://github.com/yourusername/beat-collector.git
   cd beat-collector
   ```

2. **Configure environment variables**
   ```bash
   cp .env.example .env
   # Edit .env with your configuration
   ```

3. **Get Spotify API credentials**
   - Go to [Spotify Developer Dashboard](https://developer.spotify.com/dashboard)
   - Create a new app
   - Add `http://localhost:3000/auth/callback` to Redirect URIs
   - Copy Client ID to `.env`

4. **Start the application**
   ```bash
   docker-compose up -d
   ```

5. **Access the application**
   - Web UI: http://localhost:3000
   - API: http://localhost:3000/api
   - Health Check: http://localhost:3000/health

## Manual Setup

### Database Setup

```bash
# Install PostgreSQL and create database
createdb beat_collector

# Install Redis
# On macOS: brew install redis
# On Linux: apt-get install redis-server
```

### Backend Setup

```bash
# Install Rust dependencies
cargo build --release

# Run database migrations
cargo run --bin migration

# Start the server
cargo run --release
```

### Frontend Setup (Coming Soon)

```bash
cd frontend
npm install
npm run dev
```

## Configuration

### Environment Variables

See `.env.example` for all available configuration options.

**Required:**
- `DATABASE_URL`: PostgreSQL connection string
- `SPOTIFY_CLIENT_ID`: Your Spotify app client ID
- `SPOTIFY_REDIRECT_URI`: OAuth callback URL

**Optional:**
- `LIDARR_URL`: Lidarr instance URL (can be set via UI)
- `LIDARR_API_KEY`: Lidarr API key (can be set via UI)
- `MUSIC_FOLDER`: Path to your local music directory
- `REDIS_URL`: Redis connection string (default: redis://localhost:6379)

### Lidarr Setup

1. Install and configure [Lidarr](https://lidarr.audio/)
2. In Beat Collector settings, add:
   - Lidarr URL (e.g., `http://localhost:8686`)
   - Lidarr API Key (found in Lidarr Settings → General → Security)
3. Configure webhook in Lidarr:
   - Settings → Connect → Add Webhook
   - URL: `http://beat-collector:3000/api/webhooks/lidarr`
   - Events: On Grab, On Import/Upgrade, On Download Failure

## API Documentation

### Authentication

#### `GET /api/auth/spotify/authorize`
Initiate Spotify OAuth flow

Response:
```json
{
  "authorization_url": "https://accounts.spotify.com/authorize?..."
}
```

#### `POST /api/auth/spotify/callback`
Complete OAuth flow

Request:
```json
{
  "code": "authorization_code",
  "code_verifier": "verifier_from_session"
}
```

### Albums

#### `GET /api/albums`
List albums with pagination and filters

Query Parameters:
- `ownership_status`: not_owned|owned|downloading
- `match_status`: pending|matched|manual_review|no_match
- `artist_id`: Filter by artist UUID
- `search`: Search album titles
- `page`: Page number (default: 1)
- `page_size`: Items per page (default: 50, max: 200)

#### `GET /api/albums/:id`
Get album details

#### `PATCH /api/albums/:id`
Update album

Request:
```json
{
  "ownership_status": "owned",
  "acquisition_source": "bandcamp",
  "local_path": "/music/Artist/Album"
}
```

#### `POST /api/albums/:id/search-lidarr`
Trigger Lidarr search for album

### Jobs

#### `GET /api/jobs`
List recent background jobs

#### `POST /api/jobs/spotify-sync`
Trigger full Spotify library sync

#### `POST /api/jobs/musicbrainz-match-all`
Match all unmatched albums to MusicBrainz

#### `GET /api/jobs/:id/status`
Get job status and progress

### Settings

#### `GET /api/settings`
Get application settings

#### `PUT /api/settings`
Update settings

Request:
```json
{
  "lidarr_url": "http://localhost:8686",
  "lidarr_api_key": "your_api_key",
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

Response:
```json
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

## Usage Workflow

1. **Connect Spotify**
   - Click "Connect Spotify" in the UI
   - Authorize the application

2. **Sync Your Library**
   - Click "Sync Spotify Library"
   - Wait for import to complete (~2-5 min for 500 albums)

3. **Match to MusicBrainz**
   - Click "Match All Albums"
   - System will match albums at 1 req/sec (~8 min for 500 albums)
   - Review albums with low confidence scores

4. **Configure Lidarr** (Optional)
   - Add Lidarr URL and API key in Settings
   - Test connection
   - Trigger searches for wanted albums

5. **Monitor Local Folder**
   - Point to your music directory in Settings
   - System will automatically detect new music
   - Albums update to "owned" status when found

## Rate Limiting

The application respects API rate limits:

- **Spotify**: 2 requests/second (180/minute limit)
- **MusicBrainz**: 1 request/second (strict)
- **Cover Art Archive**: No limit (respectful 100ms delay)
- **Lidarr**: No enforced limit

## Development

### Project Structure

```
beat-collector/
├── src/
│   ├── main.rs              # Application entry point
│   ├── config.rs            # Configuration management
│   ├── error.rs             # Error types
│   ├── state.rs             # Application state
│   ├── db/
│   │   ├── entities/        # SeaORM entity models
│   │   └── repositories/    # Database access layer
│   ├── services/
│   │   ├── spotify.rs       # Spotify API client
│   │   ├── musicbrainz.rs   # MusicBrainz client
│   │   ├── lidarr.rs        # Lidarr API client
│   │   └── cache.rs         # Redis caching
│   ├── handlers/            # HTTP request handlers
│   └── tasks/               # Background job workers
├── migration/               # Database migrations
├── frontend/                # React frontend (coming soon)
├── Dockerfile
├── docker-compose.yml
└── DESIGN.md               # Detailed design document
```

### Running Tests

```bash
cargo test
```

### Database Migrations

```bash
# Create a new migration
cargo run --bin migration generate MIGRATION_NAME

# Apply migrations
cargo run --bin migration up

# Rollback last migration
cargo run --bin migration down
```

## Roadmap

- [ ] Complete React frontend with album grid
- [ ] Filesystem monitoring with automatic matching
- [ ] Cover art download and local storage
- [ ] Playlist import from Spotify
- [ ] Advanced search and filtering
- [ ] Export/import library data
- [ ] Multi-user support
- [ ] Mobile app (React Native)
- [ ] Integration with Gonic/Subsonic for play stats

## Contributing

Contributions are welcome! Please open an issue or PR.

## License

MIT License - see LICENSE file for details

## Acknowledgments

- [Spotify Web API](https://developer.spotify.com/documentation/web-api/)
- [MusicBrainz](https://musicbrainz.org/)
- [Cover Art Archive](https://coverartarchive.org/)
- [Lidarr](https://lidarr.audio/)
- [SeaORM](https://www.sea-ql.org/SeaORM/)
- [Axum](https://github.com/tokio-rs/axum)

## Support

For issues, questions, or feature requests, please open a GitHub issue.

---

**Note**: This is alpha software. Backup your data and use at your own risk.
