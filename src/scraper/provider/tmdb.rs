use super::{ProviderBase, ProviderConfig};
use crate::scraper::{
    EpisodeMetadata, ExternalIds, MediaDetails, MediaSearchResult, MetadataProvider, MovieMetadata,
    MovieSearchResult, Result, ScraperError, TvMetadata, TvSearchResult,
};
use async_trait::async_trait;
use serde::Deserialize;

const TMDB_BASE_URL: &str = "https://api.themoviedb.org/3";
const TMDB_IMAGE_BASE: &str = "https://image.tmdb.org/t/p";

/// TMDB Provider
pub struct TmdbProvider {
    base: ProviderBase,
    api_key: String,
}

impl TmdbProvider {
    /// Create a new TMDB provider
    pub fn new(api_key: impl Into<String>) -> Self {
        let api_key = api_key.into();
        let config = ProviderConfig::new(TMDB_BASE_URL).with_api_key(api_key.clone());

        Self {
            base: ProviderBase::new(config),
            api_key,
        }
    }

    /// Build complete image URL
    #[allow(clippy::single_option_map)]
    fn build_image_url(&self, path: Option<&str>, size: &str) -> Option<String> {
        path.map(|p| format!("{TMDB_IMAGE_BASE}/{size}{p}"))
    }

    /// Execute TMDB API request
    async fn request<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<T> {
        let mut url = format!("{TMDB_BASE_URL}{endpoint}");
        let mut query_params = vec![("api_key", self.api_key.as_str())];
        query_params.extend_from_slice(params);

        let query_string = query_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        url.push('?');
        url.push_str(&query_string);

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
            .map_err(|e| ScraperError::Parse(format!("Failed to parse TMDB response: {e}")))
    }
}

#[async_trait]
impl MetadataProvider for TmdbProvider {
    fn name(&self) -> &'static str {
        "tmdb"
    }

    fn requires_api_key(&self) -> bool {
        true
    }

    async fn search(&self, query: &str, year: Option<i32>) -> Result<Vec<MediaSearchResult>> {
        let mut results = Vec::new();

        // TMDB supports movie and TV show searches
        if let Ok(movies) = self.search_movie_internal(query, year).await {
            results.extend(movies.into_iter().map(MediaSearchResult::Movie));
        }

        if let Ok(tv_shows) = self.search_tv_internal(query, year).await {
            results.extend(tv_shows.into_iter().map(MediaSearchResult::Tv));
        }

        if results.is_empty() {
            Err(ScraperError::NotFound(format!(
                "No results found for: {query}"
            )))
        } else {
            Ok(results)
        }
    }

    async fn get_details(&self, result: &MediaSearchResult) -> Result<MediaDetails> {
        match result {
            MediaSearchResult::Movie(m) => self
                .get_movie_details_internal(&m.id)
                .await
                .map(MediaDetails::Movie),
            MediaSearchResult::Tv(t) => self
                .get_tv_details_internal(&t.id)
                .await
                .map(MediaDetails::Tv),
            MediaSearchResult::Anime(_) => Err(ScraperError::Config(
                "TMDB does not support anime".to_string(),
            )),
        }
    }

    async fn get_episode_details(
        &self,
        series_id: &str,
        season: i32,
        episode: i32,
    ) -> Result<EpisodeMetadata> {
        let endpoint = format!("/tv/{series_id}/season/{season}/episode/{episode}");
        let ep: TmdbEpisodeDetails = self.request(&endpoint, &[]).await?;

        Ok(EpisodeMetadata {
            id: ep.id.to_string(),
            name: ep.name,
            season_number: ep.season_number,
            episode_number: ep.episode_number,
            air_date: ep.air_date,
            overview: ep.overview,
            still_path: self.build_image_url(ep.still_path.as_deref(), "w300"),
            runtime: ep.runtime,
            vote_average: ep.vote_average,
            provider: "tmdb".to_string(),
        })
    }
}

impl TmdbProvider {
    // Private helper methods
    async fn search_movie_internal(
        &self,
        query: &str,
        year: Option<i32>,
    ) -> Result<Vec<MovieSearchResult>> {
        let mut params = vec![("query", query)];
        let year_str = year.map(|y| y.to_string());
        if let Some(ref y) = year_str {
            params.push(("year", y.as_str()));
        }

        let response: TmdbSearchResponse = self.request("/search/movie", &params).await?;

        Ok(response
            .results
            .into_iter()
            .map(|movie| MovieSearchResult {
                id: movie.id.to_string(),
                title: movie.title,
                original_title: Some(movie.original_title),
                year: movie
                    .release_date
                    .as_ref()
                    .and_then(|d| d.split('-').next().and_then(|y| y.parse().ok())),
                poster_path: self.build_image_url(movie.poster_path.as_deref(), "w500"),
                overview: movie.overview,
                vote_average: movie.vote_average,
                provider: "tmdb".to_string(),
            })
            .collect())
    }

    async fn get_movie_details_internal(&self, id: &str) -> Result<MovieMetadata> {
        let params = vec![("append_to_response", "external_ids")];
        let movie: TmdbMovieDetails = self.request(&format!("/movie/{id}"), &params).await?;

        Ok(MovieMetadata {
            id: movie.id.to_string(),
            title: movie.title,
            original_title: Some(movie.original_title),
            release_date: movie.release_date,
            runtime: movie.runtime,
            overview: movie.overview,
            poster_path: self.build_image_url(movie.poster_path.as_deref(), "w500"),
            backdrop_path: self.build_image_url(movie.backdrop_path.as_deref(), "original"),
            vote_average: movie.vote_average,
            vote_count: movie.vote_count,
            genres: movie.genres.into_iter().map(|g| g.name).collect(),
            production_companies: movie
                .production_companies
                .into_iter()
                .map(|c| c.name)
                .collect(),
            production_countries: movie
                .production_countries
                .into_iter()
                .map(|c| c.name)
                .collect(),
            original_language: Some(movie.original_language),
            provider: "tmdb".to_string(),
            external_ids: ExternalIds {
                imdb_id: movie.external_ids.as_ref().and_then(|e| e.imdb_id.clone()),
                tmdb_id: Some(movie.id.to_string()),
                tvdb_id: movie
                    .external_ids
                    .as_ref()
                    .and_then(|e| e.tvdb_id.map(|i| i.to_string())),
                ..Default::default()
            },
        })
    }

