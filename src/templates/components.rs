use maud::{html, Markup};

use crate::db::enums::OwnershipStatus;

pub struct AlbumCardData {
    pub id: i32,
    pub title: String,
    pub artist_id: i32,
    pub artist_name: String,
    pub cover_art_url: Option<String>,
    pub release_date: Option<String>,
    pub ownership_status: OwnershipStatus,
    pub match_score: Option<i32>,
}

pub fn album_card(album: &AlbumCardData) -> Markup {
    let status_class = match album.ownership_status {
        OwnershipStatus::Owned => "owned",
        OwnershipStatus::NotOwned => "not-owned",
        OwnershipStatus::Downloading => "downloading",
    };

    let cover_url = album
        .cover_art_url
        .as_deref()
        .unwrap_or("https://via.placeholder.com/300x300/1a1a1a/ffffff?text=No+Cover");

    html! {
        div
            class=(format!("album-card {} bg-white rounded-lg shadow-md overflow-hidden cursor-pointer", status_class))
            hx-get={(format!("/albums/{}", album.id))}
            hx-target="#album-detail-modal"
            hx-swap="innerHTML" {

            // Album cover
            div class="relative aspect-square" {
                img
                    src=(cover_url)
                    alt={(format!("{} by {}", album.title, album.artist_name))}
                    class="w-full h-full object-cover"
                    loading="lazy";

                // Status badge
                (status_badge(&album.ownership_status))
            }

            // Album info
            div class="p-4" {
                h3 class="font-semibold text-gray-900 truncate" title=(album.title) {
                    (album.title)
                }
                a
                    href={(format!("/artists/{}", album.artist_id))}
                    class="text-sm text-gray-600 truncate block hover:text-primary hover:underline"
                    title=(album.artist_name)
                    onclick="event.stopPropagation()" {
                    (album.artist_name)
                }

                @if let Some(date) = &album.release_date {
                    p class="text-xs text-gray-500 mt-1" {
                        (date)
                    }
                }

                // Match score indicator
                @if let Some(score) = album.match_score {
                    div class="mt-2" {
                        (match_score_indicator(score))
                    }
                }
            }
        }
    }
}

fn status_badge(status: &OwnershipStatus) -> Markup {
    let (text, color) = match status {
        OwnershipStatus::Owned => ("Owned", "bg-green-500"),
        OwnershipStatus::NotOwned => ("Not Owned", "bg-gray-500"),
        OwnershipStatus::Downloading => ("Downloading", "bg-blue-500"),
    };

    html! {
        div class="absolute top-2 right-2" {
            span class={(format!("px-2 py-1 text-xs font-semibold text-white rounded-full {}", color))} {
                (text)
            }
        }
    }
}

fn match_score_indicator(score: i32) -> Markup {
    let (color, text) = if score >= 90 {
        ("text-green-600", "Excellent match")
    } else if score >= 80 {
        ("text-yellow-600", "Good match")
    } else {
        ("text-red-600", "Poor match")
    };

    html! {
        div class="flex items-center space-x-1" {
            span class={(format!("text-xs {}", color))} {
                "●"
            }
            span class="text-xs text-gray-500" {
                (text) " (" (score) "%)"
            }
        }
    }
}

