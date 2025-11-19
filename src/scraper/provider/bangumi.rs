use super::{ProviderBase, ProviderConfig};
use crate::scraper::{
    AnimeMetadata, AnimeSearchResult, EpisodeMetadata, ExternalIds, MediaDetails,
    MediaSearchResult, MetadataProvider, Result, ScraperError,
};
use async_trait::async_trait;
use serde::Deserialize;

const BANGUMI_API_URL: &str = "https://api.bgm.tv";

/// Bangumi Provider
pub struct BangumiProvider {
    base: ProviderBase,
}

impl Default for BangumiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl BangumiProvider {
    /// Create a new Bangumi provider (no API key required)
    pub fn new() -> Self {
        Self {
            base: ProviderBase::new(ProviderConfig::new(BANGUMI_API_URL)),
        }
    }

    /// Execute Bangumi API request
    async fn request<T: for<'de> Deserialize<'de>>(&self, endpoint: &str) -> Result<T> {
        let url = format!("{BANGUMI_API_URL}{endpoint}");

        let response = self.base.get(&url).await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(ScraperError::Api {
                status,
                message: text,
            });
        }

        response
            .json::<T>()
            .await
            .map_err(|e| ScraperError::Parse(format!("Failed to parse Bangumi response: {e}")))
    }

    // Private helper methods
    async fn search_anime_internal(
        &self,
        query: &str,
        _year: Option<i32>,
    ) -> Result<Vec<AnimeSearchResult>> {
        let encoded_query = urlencoding::encode(query);
        let endpoint = format!("/search/subject/{encoded_query}?type=2&responseGroup=small");

        let response: BangumiSearchResponse = self.request(&endpoint).await?;

        Ok(response
            .list
            .unwrap_or_default()
            .into_iter()
            .map(|subject| AnimeSearchResult {
                id: subject.id.to_string(),
                title: subject
                    .name_cn
                    .clone()
                    .unwrap_or_else(|| subject.name.clone()),
                title_english: None,
                title_japanese: Some(subject.name),
                year: subject
                    .air_date
                    .as_ref()
                    .and_then(|d| d.split('-').next())
                    .and_then(|y| y.parse().ok()),
                poster_path: subject.images.as_ref().map(|i| i.large.clone()),
                overview: subject.summary,
                score: subject.score,
                provider: "bangumi".to_string(),
            })
            .collect())
    }

    async fn get_anime_details_internal(&self, id: &str) -> Result<AnimeMetadata> {
        let endpoint = format!("/v0/subjects/{id}");
        let subject: BangumiSubject = self.request(&endpoint).await?;

        // Extract titles
        let title_cn = subject
            .name_cn
            .clone()
            .unwrap_or_else(|| subject.name.clone());
        let title_jp = subject.name.clone();

        // Extract date
        let start_date = subject.date.clone();

        // Extract format
        let format = match subject.type_ {
            2 => "TV".to_string(),
            6 => "Movie".to_string(),
            _ => "Unknown".to_string(),
        };

        Ok(AnimeMetadata {
            id: subject.id.to_string(),
            title: title_cn,
            title_english: None,
            title_japanese: Some(title_jp),
            start_date,
            end_date: None,
            overview: subject.summary,
            poster_path: subject.images.as_ref().map(|i| i.large.clone()),
            backdrop_path: None,
            score: subject.rating.as_ref().and_then(|r| r.score),
            genres: subject.tags.into_iter().map(|t| t.name).collect(),
            episodes: subject.eps,
            status: None,
            format: Some(format),
            provider: "bangumi".to_string(),
            external_ids: ExternalIds {
                bangumi_id: Some(subject.id.to_string()),
                ..Default::default()
            },
        })
    }
}

#[async_trait]
impl MetadataProvider for BangumiProvider {
    fn name(&self) -> &'static str {
        "bangumi"
    }

    fn requires_api_key(&self) -> bool {
        false
    }

    async fn search(&self, query: &str, year: Option<i32>) -> Result<Vec<MediaSearchResult>> {
        // Bangumi only supports anime/manga searches
        let anime = self.search_anime_internal(query, year).await?;
        Ok(anime.into_iter().map(MediaSearchResult::Anime).collect())
    }

    async fn get_details(&self, result: &MediaSearchResult) -> Result<MediaDetails> {
        match result {
            MediaSearchResult::Anime(a) => self
                .get_anime_details_internal(&a.id)
                .await
                .map(MediaDetails::Anime),
            MediaSearchResult::Movie(_) => Err(ScraperError::Config(
                "Bangumi specializes in anime/manga".to_string(),
            )),
            MediaSearchResult::Tv(_) => Err(ScraperError::Config(
                "Bangumi specializes in anime/manga".to_string(),
            )),
        }
    }

    async fn get_episode_details(
        &self,
        _series_id: &str,
        _season: i32,
        _episode: i32,
    ) -> Result<EpisodeMetadata> {
        Err(ScraperError::Config(
            "Bangumi does not provide individual episode details".to_string(),
        ))
    }
}

// Bangumi API Response Types
#[derive(Debug, Deserialize)]
struct BangumiSearchResponse {
    list: Option<Vec<BangumiSearchSubject>>,
}

#[derive(Debug, Deserialize)]
struct BangumiSearchSubject {
    id: i32,
    name: String,
    name_cn: Option<String>,
    #[serde(rename = "air_date")]
    air_date: Option<String>,
    images: Option<BangumiImages>,
    summary: Option<String>,
    score: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct BangumiSubject {
    id: i32,
    #[serde(rename = "type")]
    type_: i32,
    name: String,
    name_cn: Option<String>,
    summary: Option<String>,
    date: Option<String>,
    images: Option<BangumiImages>,
    eps: Option<i32>,
    rating: Option<BangumiRating>,
    tags: Vec<BangumiTag>,
}

#[derive(Debug, Deserialize)]
struct BangumiImages {
    large: String,
}

#[derive(Debug, Deserialize)]
struct BangumiRating {
    score: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct BangumiTag {
    name: String,
}
