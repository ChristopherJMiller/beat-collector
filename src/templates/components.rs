use maud::{html, Markup};
use uuid::Uuid;

use crate::db::entities::album::{OwnershipStatus, MatchStatus};

pub struct AlbumCardData {
    pub id: Uuid,
    pub title: String,
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
                p class="text-sm text-gray-600 truncate" title=(album.artist_name) {
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
            div class="grid grid-cols-1 md:grid-cols-4 gap-4" {
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
                        hx-include="[name='ownership_status'], [name='match_status']";
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
                        hx-include="[name='search'], [name='match_status']" {
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
                        hx-include="[name='search'], [name='ownership_status']" {
                        option value="" { "All" }
                        option value="matched" { "Matched" }
                        option value="pending" { "Pending" }
                        option value="manual_review" { "Needs Review" }
                        option value="no_match" { "No Match" }
                    }
                }

                // Actions
                div class="flex items-end" {
                    button
                        class="w-full bg-primary hover:bg-green-600 text-white font-semibold py-2 px-4 rounded-md transition"
                        hx-post="/jobs/spotify-sync"
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
    html! {
        div class="flex justify-center items-center space-x-2 mt-8" {
            // Previous button
            @if page > 1 {
                button
                    class="px-4 py-2 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
                    hx-get={(format!("{}?page={}", base_url, page - 1))}
                    hx-target="#album-grid"
                    hx-swap="innerHTML" {
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
                        hx-swap="innerHTML" {
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
