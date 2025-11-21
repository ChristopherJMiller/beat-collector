use maud::{html, Markup, DOCTYPE};

pub fn base_layout(title: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" class="h-full" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " - Beat Collector" }

                // TailwindCSS via CDN (for development; use standalone CLI for production)
                script src="https://cdn.tailwindcss.com" {}

                // HTMX
                script src="https://unpkg.com/htmx.org@1.9.10" {}

                // Custom TailwindCSS config
                script {
                    r#"
                    tailwind.config = {
                        theme: {
                            extend: {
                                colors: {
                                    primary: '#1db954',
                                    owned: '#22c55e',
                                    notOwned: '#6b7280',
                                    downloading: '#3b82f6',
                                    warning: '#eab308'
                                }
                            }
                        }
                    }
                    "#
                }

                // Custom styles
                style {
                    r#"
                    .album-card {
                        transition: all 0.2s ease-in-out;
                    }
                    .album-card:hover {
                        transform: translateY(-4px);
                        box-shadow: 0 10px 20px rgba(0,0,0,0.1);
                    }
                    .album-card.not-owned {
                        opacity: 0.5;
                        filter: grayscale(100%);
                    }
                    .album-card.owned {
                        opacity: 1;
                        filter: grayscale(0%);
                    }
                    .album-card.downloading {
                        border: 2px solid #3b82f6;
                        animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
                    }
                    @keyframes pulse {
                        0%, 100% { opacity: 1; }
                        50% { opacity: .7; }
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
