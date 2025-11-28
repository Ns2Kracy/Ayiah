use super::api_types::*;
use crate::scraper::{
    Result, ScraperError,
    provider::{HttpClient, MetadataProvider, SearchOptions},
    types::{EpisodeInfo, ExternalIds, ImageSet, MediaInfo, MediaMetadata, MediaType, PersonInfo},
};
use async_trait::async_trait;

const ANILIST_API_URL: &str = "https://graphql.anilist.co";

pub struct AniListProvider {
    client: HttpClient,
}

impl Default for AniListProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AniListProvider {
    pub fn new() -> Self {
        Self {
            client: HttpClient::new(ANILIST_API_URL),
        }
    }

    async fn query<T: serde::de::DeserializeOwned>(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<T> {
        let body = serde_json::json!({
            "query": query,
            "variables": variables
        });

        let response: GraphQLResponse<T> = self.client.post_json("", &body).await?;

        if let Some(errors) = response.errors
            && let Some(error) = errors.first()
        {
            return Err(ScraperError::Api {
                status: 400,
                message: error.message.clone(),
            });
        }

        response
            .data
            .ok_or_else(|| ScraperError::Parse("No data in response".to_string()))
    }

    fn media_to_info(&self, media: &Media) -> MediaInfo {
        let title = media
            .title
            .english
            .clone()
            .or_else(|| media.title.romaji.clone())
            .unwrap_or_default();

        let mut info = MediaInfo::new(media.id.to_string(), title, "anilist")
            .with_type(MediaType::Anime)
            .with_year(media.season_year)
            .with_original_title(media.title.native.clone())
            .with_overview(media.description.clone())
            .with_rating(media.average_score.map(|s| f64::from(s) / 10.0))
            .with_popularity(media.popularity.map(f64::from));

        // Add poster
        if let Some(ref cover) = media.cover_image {
            info = info.with_poster(cover.extra_large.clone().or_else(|| cover.large.clone()));
        }

        // Add alternative titles
        if let Some(ref romaji) = media.title.romaji {
            info = info.with_alt_title(romaji.clone());
        }
        if let Some(ref synonyms) = media.synonyms {
            for syn in synonyms {
                info = info.with_alt_title(syn.clone());
            }
        }

        info
    }

    fn media_to_metadata(&self, media: Media) -> MediaMetadata {
        let title = media
            .title
            .english
            .clone()
            .or_else(|| media.title.romaji.clone())
            .unwrap_or_default();

        let mut metadata = MediaMetadata {
            id: media.id.to_string(),
            title: title.clone(),
            original_title: media.title.native.clone(),
            sort_title: Some(title),
            media_type: MediaType::Anime,
            tagline: None,
            overview: media.description.map(|d| {
                // Remove HTML tags from description
                let re = regex::Regex::new(r"<[^>]+>").expect("Invalid regex");
                re.replace_all(&d, "").to_string()
            }),
            release_date: media.start_date.as_ref().and_then(|d| d.to_string()),
            end_date: media.end_date.as_ref().and_then(|d| d.to_string()),
            runtime: media.duration,
            rating: media.average_score.map(|s| f64::from(s) / 10.0),
            vote_count: media.popularity,
            genres: media.genres.unwrap_or_default(),
            tags: media
                .tags
                .unwrap_or_default()
                .into_iter()
                .filter(|t| t.rank.unwrap_or(0) >= 60)
                .map(|t| t.name)
                .collect(),
            studios: media
                .studios
                .map(|s| {
                    s.nodes
                        .into_iter()
                        .filter(|studio| studio.is_animation_studio)
                        .map(|studio| studio.name)
                        .collect()
                })
                .unwrap_or_default(),
            language: Some("ja".to_string()),
            content_rating: None,
            status: media.status,
            images: ImageSet {
                poster: media
                    .cover_image
                    .as_ref()
                    .and_then(|c| c.extra_large.clone().or_else(|| c.large.clone())),
                backdrop: media.banner_image,
                ..Default::default()
            },
            external_ids: ExternalIds {
                anilist: Some(media.id.to_string()),
                mal: media.id_mal.map(|id| id.to_string()),
                ..Default::default()
            },
            provider: "anilist".to_string(),
            season_count: None,
            episode_count: media.episodes,
            seasons: Vec::new(),
            cast: Vec::new(),
            crew: Vec::new(),
        };

        // Add characters as cast
        if let Some(characters) = media.characters {
            metadata.cast = characters
                .edges
                .into_iter()
                .filter(|e| matches!(e.role.as_deref(), Some("MAIN") | Some("SUPPORTING")))
                .take(20)
                .map(|edge| {
                    let character_name = edge.node.name.full.unwrap_or_default();
                    let voice_actor = edge
                        .voice_actors
                        .and_then(|vas| {
                            vas.into_iter()
                                .find(|va| va.language.as_deref() == Some("Japanese"))
                        })
                        .map(|va| va.name.full.unwrap_or_default());

                    PersonInfo {
                        id: edge.node.id.to_string(),
                        name: voice_actor.unwrap_or_else(|| character_name.clone()),
                        role: Some(character_name),
                        image_url: edge.node.image.and_then(|i| i.large),
                        order: None,
                    }
                })
                .collect();
        }

        // Add staff as crew
        if let Some(staff) = media.staff {
            metadata.crew = staff
                .edges
                .into_iter()
                .filter(|e| {
                    matches!(
                        e.role.as_deref(),
                        Some("Director")
                            | Some("Original Creator")
                            | Some("Series Composition")
                            | Some("Music")
                    )
                })
                .map(|edge| PersonInfo {
                    id: edge.node.id.to_string(),
                    name: edge.node.name.full.unwrap_or_default(),
                    role: edge.role,
                    image_url: edge.node.image.and_then(|i| i.large),
                    order: None,
                })
                .collect();
        }

        metadata
    }
}

#[async_trait]
impl MetadataProvider for AniListProvider {
    fn id(&self) -> &'static str {
        "anilist"
    }