    async fn search_tv_internal(
        &self,
        query: &str,
        year: Option<i32>,
    ) -> Result<Vec<TvSearchResult>> {
        let mut params = vec![("query", query)];
        let year_str = year.map(|y| y.to_string());
        if let Some(ref y) = year_str {
            params.push(("first_air_date_year", y.as_str()));
        }

        let response: TmdbTvSearchResponse = self.request("/search/tv", &params).await?;

        Ok(response
            .results
            .into_iter()
            .map(|tv| TvSearchResult {
                id: tv.id.to_string(),
                name: tv.name,
                original_name: Some(tv.original_name),
                first_air_date: tv.first_air_date,
                poster_path: self.build_image_url(tv.poster_path.as_deref(), "w500"),
                overview: tv.overview,
                vote_average: tv.vote_average,
                provider: "tmdb".to_string(),
            })
            .collect())
    }

    async fn get_tv_details_internal(&self, id: &str) -> Result<TvMetadata> {
        let params = vec![("append_to_response", "external_ids")];
        let tv: TmdbTvDetails = self.request(&format!("/tv/{id}"), &params).await?;

        Ok(TvMetadata {
            id: tv.id.to_string(),
            name: tv.name,
            original_name: Some(tv.original_name),
            first_air_date: tv.first_air_date,
            last_air_date: tv.last_air_date,
            overview: tv.overview,
            poster_path: self.build_image_url(tv.poster_path.as_deref(), "w500"),
            backdrop_path: self.build_image_url(tv.backdrop_path.as_deref(), "original"),
            vote_average: tv.vote_average,
            vote_count: tv.vote_count,
            genres: tv.genres.into_iter().map(|g| g.name).collect(),
            number_of_seasons: Some(tv.number_of_seasons),
            number_of_episodes: Some(tv.number_of_episodes),
            episode_run_time: tv.episode_run_time,
            status: Some(tv.status),
            original_language: Some(tv.original_language),
            production_companies: tv
                .production_companies
                .into_iter()
                .map(|c| c.name)
                .collect(),
            provider: "tmdb".to_string(),
            external_ids: ExternalIds {
                imdb_id: tv.external_ids.as_ref().and_then(|e| e.imdb_id.clone()),
                tmdb_id: Some(tv.id.to_string()),
                tvdb_id: tv
                    .external_ids
                    .as_ref()
                    .and_then(|e| e.tvdb_id.map(|i| i.to_string())),
                ..Default::default()
            },
        })
    }
}

// TMDB API Response Types
#[derive(Debug, Deserialize)]
struct TmdbSearchResponse {
    results: Vec<TmdbMovieSearchResult>,
}

#[derive(Debug, Deserialize)]
struct TmdbMovieSearchResult {
    id: i64,
    title: String,
    original_title: String,
    release_date: Option<String>,
    poster_path: Option<String>,
    overview: Option<String>,
    vote_average: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct TmdbMovieDetails {
    id: i64,
    title: String,
    original_title: String,
    release_date: Option<String>,
    runtime: Option<i32>,
    overview: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    vote_average: Option<f64>,
    vote_count: Option<i32>,
    genres: Vec<TmdbGenre>,
    production_companies: Vec<TmdbCompany>,
    production_countries: Vec<TmdbCountry>,
    original_language: String,
    external_ids: Option<TmdbExternalIds>,
}

#[derive(Debug, Deserialize)]
struct TmdbTvSearchResponse {
    results: Vec<TmdbTvSearchResult>,
}

#[derive(Debug, Deserialize)]
struct TmdbTvSearchResult {
    id: i64,
    name: String,
    original_name: String,
    first_air_date: Option<String>,
    poster_path: Option<String>,
    overview: Option<String>,
    vote_average: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct TmdbTvDetails {
    id: i64,
    name: String,
    original_name: String,
    first_air_date: Option<String>,
    last_air_date: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    vote_average: Option<f64>,
    vote_count: Option<i32>,
    genres: Vec<TmdbGenre>,
    number_of_seasons: i32,
    number_of_episodes: i32,
    episode_run_time: Vec<i32>,
    status: String,
    original_language: String,
    production_companies: Vec<TmdbCompany>,
    external_ids: Option<TmdbExternalIds>,
}

#[derive(Debug, Deserialize)]
struct TmdbEpisodeDetails {
    id: i64,
    name: String,
    season_number: i32,
    episode_number: i32,
    air_date: Option<String>,
    overview: Option<String>,
    still_path: Option<String>,
    runtime: Option<i32>,
    vote_average: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct TmdbGenre {
    #[allow(dead_code)]
    id: i64,
    name: String,
}

#[derive(Debug, Deserialize)]
struct TmdbCompany {
    #[allow(dead_code)]
    id: i64,
    name: String,
}

#[derive(Debug, Deserialize)]
struct TmdbCountry {
    #[allow(dead_code)]
    iso_3166_1: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct TmdbExternalIds {
    imdb_id: Option<String>,
    tvdb_id: Option<i64>,
}
