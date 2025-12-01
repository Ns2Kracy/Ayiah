use super::api_types::{SearchResponse, MovieResult, TvResult, MovieDetails, TvDetails, EpisodeDetails, FindResponse};
use crate::scraper::{
    provider::{HttpClient, MetadataProvider, SearchOptions},
    types::{
        EpisodeInfo, ExternalIds, ImageSet, MediaInfo, MediaMetadata, MediaType, PersonInfo,
        SeasonInfo,
    },
    Result, ScraperError,
};
use async_trait::async_trait;

const TMDB_BASE_URL: &str = "https://api.themoviedb.org/3";
const TMDB_IMAGE_BASE: &str = "https://image.tmdb.org/t/p";

pub struct TmdbProvider {
    client: HttpClient,
    api_key: String,
}

impl TmdbProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: HttpClient::new(TMDB_BASE_URL),
            api_key: api_key.into(),
        }
    }

    fn image_url(&self, path: Option<&str>, size: &str) -> Option<String> {
        path.map(|p| format!("{TMDB_IMAGE_BASE}/{size}{p}"))
    }

    fn add_api_key(&self, params: &mut Vec<(&str, String)>) {
        params.push(("api_key", self.api_key.clone()));
    }

    async fn request<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
        extra_params: &[(&str, &str)],
    ) -> Result<T> {
        let mut params: Vec<(&str, String)> = Vec::new();
        self.add_api_key(&mut params);

        for (key, value) in extra_params {
            params.push((key, (*value).to_string()));
        }

        let params_ref: Vec<(&str, &str)> = params
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();

        self.client.get_with_params(endpoint, &params_ref).await
    }

    async fn search_movies(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<Vec<MediaInfo>> {
        let mut params = vec![("query", query)];
        let year_str;
        if let Some(year) = options.year {
            year_str = year.to_string();
            params.push(("year", &year_str));
        }
        let lang;
        if let Some(ref language) = options.language {
            lang = language.clone();
            params.push(("language", &lang));
        }

        let response: SearchResponse<MovieResult> =
            self.request("/search/movie", &params).await?;

        Ok(response
            .results
            .into_iter()
            .map(|m| self.movie_result_to_info(m))
            .collect())
    }

    async fn search_tv(&self, query: &str, options: &SearchOptions) -> Result<Vec<MediaInfo>> {
        let mut params = vec![("query", query)];
        let year_str;
        if let Some(year) = options.year {
            year_str = year.to_string();
            params.push(("first_air_date_year", &year_str));
        }
        let lang;
        if let Some(ref language) = options.language {
            lang = language.clone();
            params.push(("language", &lang));
        }

        let response: SearchResponse<TvResult> = self.request("/search/tv", &params).await?;

        Ok(response
            .results
            .into_iter()
            .map(|t| self.tv_result_to_info(t))
            .collect())
    }

    fn movie_result_to_info(&self, movie: MovieResult) -> MediaInfo {
        let year = movie
            .release_date
            .as_ref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok());

        MediaInfo::new(movie.id.to_string(), movie.title, "tmdb")
            .with_type(MediaType::Movie)
            .with_year(year)
            .with_original_title(Some(movie.original_title))
            .with_poster(self.image_url(movie.poster_path.as_deref(), "w500"))
            .with_overview(movie.overview)
            .with_rating(movie.vote_average)
            .with_popularity(movie.popularity)
    }

    fn tv_result_to_info(&self, tv: TvResult) -> MediaInfo {
        let year = tv
            .first_air_date
            .as_ref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok());

        MediaInfo::new(tv.id.to_string(), tv.name, "tmdb")
            .with_type(MediaType::Tv)
            .with_year(year)
            .with_original_title(Some(tv.original_name))
            .with_poster(self.image_url(tv.poster_path.as_deref(), "w500"))
            .with_overview(tv.overview)
            .with_rating(tv.vote_average)
            .with_popularity(tv.popularity)
    }

    async fn get_movie_metadata(&self, id: &str) -> Result<MediaMetadata> {
        let endpoint = format!("/movie/{id}");
        let movie: MovieDetails = self
            .request(&endpoint, &[("append_to_response", "external_ids,credits")])
            .await?;

        let year = movie
            .release_date
            .as_ref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok());

        let mut metadata = MediaMetadata {
            id: movie.id.to_string(),
            title: movie.title,
            original_title: Some(movie.original_title),
            sort_title: None,
            media_type: MediaType::Movie,
            tagline: movie.tagline,
            overview: movie.overview,
            release_date: movie.release_date,
            end_date: None,
            runtime: movie.runtime,
            rating: movie.vote_average,
            vote_count: movie.vote_count,
            genres: movie.genres.into_iter().map(|g| g.name).collect(),
            tags: Vec::new(),
            studios: movie
                .production_companies
                .into_iter()
                .map(|c| c.name)
                .collect(),
            language: Some(movie.original_language),
            content_rating: None,
            status: movie.status,
            images: ImageSet {
                poster: self.image_url(movie.poster_path.as_deref(), "w500"),
                backdrop: self.image_url(movie.backdrop_path.as_deref(), "original"),
                ..Default::default()
            },
            external_ids: ExternalIds {
                imdb: movie.external_ids.as_ref().and_then(|e| e.imdb_id.clone()),
                tmdb: Some(movie.id.to_string()),
                tvdb: movie
                    .external_ids
                    .as_ref()
                    .and_then(|e| e.tvdb_id.map(|i| i.to_string())),
                ..Default::default()
            },
            provider: "tmdb".to_string(),
            season_count: None,
            episode_count: None,
            seasons: Vec::new(),
            cast: Vec::new(),
            crew: Vec::new(),
        };

        // Add sort title
        metadata.sort_title = Some(Self::generate_sort_title(&metadata.title, year));

        // Add credits
        if let Some(credits) = movie.credits {
            metadata.cast = credits
                .cast
                .into_iter()
                .take(20)
                .map(|c| PersonInfo {
                    id: c.id.to_string(),
                    name: c.name,
                    role: c.character,
                    image_url: self.image_url(c.profile_path.as_deref(), "w185"),
                    order: c.order,
                })
                .collect();

            metadata.crew = credits
                .crew
                .into_iter()
                .filter(|c| {
                    matches!(
                        c.job.as_deref(),
                        Some("Director" | "Writer" | "Screenplay")
                    )
                })
                .map(|c| PersonInfo {
                    id: c.id.to_string(),
                    name: c.name,
                    role: c.job,
                    image_url: self.image_url(c.profile_path.as_deref(), "w185"),
                    order: None,
                })
                .collect();
        }

        Ok(metadata)
    }

    async fn get_tv_metadata(&self, id: &str) -> Result<MediaMetadata> {
        let endpoint = format!("/tv/{id}");
        let tv: TvDetails = self
            .request(&endpoint, &[("append_to_response", "external_ids,credits")])
            .await?;

        let year = tv
            .first_air_date
            .as_ref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok());

        let mut metadata = MediaMetadata {
            id: tv.id.to_string(),
            title: tv.name,
            original_title: Some(tv.original_name),
            sort_title: None,
            media_type: MediaType::Tv,
            tagline: tv.tagline,
            overview: tv.overview,
            release_date: tv.first_air_date,
            end_date: tv.last_air_date,
            runtime: tv.episode_run_time.first().copied(),
            rating: tv.vote_average,
            vote_count: tv.vote_count,
            genres: tv.genres.into_iter().map(|g| g.name).collect(),
            tags: Vec::new(),
            studios: tv
                .production_companies
                .into_iter()
                .map(|c| c.name)
                .collect(),
            language: Some(tv.original_language),
            content_rating: None,
            status: tv.status,
            images: ImageSet {
                poster: self.image_url(tv.poster_path.as_deref(), "w500"),
                backdrop: self.image_url(tv.backdrop_path.as_deref(), "original"),
                ..Default::default()
            },
            external_ids: ExternalIds {
                imdb: tv.external_ids.as_ref().and_then(|e| e.imdb_id.clone()),
                tmdb: Some(tv.id.to_string()),
                tvdb: tv
                    .external_ids
                    .as_ref()
                    .and_then(|e| e.tvdb_id.map(|i| i.to_string())),
                ..Default::default()
            },
            provider: "tmdb".to_string(),
            season_count: Some(tv.number_of_seasons),
            episode_count: Some(tv.number_of_episodes),
            seasons: tv
                .seasons
                .into_iter()
                .map(|s| SeasonInfo {
                    number: s.season_number,
                    name: s.name,
                    overview: s.overview,
                    air_date: s.air_date,
                    episode_count: s.episode_count,
                    poster_url: self.image_url(s.poster_path.as_deref(), "w500"),
                })
                .collect(),
            cast: Vec::new(),
            crew: Vec::new(),
        };

        // Add sort title
        metadata.sort_title = Some(Self::generate_sort_title(&metadata.title, year));

        // Add credits
        if let Some(credits) = tv.credits {
            metadata.cast = credits
                .cast
                .into_iter()
                .take(20)
                .map(|c| PersonInfo {
                    id: c.id.to_string(),
                    name: c.name,
                    role: c.character,
                    image_url: self.image_url(c.profile_path.as_deref(), "w185"),
                    order: c.order,
                })
                .collect();

            metadata.crew = credits
                .crew
                .into_iter()
                .filter(|c| {
                    matches!(
                        c.job.as_deref(),
                        Some("Director" | "Writer" | "Creator" | "Executive Producer")
                    )
                })
                .map(|c| PersonInfo {
                    id: c.id.to_string(),
                    name: c.name,
                    role: c.job,
                    image_url: self.image_url(c.profile_path.as_deref(), "w185"),
                    order: None,
                })
                .collect();
        }

        Ok(metadata)
    }

    fn generate_sort_title(title: &str, year: Option<i32>) -> String {
        let sort_title = title
            .trim_start_matches("The ")
            .trim_start_matches("A ")
            .trim_start_matches("An ");

        if let Some(year) = year {
            format!("{sort_title} ({year})")
        } else {
            sort_title.to_string()
        }
    }
}

