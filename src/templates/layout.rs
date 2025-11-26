use maud::{html, Markup, DOCTYPE};

pub fn base_layout(title: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" class="h-full" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " - Beat Collector" }

                // Compiled TailwindCSS
                link rel="stylesheet" href="/static/css/output.css";

                // HTMX for interactivity
                script src="https://unpkg.com/htmx.org@1.9.10" {}

                // Additional custom styles
                style {
                    r#"
                    .album-card:hover {
                        transform: translateY(-4px);
                        box-shadow: 0 10px 20px rgba(0,0,0,0.1);
                    }
                    "#
                }
            }
            body class="h-full bg-gray-50" {
                div class="min-h-full" {
                    // Navigation
                    (nav_bar())

                    // Main content
                    main class="container mx-auto px-4 py-8" {
                        (content)
                    }

                    // Footer
                    (footer())
                }
            }
        }
    }
}

fn nav_bar() -> Markup {
    html! {
        nav class="bg-white shadow-sm" {
            div class="container mx-auto px-4" {
                div class="flex justify-between items-center h-16" {
                    // Logo/Brand
                    a href="/" class="flex items-center space-x-3" {
                        span class="text-2xl" { "🎵" }
                        span class="text-xl font-bold text-gray-900" { "Beat Collector" }
                    }

                    // Navigation links
                    div class="flex space-x-4" {
                        a href="/" class="text-gray-700 hover:text-primary px-3 py-2 rounded-md text-sm font-medium" {
                            "Library"
                        }
                        a href="/artists" class="text-gray-700 hover:text-primary px-3 py-2 rounded-md text-sm font-medium" {
                            "Artists"
                        }
                        a href="/playlists" class="text-gray-700 hover:text-primary px-3 py-2 rounded-md text-sm font-medium" {
                            "Playlists"
                        }
                        a href="/settings" class="text-gray-700 hover:text-primary px-3 py-2 rounded-md text-sm font-medium" {
                            "Settings"
                        }
                        a href="/jobs" class="text-gray-700 hover:text-primary px-3 py-2 rounded-md text-sm font-medium" {
                            "Jobs"
                        }
                        a href="/stats" class="text-gray-700 hover:text-primary px-3 py-2 rounded-md text-sm font-medium" {
                            "Stats"
                        }
                    }
                }
            }
        }
    }
}

fn footer() -> Markup {
    html! {
        footer class="bg-white border-t border-gray-200 mt-12" {
            div class="container mx-auto px-4 py-6" {
                div class="text-center text-gray-600 text-sm" {
                    "Beat Collector - Self-hosted music library management"
                    " · "
                    a href="https://github.com/yourusername/beat-collector"
                      class="text-primary hover:underline"
                      target="_blank" {
                        "GitHub"
                    }
                }
            }
        }
    }
}
