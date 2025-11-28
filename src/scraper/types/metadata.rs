use super::MediaType;
use serde::{Deserialize, Serialize};

/// Complete metadata for a media item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMetadata {
    /// Provider-specific ID
    pub id: String,
    /// Primary title
    pub title: String,
    /// Original/native title
    pub original_title: Option<String>,
    /// Sort title
    pub sort_title: Option<String>,
    /// Media type
    pub media_type: MediaType,
    /// Tagline
    pub tagline: Option<String>,
    /// Full description/plot
    pub overview: Option<String>,
    /// Release/air date (YYYY-MM-DD)
    pub release_date: Option<String>,
    /// End date for series (YYYY-MM-DD)
    pub end_date: Option<String>,
    /// Runtime in minutes
    pub runtime: Option<i32>,
    /// Rating (0-10 scale)
    pub rating: Option<f64>,
    /// Vote count
    pub vote_count: Option<i32>,
    /// Genres
    pub genres: Vec<String>,
    /// Tags/keywords
    pub tags: Vec<String>,
    /// Studios/production companies
    pub studios: Vec<String>,
    /// Original language code (e.g., "en", "ja")
    pub language: Option<String>,
    /// Content rating (e.g., "PG-13", "TV-MA")
    pub content_rating: Option<String>,
    /// Status (e.g., "Released", "Ended", "Continuing")
    pub status: Option<String>,
    /// Images
    pub images: ImageSet,
    /// External IDs
    pub external_ids: ExternalIds,
    /// Provider name
    pub provider: String,

    // TV/Anime specific
    /// Number of seasons
    pub season_count: Option<i32>,
    /// Number of episodes
    pub episode_count: Option<i32>,
    /// Season information
    pub seasons: Vec<SeasonInfo>,

    // People
    /// Cast members
    pub cast: Vec<PersonInfo>,
    /// Crew members
    pub crew: Vec<PersonInfo>,
}

impl Default for MediaMetadata {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            original_title: None,
            sort_title: None,
            media_type: MediaType::Unknown,
            tagline: None,
            overview: None,
            release_date: None,
            end_date: None,
            runtime: None,
            rating: None,
            vote_count: None,
            genres: Vec::new(),
            tags: Vec::new(),
            studios: Vec::new(),
            language: None,
            content_rating: None,
            status: None,
            images: ImageSet::default(),
            external_ids: ExternalIds::default(),
            provider: String::new(),
            season_count: None,
            episode_count: None,
            seasons: Vec::new(),
            cast: Vec::new(),
            crew: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_metadata_default() {
        let metadata = MediaMetadata::default();

        assert!(metadata.id.is_empty());
        assert!(metadata.title.is_empty());
        assert_eq!(metadata.media_type, MediaType::Unknown);
        assert!(metadata.genres.is_empty());
        assert!(metadata.cast.is_empty());
    }

    #[test]
    fn test_external_ids_merge() {
        let mut ids1 = ExternalIds {
            imdb: Some("tt1234567".to_string()),
            tmdb: Some("123".to_string()),
            ..Default::default()
        };

        let ids2 = ExternalIds {
            tmdb: Some("456".to_string()),
            tvdb: Some("789".to_string()),
            ..Default::default()
        };

        ids1.merge(&ids2);

        assert_eq!(ids1.imdb, Some("tt1234567".to_string()));
        assert_eq!(ids1.tmdb, Some("456".to_string()));
        assert_eq!(ids1.tvdb, Some("789".to_string()));
    }

    #[test]
    fn test_external_ids_has_any() {
        let empty = ExternalIds::default();
        assert!(!empty.has_any());

        let with_imdb = ExternalIds {
            imdb: Some("tt1234567".to_string()),
            ..Default::default()
        };
        assert!(with_imdb.has_any());
    }
}

/// Image URLs for a media item
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageSet {
    /// Primary poster
    pub poster: Option<String>,
    /// Backdrop/fanart
    pub backdrop: Option<String>,
    /// Logo
    pub logo: Option<String>,
    /// Thumbnail
    pub thumb: Option<String>,
    /// Banner
    pub banner: Option<String>,
}

/// External IDs for cross-referencing
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExternalIds {
    pub imdb: Option<String>,
    pub tmdb: Option<String>,
    pub tvdb: Option<String>,
    pub anilist: Option<String>,
    pub anidb: Option<String>,
    pub mal: Option<String>,
    pub bangumi: Option<String>,
}

impl ExternalIds {
    /// Check if any ID is set
    pub fn has_any(&self) -> bool {
        self.imdb.is_some()
            || self.tmdb.is_some()
            || self.tvdb.is_some()
            || self.anilist.is_some()
            || self.anidb.is_some()
            || self.mal.is_some()
            || self.bangumi.is_some()
    }

    /// Merge with another ExternalIds, preferring non-None values from other
    pub fn merge(&mut self, other: &ExternalIds) {
        if other.imdb.is_some() {
            self.imdb = other.imdb.clone();
        }
        if other.tmdb.is_some() {
            self.tmdb = other.tmdb.clone();
        }
        if other.tvdb.is_some() {
            self.tvdb = other.tvdb.clone();
        }
        if other.anilist.is_some() {
            self.anilist = other.anilist.clone();
        }
        if other.anidb.is_some() {
            self.anidb = other.anidb.clone();
        }
        if other.mal.is_some() {
            self.mal = other.mal.clone();
        }
        if other.bangumi.is_some() {
            self.bangumi = other.bangumi.clone();
        }
    }
}

/// Season information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonInfo {
    /// Season number (0 for specials)
    pub number: i32,
    /// Season name
    pub name: Option<String>,
    /// Overview
    pub overview: Option<String>,
    /// Air date
    pub air_date: Option<String>,
    /// Episode count
    pub episode_count: Option<i32>,
    /// Poster URL
    pub poster_url: Option<String>,
}

/// Episode information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeInfo {
    /// Episode ID
    pub id: String,
    /// Episode title
    pub title: String,
    /// Season number
    pub season: i32,
    /// Episode number
    pub episode: i32,
    /// Absolute episode number (for anime)
    pub absolute_number: Option<i32>,
    /// Air date
    pub air_date: Option<String>,
    /// Overview
    pub overview: Option<String>,
    /// Runtime in minutes
    pub runtime: Option<i32>,
    /// Rating
    pub rating: Option<f64>,
    /// Still/thumbnail image
    pub still_url: Option<String>,
    /// Provider name
    pub provider: String,
}

/// Person information (cast/crew)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonInfo {
    /// Person ID
    pub id: String,
    /// Name
    pub name: String,
    /// Role/character name (for cast) or job (for crew)
    pub role: Option<String>,
    /// Profile image URL
    pub image_url: Option<String>,
    /// Order/importance
    pub order: Option<i32>,
}
