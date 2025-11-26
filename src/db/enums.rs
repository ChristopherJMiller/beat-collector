use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OwnershipStatus {
    NotOwned,
    Owned,
    Downloading,
}

impl OwnershipStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::NotOwned => "not_owned",
            Self::Owned => "owned",
            Self::Downloading => "downloading",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "not_owned" => Some(Self::NotOwned),
            "owned" => Some(Self::Owned),
            "downloading" => Some(Self::Downloading),
            _ => None,
        }
    }
}

impl From<OwnershipStatus> for String {
    fn from(status: OwnershipStatus) -> String {
        status.as_str().to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchStatus {
    Pending,
    Matched,
    ManualReview,
    NoMatch,
}

impl MatchStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::Matched => "matched",
            Self::ManualReview => "manual_review",
            Self::NoMatch => "no_match",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "matched" => Some(Self::Matched),
            "manual_review" => Some(Self::ManualReview),
            "no_match" => Some(Self::NoMatch),
            _ => None,
        }
    }
}

impl From<MatchStatus> for String {
    fn from(status: MatchStatus) -> String {
        status.as_str().to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobType {
    SpotifySync,
    MusicbrainzMatch,
    LidarrSearch,
    CoverArtFetch,
    FilesystemScan,
}

impl JobType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::SpotifySync => "spotify_sync",
            Self::MusicbrainzMatch => "musicbrainz_match",
            Self::LidarrSearch => "lidarr_search",
            Self::CoverArtFetch => "cover_art_fetch",
            Self::FilesystemScan => "filesystem_scan",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "spotify_sync" => Some(Self::SpotifySync),
            "musicbrainz_match" => Some(Self::MusicbrainzMatch),
            "lidarr_search" => Some(Self::LidarrSearch),
            "cover_art_fetch" => Some(Self::CoverArtFetch),
            "filesystem_scan" => Some(Self::FilesystemScan),
            _ => None,
        }
    }
}

impl From<JobType> for String {
    fn from(job_type: JobType) -> String {
        job_type.as_str().to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl JobStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

impl From<JobStatus> for String {
    fn from(status: JobStatus) -> String {
        status.as_str().to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadStatus {
    Pending,
    Searching,
    Downloading,
    Completed,
    Failed,
}

impl DownloadStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::Searching => "searching",
            Self::Downloading => "downloading",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "searching" => Some(Self::Searching),
            "downloading" => Some(Self::Downloading),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

impl From<DownloadStatus> for String {
    fn from(status: DownloadStatus) -> String {
        status.as_str().to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcquisitionSource {
    Bandcamp,
    Physical,
    Lidarr,
    Unknown,
}

impl AcquisitionSource {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Bandcamp => "bandcamp",
            Self::Physical => "physical",
            Self::Lidarr => "lidarr",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "bandcamp" => Some(Self::Bandcamp),
            "physical" => Some(Self::Physical),
            "lidarr" => Some(Self::Lidarr),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

impl From<AcquisitionSource> for String {
    fn from(source: AcquisitionSource) -> String {
        source.as_str().to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AlbumSource {
    #[default]
    SavedAlbum,
    PlaylistImport,
}

impl AlbumSource {
    pub fn as_str(&self) -> &str {
        match self {
            Self::SavedAlbum => "saved_album",
            Self::PlaylistImport => "playlist_import",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "saved_album" => Some(Self::SavedAlbum),
            "playlist_import" => Some(Self::PlaylistImport),
            _ => None,
        }
    }
}

impl From<AlbumSource> for String {
    fn from(source: AlbumSource) -> String {
        source.as_str().to_string()
    }
}