pub fn filter_bar() -> Markup {
    html! {
        div class="bg-white rounded-lg shadow-sm p-4 mb-6" {
            div class="grid grid-cols-1 md:grid-cols-6 gap-4" {
                // Search
                div {
                    label class="block text-sm font-medium text-gray-700 mb-2" {
                        "Search Albums"
                    }
                    input
                        type="text"
                        name="search"
                        placeholder="Search by title..."
                        class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
                        hx-get="/albums"
                        hx-trigger="keyup changed delay:500ms"
                        hx-target="#album-grid"
                        hx-include="[name='ownership_status'], [name='match_status'], [name='sort_by'], [name='sort_order']";
                }

                // Ownership filter
                div {
                    label class="block text-sm font-medium text-gray-700 mb-2" {
                        "Ownership Status"
                    }
                    select
                        name="ownership_status"
                        class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
                        hx-get="/albums"
                        hx-trigger="change"
                        hx-target="#album-grid"
                        hx-include="[name='search'], [name='match_status'], [name='sort_by'], [name='sort_order']" {
                        option value="" { "All" }
                        option value="owned" { "Owned" }
                        option value="not_owned" { "Not Owned" }
                        option value="downloading" { "Downloading" }
                    }
                }

                // Match status filter
                div {
                    label class="block text-sm font-medium text-gray-700 mb-2" {
                        "Match Status"
                    }
                    select
                        name="match_status"
                        class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
                        hx-get="/albums"
                        hx-trigger="change"
                        hx-target="#album-grid"
                        hx-include="[name='search'], [name='ownership_status'], [name='sort_by'], [name='sort_order']" {
                        option value="" { "All" }
                        option value="matched" { "Matched" }
                        option value="pending" { "Pending" }
                        option value="manual_review" { "Needs Review" }
                        option value="no_match" { "No Match" }
                    }
                }

                // Sort by
                div {
                    label class="block text-sm font-medium text-gray-700 mb-2" {
                        "Sort By"
                    }
                    select
                        name="sort_by"
                        class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
                        hx-get="/albums"
                        hx-trigger="change"
                        hx-target="#album-grid"
                        hx-include="[name='search'], [name='ownership_status'], [name='match_status'], [name='sort_order']" {
                        option value="created_at" { "Date Added" }
                        option value="title" { "Title" }
                        option value="artist" { "Artist" }
                        option value="release_date" { "Release Date" }
                    }
                }

                // Sort order
                div {
                    label class="block text-sm font-medium text-gray-700 mb-2" {
                        "Order"
                    }
                    select
                        name="sort_order"
                        class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
                        hx-get="/albums"
                        hx-trigger="change"
                        hx-target="#album-grid"
                        hx-include="[name='search'], [name='ownership_status'], [name='match_status'], [name='sort_by']" {
                        option value="desc" { "Descending" }
                        option value="asc" { "Ascending" }
                    }
                }

                // Actions
                div class="flex items-end" {
                    button
                        class="w-full bg-primary hover:bg-green-600 text-white font-semibold py-2 px-4 rounded-md transition"
                        hx-post="/api/jobs/spotify-sync"
                        hx-target="#notification-area"
                        hx-swap="innerHTML" {
                        "Sync Spotify"
                    }
                }
            }
        }
    }
}

