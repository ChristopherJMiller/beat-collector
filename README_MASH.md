# MASH Stack Implementation Notes

## What is the MASH Stack?

**MASH** = **M**aud + **A**xum + **S**eaORM + **H**TMX (+ TailwindCSS)

This is a full-stack Rust web development stack that provides:
- **Type safety from database to HTML** - Everything in Rust
- **Server-side rendering** - No JavaScript build process needed
- **Progressive enhancement** - Works without JS, enhanced with HTMX
- **Single binary deployment** - No separate frontend/backend

## Architecture

```
┌─────────────────────────────────────────┐
│          Browser (HTMX + Tailwind)      │
└─────────────────────────────────────────┘
                    ↓ HTTP
┌─────────────────────────────────────────┐
│         Axum Web Server (Rust)          │
├─────────────────────────────────────────┤
│  Handlers → Templates (Maud) → HTML     │
├─────────────────────────────────────────┤
│     SeaORM ↔ PostgreSQL                 │
│     Redis Cache                          │
└─────────────────────────────────────────┘
```

## Key Benefits

### 1. True Full-Stack Rust
- No context switching between languages
- Share types between database, backend, and templates
- Compile-time guarantees for templates

### 2. Simplicity
- No Node.js needed
- No separate build process for frontend
- No API serialization/deserialization overhead
- Single binary to deploy

### 3. Performance
- Server-side rendering (fast first page load)
- HTMX for dynamic updates (minimal JS)
- Maud templates compile to Rust (zero runtime overhead)
- Built-in compression and caching

### 4. Developer Experience
- Templates are just Rust functions
- Full IDE support (autocomplete, refactoring, go-to-definition)
- Compile-time template checking
- Easy debugging (it's all Rust!)

## How It Works

### Maud Templates

Maud lets you write HTML in Rust using the `html!` macro:

```rust
use maud::{html, Markup};

pub fn album_card(title: &str, artist: &str) -> Markup {
    html! {
        div class="album-card" {
            h2 { (title) }
            p { (artist) }
        }
    }
}
```

**Benefits:**
- Compile-time checking (typos become compiler errors)
- Full Rust type system (pass structs, enums, etc.)
- IDE support (autocomplete, refactoring)
- Zero runtime overhead

### HTMX for Interactivity

HTMX adds AJAX capabilities via HTML attributes:

```html
<!-- Load album grid on page load -->
<div hx-get="/albums" hx-trigger="load">
    Loading...
</div>

<!-- Filter with search -->
<input
    type="text"
    hx-get="/albums"
    hx-trigger="keyup changed delay:500ms"
    hx-target="#album-grid"
/>

<!-- Pagination -->
<button hx-get="/albums?page=2" hx-target="#album-grid">
    Next Page
</button>
```

**What HTMX does:**
- Makes AJAX requests based on user interactions
- Swaps returned HTML into the page
- No JavaScript needed for common patterns
- Progressive enhancement (works without JS)

### Example: Album Grid with Filtering

1. **Initial page load** (src/handlers/html.rs):
```rust
pub async fn index() -> Html<String> {
    Html(home_page().into_string())
}
```

2. **Template** (src/templates/pages.rs):
```rust
pub fn home_page() -> Markup {
    base_layout("Library", html! {
        // Filter bar with HTMX attributes
        (filter_bar())

        // Grid loads via HTMX
        div id="album-grid" hx-get="/albums" hx-trigger="load" {
            "Loading..."
        }
    })
}
```

3. **Grid partial handler**:
```rust
pub async fn albums_grid(
    State(state): State<AppState>,
    Query(query): Query<ListAlbumsQuery>,
) -> Result<Html<String>> {
    // Query database
    let albums = fetch_albums(&state.db, &query).await?;

    // Return HTML partial
    Ok(Html(album_grid_partial(albums).into_string()))
}
```

4. **HTMX automatically:**
   - Triggers on page load
   - Makes GET request to `/albums`
   - Swaps returned HTML into `#album-grid`
   - No JavaScript needed!

## File Structure

```
src/
├── templates/
│   ├── layout.rs      # Base HTML layout, nav, footer
│   ├── components.rs  # Reusable UI components
│   └── pages.rs       # Full page templates
├── handlers/
│   ├── html.rs        # HTML-returning handlers
│   ├── albums.rs      # JSON API handlers (still available)
│   └── ...
└── main.rs            # Routes both HTML and JSON
```

## Comparison: MASH vs React SPA

| Aspect | MASH Stack | React SPA |
|--------|-----------|-----------|
| Languages | Rust only | Rust + TypeScript |
| Build process | `cargo build` | `cargo build` + `npm build` |
| Dependencies | Cargo | Cargo + npm |
| Type safety | Compile-time | Runtime (with effort) |
| Bundle size | ~10MB binary | Binary + ~500KB+ JS |
| First paint | Instant (SSR) | Slower (hydration) |
| SEO | Excellent | Requires SSR setup |
| Deployment | Single binary | Binary + static files |
| Interactivity | HTMX (lightweight) | Full React (heavier) |

## When to Use MASH

✅ **Good for:**
- Self-hosted applications
- Admin panels and dashboards
- Content-heavy sites
- Traditional web apps
- When you want full-stack Rust
- Simpler deployment

❌ **Maybe not for:**
- Highly interactive SPAs (games, editors)
- Offline-first applications
- When you need React ecosystem

## Production Considerations

### TailwindCSS

Currently using CDN for development. For production:

1. **Option 1: Standalone CLI** (recommended)
```bash
# Download standalone CLI
curl -sLO https://github.com/tailwindlabs/tailwindcss/releases/latest/download/tailwindcss-linux-x64
chmod +x tailwindcss-linux-x64

# Build CSS
./tailwindcss-linux-x64 -o static/tailwind.css --minify
```

2. **Option 2: Include in template**
```rust
// Embed CSS directly
const TAILWIND_CSS: &str = include_str!("../static/tailwind.css");
```

### HTMX

Currently using CDN. For production, download and serve locally:
```rust
.nest_service("/static", ServeDir::new("static"))
```

## Learning Resources

- **Maud**: https://maud.lambda.xyz/
- **HTMX**: https://htmx.org/docs/
- **Axum**: https://docs.rs/axum/
- **SeaORM**: https://www.sea-ql.org/SeaORM/

## Migration Notes

If you started with the React version:
1. Templates are in `src/templates/` (not `frontend/`)
2. No `npm install` or `npm build` needed
3. Docker image is simpler (no Node.js stage)
4. Single binary deployment
5. API routes still available at `/api/*` for programmatic access
