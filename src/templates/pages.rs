use maud::{html, Markup};

use super::components::{
    album_card, filter_bar, pagination, playlist_card, playlist_track_row,
    AlbumCardData, PlaylistCardData, PlaylistTrackData,
};
use super::layout::base_layout;

pub fn home_page() -> Markup {
    base_layout(
        "Library",
        html! {
            // Notification area for HTMX responses
            div id="notification-area" class="mb-4" {}

            // Filter bar
            (filter_bar())

            // Album grid
            div id="album-grid" hx-get="/albums" hx-trigger="load" {
                div class="flex justify-center items-center py-12" {
                    div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary" {}
                    span class="ml-3 text-gray-600" { "Loading your library..." }
                }
            }

            // Album detail modal (populated by HTMX)
            div id="album-detail-modal" {}
        },
    )
}

pub fn album_grid_partial(
    albums: Vec<AlbumCardData>,
    page: u64,
    total_pages: u64,
) -> Markup {
    html! {
        @if albums.is_empty() {
            div class="text-center py-12" {
                p class="text-gray-600 text-lg" { "No albums found." }
                p class="text-gray-500 mt-2" {
                    "Try connecting your Spotify account or adjusting your filters."
                }
            }
        } @else {
            div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-6" {
                @for album in albums {
                    (album_card(&album))
                }
            }

            // Pagination
            (pagination(page, total_pages, "/albums"))
        }
    }
}

