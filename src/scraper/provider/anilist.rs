use super::ProviderBase;
use crate::scraper::{
    AnimeMetadata, AnimeSearchResult, EpisodeMetadata, ExternalIds, MediaDetails,
    MediaSearchResult, MetadataProvider, Result, ScraperError,
};
use async_trait::async_trait;
use serde::Deserialize;

const ANILIST_API_URL: &str = "https://graphql.anilist.co";

/// `AniList` Provider
#[derive(Default)]
pub struct AniListProvider {
    base: ProviderBase,
}

impl AniListProvider {
    /// Execute GraphQL query
    async fn query<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<T> {
        let body = serde_json::json!({
            "query": query,
            "variables": variables
        });

        let response = self
            .base
            .client
            .post(ANILIST_API_URL)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(ScraperError::Network)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(ScraperError::Api {
                status,
                message: text,
            });
        }

        let result: AniListResponse<T> = response
            .json()
            .await
            .map_err(|e| ScraperError::Parse(format!("Failed to parse AniList response: {e}")))?;

        result
            .data
            .ok_or_else(|| ScraperError::Parse("No data in response".to_string()))
    }

    // Private helper methods
    async fn search_anime_internal(
        &self,
        query: &str,
        year: Option<i32>,
    ) -> Result<Vec<AnimeSearchResult>> {
        let gql_query = r"
            query ($search: String, $year: Int) {
                Page(page: 1, perPage: 20) {
                    media(search: $search, seasonYear: $year, type: ANIME) {
                        id
                        title {
                            romaji
                            english
                            native
                        }
                        seasonYear
                        coverImage {
                            large
                        }
                        description
                        averageScore
                    }
                }
            }
        ";

        let variables = serde_json::json!({
            "search": query,
            "year": year
        });

        let response: AniListSearchData = self.query(gql_query, variables).await?;

        Ok(response
            .page
            .media
            .into_iter()
            .map(|anime| AnimeSearchResult {
                id: anime.id.to_string(),
                title: anime.title.romaji,
                title_english: anime.title.english,
                title_japanese: Some(anime.title.native),
                year: anime.season_year,
                poster_path: Some(anime.cover_image.large),
                overview: anime.description,
                score: anime.average_score.map(|s| f64::from(s) / 10.0),
                provider: "anilist".to_string(),
            })
            .collect())
    }

    async fn get_anime_details_internal(&self, id: &str) -> Result<AnimeMetadata> {
        let gql_query = r"
            query ($id: Int) {
                Media(id: $id, type: ANIME) {
                    id
                    title {
                        romaji
                        english
                        native
                    }
                    startDate {
                        year
                        month
                        day
                    }
                    endDate {
                        year
                        month
                        day
                    }
                    description
                    coverImage {
                        large
                    }
                    bannerImage
                    averageScore
                    genres
                    episodes
                    status
                    format
                    idMal
                }
            }
        ";

        let anime_id: i32 = id
            .parse()
            .map_err(|_| ScraperError::Parse(format!("Invalid AniList ID: {id}")))?;

        let variables = serde_json::json!({
            "id": anime_id
        });

        let response: AniListMediaData = self.query(gql_query, variables).await?;
        let anime = response.media;

        // Format date
        let format_date = |date: Option<&AniListDate>| -> Option<String> {
            date.and_then(|d| {
                if let (Some(y), Some(m), Some(day)) = (d.year, d.month, d.day) {
                    Some(format!("{y:04}-{m:02}-{day:02}"))
                } else {
                    None
                }
            })
        };

        Ok(AnimeMetadata {
            id: anime.id.to_string(),
            title: anime.title.romaji,
            title_english: anime.title.english,
            title_japanese: Some(anime.title.native),
            start_date: format_date(anime.start_date.as_ref()),
            end_date: format_date(anime.end_date.as_ref()),
            overview: anime.description,
            poster_path: Some(anime.cover_image.large),
            backdrop_path: anime.banner_image,
            score: anime.average_score.map(|s| f64::from(s) / 10.0),
            genres: anime.genres,
            episodes: anime.episodes,
            status: Some(anime.status),
            format: Some(anime.format),
            provider: "anilist".to_string(),
            external_ids: ExternalIds {
                anilist_id: Some(anime.id.to_string()),
                mal_id: anime.id_mal.map(|id| id.to_string()),
                ..Default::default()
            },
        })
    }
}

#[async_trait]
impl MetadataProvider for AniListProvider {
    fn name(&self) -> &'static str {
        "anilist"
    }

    fn requires_api_key(&self) -> bool {
        false
    }

    async fn search(&self, query: &str, year: Option<i32>) -> Result<Vec<MediaSearchResult>> {
        // AniList only supports anime searches
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
                "AniList specializes in anime".to_string(),
            )),
            MediaSearchResult::Tv(_) => Err(ScraperError::Config(
                "AniList specializes in anime".to_string(),
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
            "AniList does not provide individual episode details".to_string(),
        ))
    }
}

// AniList API Response Types
#[derive(Debug, Deserialize)]
struct AniListResponse<T> {
    data: Option<T>,
}

#[derive(Debug, Deserialize)]
struct AniListSearchData {
    #[serde(rename = "Page")]
    page: AniListPage,
}

#[derive(Debug, Deserialize)]
struct AniListPage {
    media: Vec<AniListSearchMedia>,
}

#[derive(Debug, Deserialize)]
struct AniListSearchMedia {
    id: i32,
    title: AniListTitle,
    #[serde(rename = "seasonYear")]
    season_year: Option<i32>,
    #[serde(rename = "coverImage")]
    cover_image: AniListCoverImage,
    description: Option<String>,
    #[serde(rename = "averageScore")]
    average_score: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct AniListMediaData {
    #[serde(rename = "Media")]
    media: AniListMedia,
}

#[derive(Debug, Deserialize)]
struct AniListMedia {
    id: i32,
    title: AniListTitle,
    #[serde(rename = "startDate")]
    start_date: Option<AniListDate>,
    #[serde(rename = "endDate")]
    end_date: Option<AniListDate>,
    description: Option<String>,
    #[serde(rename = "coverImage")]
    cover_image: AniListCoverImage,
    #[serde(rename = "bannerImage")]
    banner_image: Option<String>,
    #[serde(rename = "averageScore")]
    average_score: Option<i32>,
    genres: Vec<String>,
    episodes: Option<i32>,
    status: String,
    format: String,
    #[serde(rename = "idMal")]
    id_mal: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct AniListTitle {
    romaji: String,
    english: Option<String>,
    native: String,
}

#[derive(Debug, Deserialize)]
struct AniListCoverImage {
    large: String,
}

#[derive(Debug, Deserialize)]
struct AniListDate {
    year: Option<i32>,
    month: Option<i32>,
    day: Option<i32>,
}