    fn name(&self) -> &'static str {
        "AniList"
    }

    fn supported_types(&self) -> &[MediaType] {
        &[MediaType::Anime]
    }

    fn requires_api_key(&self) -> bool {
        false
    }

    fn priority_for(&self, media_type: MediaType) -> i32 {
        match media_type {
            MediaType::Anime => 100,
            MediaType::Tv => 20, // Some anime might be categorized as TV
            _ => 0,
        }
    }

    async fn search(&self, query: &str, options: &SearchOptions) -> Result<Vec<MediaInfo>> {
        let gql_query = r#"
            query ($search: String, $year: Int, $perPage: Int) {
                Page(page: 1, perPage: $perPage) {
                    media(search: $search, seasonYear: $year, type: ANIME, sort: SEARCH_MATCH) {
                        id
                        title { romaji english native }
                        format
                        status
                        description
                        seasonYear
                        episodes
                        duration
                        coverImage { large extraLarge }
                        bannerImage
                        averageScore
                        popularity
                        genres
                        synonyms
                        idMal
                    }
                }
            }
        "#;

        let variables = serde_json::json!({
            "search": query,
            "year": options.year,
            "perPage": options.limit.unwrap_or(20)
        });

        let data: SearchData = self.query(gql_query, variables).await?;

        if data.page.media.is_empty() {
            return Err(ScraperError::NotFound(format!(
                "No anime found for: {query}"
            )));
        }

        Ok(data
            .page
            .media
            .iter()
            .map(|m| self.media_to_info(m))
            .collect())
    }

    async fn get_metadata(&self, id: &str, _media_type: MediaType) -> Result<MediaMetadata> {
        let gql_query = r#"
            query ($id: Int) {
                Media(id: $id, type: ANIME) {
                    id
                    title { romaji english native }
                    format
                    status
                    description
                    season
                    seasonYear
                    episodes
                    duration
                    coverImage { large extraLarge }
                    bannerImage
                    averageScore
                    popularity
                    genres
                    tags { name rank }
                    studios { nodes { name isAnimationStudio } }
                    startDate { year month day }
                    endDate { year month day }
                    idMal
                    synonyms
                    characters(sort: ROLE, perPage: 25) {
                        edges {
                            node { id name { full } image { large } }
                            role
                            voiceActors(language: JAPANESE) {
                                id name { full } image { large } language
                            }
                        }
                    }
                    staff(perPage: 10) {
                        edges {
                            node { id name { full } image { large } }
                            role
                        }
                    }
                }
            }
        "#;

        let anime_id: i32 = id
            .parse()
            .map_err(|_| ScraperError::Parse(format!("Invalid AniList ID: {id}")))?;

        let variables = serde_json::json!({ "id": anime_id });

        let data: MediaData = self.query(gql_query, variables).await?;

        Ok(self.media_to_metadata(data.media))
    }

    async fn get_episode(
        &self,
        _series_id: &str,
        _season: i32,
        _episode: i32,
    ) -> Result<EpisodeInfo> {
        Err(ScraperError::NotFound(
            "AniList does not provide individual episode details".to_string(),
        ))
    }

    async fn find_by_external_id(
        &self,
        external_id: &str,
        source: &str,
    ) -> Result<Option<MediaInfo>> {
        if source != "mal" {
            return Ok(None);
        }

        let mal_id: i32 = external_id
            .parse()
            .map_err(|_| ScraperError::Parse(format!("Invalid MAL ID: {external_id}")))?;

        let gql_query = r#"
            query ($malId: Int) {
                Media(idMal: $malId, type: ANIME) {
                    id
                    title { romaji english native }
                    seasonYear
                    coverImage { large extraLarge }
                    description
                    averageScore
                    popularity
                    synonyms
                    idMal
                }
            }
        "#;

        let variables = serde_json::json!({ "malId": mal_id });

        let data: MediaData = self.query(gql_query, variables).await?;

        Ok(Some(self.media_to_info(&data.media)))
    }
}