pub fn album_detail_modal(
    album: &AlbumCardData,
    artist_name: &str,
    genres: &Option<Vec<String>>,
    total_tracks: Option<i32>,
) -> Markup {
    html! {
        // Modal backdrop
        div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4"
             onclick="this.remove()" {

            // Modal content
            div class="bg-white rounded-lg shadow-xl max-w-2xl w-full max-h-screen overflow-y-auto"
                 onclick="event.stopPropagation()" {

                // Header
                div class="flex justify-between items-center p-6 border-b" {
                    h2 class="text-2xl font-bold text-gray-900" { (album.title) }
                    button
                        class="text-gray-400 hover:text-gray-600 text-2xl"
                        onclick="document.getElementById('album-detail-modal').innerHTML = ''" {
                        "×"
                    }
                }

                // Content
                div class="p-6" {
                    div class="flex flex-col md:flex-row gap-6" {
                        // Album cover
                        div class="flex-shrink-0" {
                            img
                                src={(album.cover_art_url.as_deref().unwrap_or("https://via.placeholder.com/300"))}
                                alt={(format!("{} cover", album.title))}
                                class="w-full md:w-64 rounded-lg shadow-md";
                        }

                        // Details
                        div class="flex-grow" {
                            dl class="space-y-4" {
                                div {
                                    dt class="text-sm font-medium text-gray-500" { "Artist" }
                                    dd class="mt-1 text-lg text-gray-900" { (artist_name) }
                                }

                                @if let Some(date) = &album.release_date {
                                    div {
                                        dt class="text-sm font-medium text-gray-500" { "Release Date" }
                                        dd class="mt-1 text-gray-900" { (date) }
                                    }
                                }

                                @if let Some(tracks) = total_tracks {
                                    div {
                                        dt class="text-sm font-medium text-gray-500" { "Tracks" }
                                        dd class="mt-1 text-gray-900" { (tracks) }
                                    }
                                }

                                div {
                                    dt class="text-sm font-medium text-gray-500" { "Status" }
                                    dd class="mt-1" {
                                        (status_badge_large(&album.ownership_status))
                                    }
                                }

                                @if let Some(score) = album.match_score {
                                    div {
                                        dt class="text-sm font-medium text-gray-500" { "MusicBrainz Match" }
                                        dd class="mt-1 text-gray-900" { (score) "% confidence" }
                                    }
                                }

                                @if let Some(genre_list) = genres {
                                    @if !genre_list.is_empty() {
                                        div {
                                            dt class="text-sm font-medium text-gray-500" { "Genres" }
                                            dd class="mt-1 flex flex-wrap gap-2" {
                                                @for genre in genre_list {
                                                    span class="px-2 py-1 bg-gray-100 text-gray-700 text-sm rounded" {
                                                        (genre)
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Actions
                    div class="mt-6 pt-6 border-t flex flex-wrap gap-3" {
                        button
                            class="px-4 py-2 bg-primary hover:bg-green-600 text-white font-semibold rounded-md"
                            hx-post={(format!("/api/albums/{}/search-lidarr", album.id))}
                            hx-target="#notification-area"
                            hx-swap="innerHTML" {
                            "Search in Lidarr"
                        }

                        button
                            class="px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white font-semibold rounded-md"
                            hx-post={(format!("/api/albums/{}/match", album.id))}
                            hx-target="#notification-area"
                            hx-swap="innerHTML" {
                            "Re-match MusicBrainz"
                        }

                        @if let Some(source_artist) = artist_name.split(" feat.").next() {
                            a
                                href={(format!("https://bandcamp.com/search?q={}+{}&item_type=a",
                                    urlencoding::encode(source_artist),
                                    urlencoding::encode(&album.title)))}
                                target="_blank"
                                class="px-4 py-2 bg-gray-700 hover:bg-gray-800 text-white font-semibold rounded-md" {
                                "Search on Bandcamp"
                            }
                        }

                        button
                            class="px-4 py-2 bg-green-500 hover:bg-green-600 text-white font-semibold rounded-md"
                            hx-patch={(format!("/api/albums/{}", album.id))}
                            hx-vals=r#"{"ownership_status": "owned", "acquisition_source": "manual"}"#
                            hx-target="#notification-area"
                            hx-swap="innerHTML" {
                            "Mark as Owned"
                        }
                    }
                }
            }
        }
    }
}

fn status_badge_large(status: &crate::db::OwnershipStatus) -> Markup {
    use crate::db::OwnershipStatus;

    let (text, color) = match status {
        OwnershipStatus::Owned => ("Owned", "bg-green-100 text-green-800"),
        OwnershipStatus::NotOwned => ("Not Owned", "bg-gray-100 text-gray-800"),
        OwnershipStatus::Downloading => ("Downloading", "bg-blue-100 text-blue-800"),
    };

    html! {
        span class={(format!("px-3 py-1 text-sm font-semibold rounded-full {}", color))} {
            (text)
        }
    }
}

pub fn settings_page(
    lidarr_url: Option<String>,
    music_folder: Option<String>,
    spotify_connected: bool,
) -> Markup {
    base_layout(
        "Settings",
        html! {
            div id="notification-area" class="mb-4" {}

            div class="max-w-3xl mx-auto" {
                h1 class="text-3xl font-bold text-gray-900 mb-8" { "Settings" }

                // Spotify connection
                div class="bg-white rounded-lg shadow-sm p-6 mb-6" {
                    h2 class="text-xl font-semibold mb-4" { "Spotify Connection" }

                    @if spotify_connected {
                        div class="flex items-center space-x-2 text-green-600 mb-4" {
                            span { "✓" }
                            span { "Connected to Spotify" }
                        }
                        button
                            class="px-4 py-2 bg-primary hover:bg-green-600 text-white font-semibold rounded-md"
                            hx-post="/api/jobs/spotify-sync"
                            hx-target="#notification-area" {
                            "Sync Library Now"
                        }
                    } @else {
                        p class="text-gray-600 mb-4" {
                            "Connect your Spotify account to import your music library."
                        }
                        button
                            class="px-4 py-2 bg-primary hover:bg-green-600 text-white font-semibold rounded-md"
                            hx-get="/api/auth/spotify/authorize"
                            hx-target="this"
                            hx-swap="outerHTML" {
                            "Connect Spotify"
                        }
                    }
                }

                // Lidarr settings
                div class="bg-white rounded-lg shadow-sm p-6 mb-6" {
                    h2 class="text-xl font-semibold mb-4" { "Lidarr Integration" }

                    form hx-put="/api/settings" hx-target="#notification-area" {
                        div class="space-y-4" {
                            div {
                                label class="block text-sm font-medium text-gray-700 mb-2" {
                                    "Lidarr URL"
                                }
                                input
                                    type="url"
                                    name="lidarr_url"
                                    value=[lidarr_url]
                                    placeholder="http://localhost:8686"
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary";
                            }

                            div {
                                label class="block text-sm font-medium text-gray-700 mb-2" {
                                    "Lidarr API Key"
                                }
                                input
                                    type="password"
                                    name="lidarr_api_key"
                                    placeholder="Your API key from Lidarr settings"
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary";
                            }

                            div class="flex space-x-3" {
                                button
                                    type="submit"
                                    class="px-4 py-2 bg-primary hover:bg-green-600 text-white font-semibold rounded-md" {
                                    "Save Settings"
                                }

                                button
                                    type="button"
                                    class="px-4 py-2 bg-gray-200 hover:bg-gray-300 text-gray-700 font-semibold rounded-md"
                                    hx-post="/api/settings/test-lidarr"
                                    hx-target="#notification-area" {
                                    "Test Connection"
                                }
                            }
                        }
                    }
                }

                // Music folder settings
                div class="bg-white rounded-lg shadow-sm p-6" {
                    h2 class="text-xl font-semibold mb-4" { "Music Folder" }

                    form hx-put="/api/settings" hx-target="#notification-area" {
                        div class="space-y-4" {
                            div {
                                label class="block text-sm font-medium text-gray-700 mb-2" {
                                    "Local Music Directory"
                                }
                                input
                                    type="text"
                                    name="music_folder_path"
                                    value=[music_folder]
                                    placeholder="/path/to/your/music"
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary";
                                p class="mt-2 text-sm text-gray-500" {
                                    "Path to your local music folder (e.g., /music or /home/user/Music)"
                                }
                            }

                            button
                                type="submit"
                                class="px-4 py-2 bg-primary hover:bg-green-600 text-white font-semibold rounded-md" {
                                "Save Path"
                            }
                        }
                    }
                }
            }
        },
    )
}

pub fn jobs_page() -> Markup {
    base_layout(
        "Jobs",
        html! {
            div class="max-w-5xl mx-auto" {
                h1 class="text-3xl font-bold text-gray-900 mb-8" { "Background Jobs" }

                div id="jobs-list" hx-get="/api/jobs" hx-trigger="load, every 5s" {
                    div class="flex justify-center py-12" {
                        div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary" {}
                    }
                }
            }
        },
    )
}

pub fn stats_page() -> Markup {
    base_layout(
        "Statistics",
        html! {
            div class="max-w-5xl mx-auto" {
                h1 class="text-3xl font-bold text-gray-900 mb-8" { "Library Statistics" }

                div id="stats-content" hx-get="/api/stats" hx-trigger="load" {
                    div class="flex justify-center py-12" {
                        div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary" {}
                    }
                }
            }
        },
    )
}

pub fn playlists_page() -> Markup {
    base_layout(
        "Playlists",
        html! {
            // Notification area for HTMX responses
            div id="notification-area" class="mb-4" {}

            // Header with actions
            div class="flex justify-between items-center mb-8" {
                h1 class="text-3xl font-bold text-gray-900" { "Your Playlists" }

                button
                    class="px-4 py-2 bg-primary hover:bg-green-600 text-white font-semibold rounded-md"
                    hx-post="/api/jobs/spotify-sync"
                    hx-target="#notification-area"
                    hx-swap="innerHTML" {
                    "Sync from Spotify"
                }
            }

            // Playlist grid
            div id="playlist-grid" hx-get="/playlists-grid" hx-trigger="load" {
                div class="flex justify-center items-center py-12" {
                    div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary" {}
                    span class="ml-3 text-gray-600" { "Loading your playlists..." }
                }
            }

            // Playlist detail modal (populated by HTMX)
            div id="playlist-detail-modal" {}

            // Album detail modal (for clicking on albums within playlist tracks)
            div id="album-detail-modal" {}
        },
    )
}

pub fn playlist_grid_partial(
    playlists: Vec<PlaylistCardData>,
    page: u64,
    total_pages: u64,
) -> Markup {
    html! {
        @if playlists.is_empty() {
            div class="text-center py-12" {
                p class="text-gray-600 text-lg" { "No playlists found." }
                p class="text-gray-500 mt-2" {
                    "Sync your Spotify account to import your playlists."
                }
            }
        } @else {
            div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-6" {
                @for playlist in playlists {
                    (playlist_card(&playlist))
                }
            }

            // Pagination
            (playlist_pagination(page, total_pages, "/playlists-grid"))
        }
    }
}

pub fn playlist_detail_partial(
    playlist: &PlaylistCardData,
    tracks: Vec<PlaylistTrackData>,
) -> Markup {
    html! {
        // Modal backdrop
        div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4"
             onclick="this.remove()" {

            // Modal content
            div class="bg-white rounded-lg shadow-xl max-w-4xl w-full max-h-[90vh] overflow-hidden flex flex-col"
                 onclick="event.stopPropagation()" {

                // Header
                div class="flex justify-between items-center p-6 border-b flex-shrink-0" {
                    div class="flex items-center space-x-4" {
                        @if let Some(cover_url) = &playlist.cover_image_url {
                            img
                                src=(cover_url)
                                alt="Playlist cover"
                                class="w-16 h-16 rounded-md object-cover";
                        }
                        div {
                            h2 class="text-2xl font-bold text-gray-900" { (playlist.name) }
                            @if let Some(owner) = &playlist.owner_name {
                                p class="text-sm text-gray-600" { "by " (owner) }
                            }
                        }
                    }

                    div class="flex items-center space-x-4" {
                        // Enable/Disable toggle
                        button
                            class=(format!("px-3 py-1 rounded-full text-sm font-semibold {}",
                                if playlist.is_enabled { "bg-green-100 text-green-800" }
                                else { "bg-gray-100 text-gray-600" }
                            ))
                            hx-post={(format!("/api/playlists/{}/toggle", playlist.id))}
                            hx-target="#playlist-detail-modal"
                            hx-swap="innerHTML" {
                            @if playlist.is_enabled { "Enabled" } @else { "Disabled" }
                        }

                        button
                            class="text-gray-400 hover:text-gray-600 text-2xl"
                            onclick="document.getElementById('playlist-detail-modal').innerHTML = ''" {
                            "×"
                        }
                    }
                }

                // Stats bar
                div class="px-6 py-3 bg-gray-50 border-b flex items-center space-x-6 flex-shrink-0" {
                    div class="text-sm" {
                        span class="text-gray-500" { "Tracks: " }
                        span class="font-semibold" { (playlist.track_count) }
                    }
                    div class="text-sm" {
                        span class="text-gray-500" { "Owned: " }
                        span class="font-semibold text-green-600" { (playlist.owned_count) }
                    }
                    div class="text-sm" {
                        span class="text-gray-500" { "Ownership: " }
                        span class=(format!("font-semibold {}",
                            if playlist.ownership_percentage >= 80.0 { "text-green-600" }
                            else if playlist.ownership_percentage >= 50.0 { "text-yellow-600" }
                            else { "text-gray-600" }
                        )) {
                            (format!("{:.1}%", playlist.ownership_percentage))
                        }
                    }
                }

                // Track list
                div class="overflow-y-auto flex-grow" {
                    @if tracks.is_empty() {
                        div class="p-8 text-center text-gray-500" {
                            "No tracks synced yet. Enable the playlist and run a Spotify sync."
                        }
                    } @else {
                        table class="w-full" {
                            thead class="sticky top-0 bg-white border-b" {
                                tr {
                                    th class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase w-12" { "#" }
                                    th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase" { "Track" }
                                    th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase" { "Album" }
                                    th class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase" { "Duration" }
                                    th class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase w-16" { "Owned" }
                                }
                            }
                            tbody class="divide-y divide-gray-200" {
                                @for track in tracks {
                                    (playlist_track_row(&track))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn playlist_pagination(page: u64, total_pages: u64, base_url: &str) -> Markup {
    html! {
        div class="flex justify-center items-center space-x-2 mt-8" {
            // Previous button
            @if page > 1 {
                button
                    class="px-4 py-2 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
                    hx-get={(format!("{}?page={}", base_url, page - 1))}
                    hx-target="#playlist-grid"
                    hx-swap="innerHTML" {
                    "Previous"
                }
            } @else {
                button class="px-4 py-2 bg-gray-100 border border-gray-300 rounded-md text-gray-400 cursor-not-allowed" disabled {
                    "Previous"
                }
            }

            // Page indicator
            span class="px-4 py-2 text-gray-600" {
                "Page " (page) " of " (total_pages)
            }

            // Next button
            @if page < total_pages {
                button
                    class="px-4 py-2 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
                    hx-get={(format!("{}?page={}", base_url, page + 1))}
                    hx-target="#playlist-grid"
                    hx-swap="innerHTML" {
                    "Next"
                }
            } @else {
                button class="px-4 py-2 bg-gray-100 border border-gray-300 rounded-md text-gray-400 cursor-not-allowed" disabled {
                    "Next"
                }
            }
        }
    }
}