#[async_trait]
impl MetadataProvider for TmdbProvider {
    fn id(&self) -> &'static str {
        "tmdb"
    }

    fn name(&self) -> &'static str {
        "The Movie Database"
    }

    fn supported_types(&self) -> &[MediaType] {
        &[MediaType::Movie, MediaType::Tv]
    }

    fn requires_api_key(&self) -> bool {
        true
    }

    fn priority_for(&self, media_type: MediaType) -> i32 {
        match media_type {
            MediaType::Movie => 100,
            MediaType::Tv => 90,
            MediaType::Anime => 30, // Can handle anime but not preferred
            MediaType::Unknown => 50,
        }
    }

    async fn search(&self, query: &str, options: &SearchOptions) -> Result<Vec<MediaInfo>> {
        let mut results = Vec::new();

        // Search based on media type filter
        match options.media_type {
            Some(MediaType::Movie) => {
                results.extend(self.search_movies(query, options).await?);
            }
            Some(MediaType::Tv | MediaType::Anime) => {
                results.extend(self.search_tv(query, options).await?);
            }
            _ => {
                // Search both
                if let Ok(movies) = self.search_movies(query, options).await {
                    results.extend(movies);
                }
                if let Ok(tv) = self.search_tv(query, options).await {
                    results.extend(tv);
                }
            }
        }

        if results.is_empty() {
            return Err(ScraperError::NotFound(format!(
                "No results found for: {query}"
            )));
        }

        // Apply limit
        if let Some(limit) = options.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    async fn get_metadata(&self, id: &str, media_type: MediaType) -> Result<MediaMetadata> {
        match media_type {
            MediaType::Movie => self.get_movie_metadata(id).await,
            MediaType::Tv | MediaType::Anime => self.get_tv_metadata(id).await,
            MediaType::Unknown => {
                // Try movie first, then TV
                if let Ok(metadata) = self.get_movie_metadata(id).await {
                    return Ok(metadata);
                }
                self.get_tv_metadata(id).await
            }
        }
    }

    async fn get_episode(
        &self,
        series_id: &str,
        season: i32,
        episode: i32,
    ) -> Result<EpisodeInfo> {
        let endpoint = format!("/tv/{series_id}/season/{season}/episode/{episode}");
        let ep: EpisodeDetails = self.request(&endpoint, &[]).await?;

        Ok(EpisodeInfo {
            id: ep.id.to_string(),
            title: ep.name,
            season: ep.season_number,
            episode: ep.episode_number,
            absolute_number: None,
            air_date: ep.air_date,
            overview: ep.overview,
            runtime: ep.runtime,
            rating: ep.vote_average,
            still_url: self.image_url(ep.still_path.as_deref(), "w300"),
            provider: "tmdb".to_string(),
        })
    }

    async fn find_by_external_id(
        &self,
        external_id: &str,
        source: &str,
    ) -> Result<Option<MediaInfo>> {
        let source_param = match source {
            "imdb" => "imdb_id",
            "tvdb" => "tvdb_id",
            _ => return Ok(None),
        };

        let endpoint = format!("/find/{external_id}");
        let response: FindResponse = self
            .request(&endpoint, &[("external_source", source_param)])
            .await?;

        // Return first movie or TV result
        if let Some(movie) = response.movie_results.into_iter().next() {
            return Ok(Some(self.movie_result_to_info(movie)));
        }
        if let Some(tv) = response.tv_results.into_iter().next() {
            return Ok(Some(self.tv_result_to_info(tv)));
        }

        Ok(None)
    }
}
