use maud::{html, Markup};

use super::components::{album_card, filter_bar, pagination, AlbumCardData};
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