pub fn pagination(page: u64, total_pages: u64, base_url: &str) -> Markup {
    // Common hx-include for all filter/sort params
    let hx_include = "[name='search'], [name='ownership_status'], [name='match_status'], [name='sort_by'], [name='sort_order']";

    html! {
        div class="flex justify-center items-center space-x-2 mt-8" {
            // Previous button
            @if page > 1 {
                button
                    class="px-4 py-2 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
                    hx-get={(format!("{}?page={}", base_url, page - 1))}
                    hx-target="#album-grid"
                    hx-swap="innerHTML"
                    hx-include=(hx_include) {
                    "Previous"
                }
            } @else {
                button class="px-4 py-2 bg-gray-100 border border-gray-300 rounded-md text-gray-400 cursor-not-allowed" disabled {
                    "Previous"
                }
            }

            // Page numbers
            @for p in page_range(page, total_pages) {
                @if p == page {
                    span class="px-4 py-2 bg-primary text-white rounded-md font-semibold" {
                        (p)
                    }
                } @else {
                    button
                        class="px-4 py-2 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
                        hx-get={(format!("{}?page={}", base_url, p))}
                        hx-target="#album-grid"
                        hx-swap="innerHTML"
                        hx-include=(hx_include) {
                        (p)
                    }
                }
            }

            // Next button
            @if page < total_pages {
                button
                    class="px-4 py-2 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
                    hx-get={(format!("{}?page={}", base_url, page + 1))}
                    hx-target="#album-grid"
                    hx-swap="innerHTML"
                    hx-include=(hx_include) {
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

fn page_range(current: u64, total: u64) -> Vec<u64> {
    let mut pages = Vec::new();
    let range = 2; // Show 2 pages before and after current

    let start = current.saturating_sub(range).max(1);
    let end = (current + range).min(total);

    for p in start..=end {
        pages.push(p);
    }

    pages
}

pub fn loading_spinner() -> Markup {
    html! {
        div class="flex justify-center items-center py-12" {
            div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary" {}
        }
    }
}

pub fn notification(message: &str, notification_type: &str) -> Markup {
    let (bg_color, text_color, icon) = match notification_type {
        "success" => ("bg-green-50", "text-green-800", "✓"),
        "error" => ("bg-red-50", "text-red-800", "✗"),
        "info" => ("bg-blue-50", "text-blue-800", "ℹ"),
        _ => ("bg-gray-50", "text-gray-800", "•"),
    };

    html! {
        div class={(format!("p-4 rounded-md {} {}", bg_color, text_color))} {
            div class="flex items-center" {
                span class="font-bold mr-2" { (icon) }
                span { (message) }
            }
        }
    }
}

// Playlist-related types and components

pub struct PlaylistCardData {
    pub id: i32,
    pub name: String,
    pub owner_name: Option<String>,
    pub track_count: i32,
    pub owned_count: i32,
    pub cover_image_url: Option<String>,
    pub is_enabled: bool,
    pub ownership_percentage: f64,
    pub is_synthetic: bool,
}

pub struct PlaylistTrackData {
    pub position: i32,
    pub track_name: String,
    pub artist_name: String,
    pub album_id: i32,
    pub album_name: String,
    pub duration_ms: Option<i32>,
    pub ownership_status: OwnershipStatus,
}

pub fn playlist_card(playlist: &PlaylistCardData) -> Markup {
    playlist_card_inner(playlist, false)
}

/// Playlist card with optional out-of-band swap attribute for HTMX updates
pub fn playlist_card_oob(playlist: &PlaylistCardData) -> Markup {
    playlist_card_inner(playlist, true)
}

fn playlist_card_inner(playlist: &PlaylistCardData, oob: bool) -> Markup {
    let status_class = if playlist.is_enabled { "enabled" } else { "disabled" };

    // Check if we should show gradient (synthetic playlist without cover)
    let show_gradient = playlist.is_synthetic && playlist.cover_image_url.is_none();

    html! {
        div
            id={(format!("playlist-card-{}", playlist.id))}
            class=(format!("playlist-card {} bg-white rounded-lg shadow-md overflow-hidden cursor-pointer hover:shadow-lg transition-shadow", status_class))
            hx-get={(format!("/playlists/{}", playlist.id))}
            hx-target="#playlist-detail-modal"
            hx-swap="innerHTML"
            hx-swap-oob=[if oob { Some("true") } else { None }] {

            // Playlist cover
            div class="relative aspect-square" {
                @if show_gradient {
                    // Gradient background for Liked Songs
                    div
                        class="w-full h-full flex items-center justify-center"
                        style="background: linear-gradient(135deg, #450af5, #c4efd9);" {
                        span class="text-white text-6xl" { "♥" }
                    }
                } @else {
                    img
                        src=(playlist.cover_image_url.as_deref().unwrap_or("https://via.placeholder.com/300x300/1a1a1a/ffffff?text=Playlist"))
                        alt={(format!("{} playlist", playlist.name))}
                        class="w-full h-full object-cover"
                        loading="lazy";
                }

                // Enabled/disabled badge
                @if !playlist.is_enabled {
                    div class="absolute inset-0 bg-black bg-opacity-50 flex items-center justify-center" {
                        span class="text-white text-sm font-semibold" { "Disabled" }
                    }
                }

                // Ownership percentage badge
                div class="absolute top-2 right-2" {
                    span class=(format!("px-2 py-1 text-xs font-semibold text-white rounded-full {}",
                        if playlist.ownership_percentage >= 80.0 { "bg-green-500" }
                        else if playlist.ownership_percentage >= 50.0 { "bg-yellow-500" }
                        else { "bg-gray-500" }
                    )) {
                        (format!("{:.0}%", playlist.ownership_percentage))
                    }
                }
            }

            // Playlist info
            div class="p-4" {
                h3 class="font-semibold text-gray-900 truncate" title=(playlist.name) {
                    (playlist.name)
                }

                @if let Some(owner) = &playlist.owner_name {
                    p class="text-sm text-gray-600 truncate" {
                        "by " (owner)
                    }
                }

                div class="mt-2 flex justify-between items-center" {
                    p class="text-xs text-gray-500" {
                        (playlist.track_count) " tracks"
                    }
                    p class="text-xs text-green-600" {
                        (playlist.owned_count) " owned"
                    }
                }
            }
        }
    }
}

pub fn playlist_track_row(track: &PlaylistTrackData) -> Markup {
    let status_color = match track.ownership_status {
        OwnershipStatus::Owned => "text-green-600",
        OwnershipStatus::NotOwned => "text-gray-400",
        OwnershipStatus::Downloading => "text-blue-600",
    };

    let duration_str = track.duration_ms.map(format_duration).unwrap_or_default();

    html! {
        tr class="hover:bg-gray-50" {
            // Position
            td class="px-4 py-3 text-sm text-gray-500 text-right w-12" {
                (track.position + 1)
            }

            // Track name
            td class="px-4 py-3" {
                div class="text-sm font-medium text-gray-900" { (track.track_name) }
                div class="text-sm text-gray-500" { (track.artist_name) }
            }

            // Album (clickable)
            td class="px-4 py-3 text-sm text-gray-600" {
                span
                    class="cursor-pointer hover:text-primary hover:underline"
                    hx-get={(format!("/albums/{}", track.album_id))}
                    hx-target="#album-detail-modal"
                    hx-swap="innerHTML" {
                    (track.album_name)
                }
            }

            // Duration
            td class="px-4 py-3 text-sm text-gray-500 text-right" {
                (duration_str)
            }

            // Ownership status
            td class="px-4 py-3 text-right" {
                span class=(format!("text-lg {}", status_color)) {
                    @match track.ownership_status {
                        OwnershipStatus::Owned => "●",
                        OwnershipStatus::NotOwned => "○",
                        OwnershipStatus::Downloading => "◐",
                    }
                }
            }
        }
    }
}

fn format_duration(ms: i32) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{}:{:02}", minutes, seconds)
}

/// Render playlist track rows for infinite scroll
pub fn playlist_tracks_rows(
    tracks: Vec<PlaylistTrackData>,
    has_more: bool,
    playlist_id: i32,
    next_offset: u64,
) -> Markup {
    html! {
        @for track in &tracks {
            (playlist_track_row(track))
        }

        @if has_more {
            // Sentinel element that triggers loading more when scrolled into view
            tr
                id="load-more-trigger"
                hx-get={(format!("/playlists/{}/tracks?offset={}&limit=50", playlist_id, next_offset))}
                hx-trigger="revealed"
                hx-swap="outerHTML" {
                td colspan="5" class="px-4 py-3 text-center text-gray-500" {
                    div class="flex justify-center items-center" {
                        div class="animate-spin rounded-full h-5 w-5 border-b-2 border-primary mr-2" {}
                        "Loading more tracks..."
                    }
                }
            }
        }
    }
}

// Artist-related types and components

pub struct ArtistCardData {
    pub id: i32,
    pub name: String,
    pub album_count: i64,
    pub owned_count: i64,
    pub ownership_percentage: f64,
}

pub fn artist_card(artist: &ArtistCardData) -> Markup {
    let progress_width = artist.ownership_percentage.min(100.0).max(0.0);

    let progress_color = if artist.ownership_percentage >= 80.0 {
        "bg-green-500"
    } else if artist.ownership_percentage >= 50.0 {
        "bg-yellow-500"
    } else {
        "bg-gray-400"
    };

    html! {
        a
            href={(format!("/artists/{}", artist.id))}
            class="artist-card block bg-white rounded-lg shadow-md overflow-hidden cursor-pointer hover:shadow-lg transition-shadow p-4" {

            // Artist name
            h3 class="font-semibold text-gray-900 text-lg truncate mb-2" title=(artist.name) {
                (artist.name)
            }

            // Album count badge
            div class="flex items-center justify-between mb-3" {
                span class="text-sm text-gray-600" {
                    (artist.album_count) " album" @if artist.album_count != 1 { "s" }
                }
                span class="text-sm font-medium text-green-600" {
                    (artist.owned_count) " owned"
                }
            }

            // Ownership progress bar
            div class="w-full bg-gray-200 rounded-full h-2 mb-2" {
                div
                    class={(format!("h-2 rounded-full {}", progress_color))}
                    style={(format!("width: {}%", progress_width))} {}
            }

            // Percentage
            div class="text-right" {
                span class=(format!("text-sm font-medium {}",
                    if artist.ownership_percentage >= 80.0 { "text-green-600" }
                    else if artist.ownership_percentage >= 50.0 { "text-yellow-600" }
                    else { "text-gray-500" }
                )) {
                    (format!("{:.0}%", artist.ownership_percentage)) " complete"
                }
            }
        }
    }
}

pub fn artist_filter_bar() -> Markup {
    html! {
        div class="bg-white rounded-lg shadow-sm p-4 mb-6" {
            div class="grid grid-cols-1 md:grid-cols-3 gap-4" {
                // Search
                div {
                    label class="block text-sm font-medium text-gray-700 mb-2" {
                        "Search Artists"
                    }
                    input
                        type="text"
                        name="search"
                        placeholder="Search by artist name..."
                        class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
                        hx-get="/artists-grid"
                        hx-trigger="keyup changed delay:500ms"
                        hx-target="#artist-grid"
                        hx-include="[name='sort_by'], [name='sort_order']";
                }

                // Sort by
                div {
                    label class="block text-sm font-medium text-gray-700 mb-2" {
                        "Sort By"
                    }
                    select
                        name="sort_by"
                        class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
                        hx-get="/artists-grid"
                        hx-trigger="change"
                        hx-target="#artist-grid"
                        hx-include="[name='search'], [name='sort_order']" {
                        option value="name" { "Name" }
                        option value="album_count" { "Album Count" }
                        option value="ownership" { "Ownership %" }
                    }
                }

                // Sort order
                div {
                    label class="block text-sm font-medium text-gray-700 mb-2" {
                        "Order"
                    }
                    select
                        name="sort_order"
                        class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
                        hx-get="/artists-grid"
                        hx-trigger="change"
                        hx-target="#artist-grid"
                        hx-include="[name='search'], [name='sort_by']" {
                        option value="asc" { "Ascending" }
                        option value="desc" { "Descending" }
                    }
                }
            }
        }
    }
}

pub fn artist_pagination(page: u64, total_pages: u64, base_url: &str) -> Markup {
    html! {
        div class="flex justify-center items-center space-x-2 mt-8" {
            // Previous button
            @if page > 1 {
                button
                    class="px-4 py-2 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
                    hx-get={(format!("{}?page={}", base_url, page - 1))}
                    hx-target="#artist-grid"
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
                    hx-target="#artist-grid"
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
