pub use sea_orm_migration::prelude::*;

mod m20240101_000001_create_artists_table;
mod m20240101_000002_create_albums_table;
mod m20240101_000003_create_tracks_table;
mod m20240101_000004_create_user_settings_table;
mod m20240101_000005_create_jobs_table;
mod m20240101_000006_create_lidarr_downloads_table;
mod m20240101_000007_create_playlists_table;
mod m20240101_000008_create_playlist_tracks_table;
mod m20240101_000009_add_album_source_column;
mod m20240101_000010_add_playlist_is_synthetic;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_artists_table::Migration),
            Box::new(m20240101_000002_create_albums_table::Migration),
            Box::new(m20240101_000003_create_tracks_table::Migration),
            Box::new(m20240101_000004_create_user_settings_table::Migration),
            Box::new(m20240101_000005_create_jobs_table::Migration),
            Box::new(m20240101_000006_create_lidarr_downloads_table::Migration),
            Box::new(m20240101_000007_create_playlists_table::Migration),
            Box::new(m20240101_000008_create_playlist_tracks_table::Migration),
            Box::new(m20240101_000009_add_album_source_column::Migration),
            Box::new(m20240101_000010_add_playlist_is_synthetic::Migration),
        ]
    }
}
