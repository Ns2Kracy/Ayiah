use serde::{Deserialize, Serialize};

/// Media type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    #[default]
    Unknown,
    Movie,
    Tv,
    Anime,
}

impl MediaType {
    /// Check if this type is compatible with another
    pub fn is_compatible_with(&self, other: MediaType) -> bool {
        match (self, other) {
            (Self::Unknown, _) | (_, Self::Unknown) => true,
            (Self::Anime, Self::Tv) | (Self::Tv, Self::Anime) => true,
            (a, b) => *a == b,
        }
    }
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "unknown"),
            Self::Movie => write!(f, "movie"),
            Self::Tv => write!(f, "tv"),
            Self::Anime => write!(f, "anime"),
        }
    }
}

/// Unified search result from any provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    /// Provider-specific ID
    pub id: String,
    /// Primary title
    pub title: String,
    /// Original/native title
    pub original_title: Option<String>,
    /// Alternative titles for matching
    pub alt_titles: Vec<String>,
    /// Media type
    pub media_type: MediaType,
    /// Release year
    pub year: Option<i32>,
    /// Poster image URL
    pub poster_url: Option<String>,
    /// Short description
    pub overview: Option<String>,
    /// Rating (0-10 scale)
    pub rating: Option<f64>,
    /// Provider name (e.g., "tmdb", "anilist")
    pub provider: String,
    /// Provider-specific score for ranking
    pub popularity: Option<f64>,
}

impl MediaInfo {
    /// Create a new MediaInfo with required fields
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            original_title: None,
            alt_titles: Vec::new(),
            media_type: MediaType::Unknown,
            year: None,
            poster_url: None,
            overview: None,
            rating: None,
            provider: provider.into(),
            popularity: None,
        }
    }

    /// Builder pattern: set media type
    pub fn with_type(mut self, media_type: MediaType) -> Self {
        self.media_type = media_type;
        self
    }

    /// Builder pattern: set year
    pub fn with_year(mut self, year: Option<i32>) -> Self {
        self.year = year;
        self
    }

    /// Builder pattern: set original title
    pub fn with_original_title(mut self, title: Option<String>) -> Self {
        self.original_title = title;
        self
    }

    /// Builder pattern: add alternative title
    pub fn with_alt_title(mut self, title: impl Into<String>) -> Self {
        self.alt_titles.push(title.into());
        self
    }

    /// Builder pattern: set poster URL
    pub fn with_poster(mut self, url: Option<String>) -> Self {
        self.poster_url = url;
        self
    }

    /// Builder pattern: set overview
    pub fn with_overview(mut self, overview: Option<String>) -> Self {
        self.overview = overview;
        self
    }

    /// Builder pattern: set rating
    pub fn with_rating(mut self, rating: Option<f64>) -> Self {
        self.rating = rating;
        self
    }

    /// Builder pattern: set popularity
    pub fn with_popularity(mut self, popularity: Option<f64>) -> Self {
        self.popularity = popularity;
        self
    }

    /// Get all titles for matching (primary + original + alternatives)
    pub fn all_titles(&self) -> Vec<&str> {
        let mut titles = vec![self.title.as_str()];
        if let Some(ref orig) = self.original_title {
            titles.push(orig.as_str());
        }
        titles.extend(self.alt_titles.iter().map(String::as_str));
        titles
    }
}
