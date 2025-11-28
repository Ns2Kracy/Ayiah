use super::api_types::*;
use crate::scraper::{
    Result, ScraperError,
    provider::{HttpClient, MetadataProvider, SearchOptions},
    types::{EpisodeInfo, ExternalIds, ImageSet, MediaInfo, MediaMetadata, MediaType},
};
use async_trait::async_trait;

const BANGUMI_API_URL: &str = "https://api.bgm.tv";

pub struct BangumiProvider {
    client: HttpClient,
}

impl Default for BangumiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl BangumiProvider {
    pub fn new() -> Self {
        Self {
            client: HttpClient::new(BANGUMI_API_URL),
        }
    }

    fn subject_to_info(&self, subject: &Subject) -> MediaInfo {
        let title = subject
            .name_cn
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| subject.name.clone());

        let year = subject
            .date
            .as_ref()
            .or(subject.air_date.as_ref())
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok());

        let poster = subject
            .images
            .as_ref()
            .and_then(|i| i.large.clone().or_else(|| i.common.clone()));

        let rating = subject.rating.as_ref().and_then(|r| r.score);

        MediaInfo::new(subject.id.to_string(), title, "bangumi")
            .with_type(MediaType::Anime)
            .with_year(year)
            .with_original_title(Some(subject.name.clone()))
            .with_poster(poster)
            .with_overview(subject.summary.clone())
            .with_rating(rating)
    }

    fn subject_to_metadata(&self, subject: Subject) -> MediaMetadata {
        let title = subject
            .name_cn
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| subject.name.clone());

        let release_date = subject.date.clone().or_else(|| subject.air_date.clone());

        let year = release_date
            .as_ref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse::<i32>().ok());

        // Extract info from infobox
        let mut studios = Vec::new();
        let mut director = None;

        if let Some(ref infobox) = subject.infobox {
            for item in infobox {
                match item.key.as_str() {
                    "动画制作" | "製作" => {
                        if let InfoBoxValue::String(ref s) = item.value {
                            studios.push(s.clone());
                        } else if let InfoBoxValue::Array(ref arr) = item.value {
                            for i in arr {
                                if let Some(ref v) = i.v {
                                    studios.push(v.clone());
                                }
                            }
                        }
                    }
                    "导演" | "監督" => {
                        if let InfoBoxValue::String(ref s) = item.value {
                            director = Some(s.clone());
                        }
                    }
                    _ => {}
                }
            }
        }

        let format = match subject.subject_type {
            SUBJECT_TYPE_ANIME => "TV",
            SUBJECT_TYPE_MOVIE => "Movie",
            _ => "Unknown",
        };

        MediaMetadata {
            id: subject.id.to_string(),
            title: title.clone(),
            original_title: Some(subject.name),
            sort_title: Some(title),
            media_type: MediaType::Anime,
            tagline: None,
            overview: subject.summary,
            release_date,
            end_date: None,
            runtime: None,
            rating: subject.rating.as_ref().and_then(|r| r.score),
            vote_count: subject.rating.and_then(|r| r.total),
            genres: subject
                .tags
                .unwrap_or_default()
                .into_iter()
                .take(10)
                .map(|t| t.name)
                .collect(),
            tags: Vec::new(),
            studios,
            language: Some("ja".to_string()),
            content_rating: None,
            status: Some(format.to_string()),
            images: ImageSet {
                poster: subject
                    .images
                    .as_ref()
                    .and_then(|i| i.large.clone().or_else(|| i.common.clone())),
                ..Default::default()
            },
            external_ids: ExternalIds {
                bangumi: Some(subject.id.to_string()),
                ..Default::default()
            },
            provider: "bangumi".to_string(),
            season_count: None,
            episode_count: subject.eps,
            seasons: Vec::new(),
            cast: Vec::new(),
            crew: if let Some(dir) = director {
                vec![crate::scraper::types::PersonInfo {
                    id: String::new(),
                    name: dir,
                    role: Some("Director".to_string()),
                    image_url: None,
                    order: Some(0),
                }]
            } else {
                Vec::new()
            },
        }
    }

    fn parse_duration(&self, duration: Option<&str>) -> Option<i32> {
        duration.and_then(|d| {
            // Parse formats like "24:00" or "24分"
            if d.contains(':') {
                let parts: Vec<&str> = d.split(':').collect();
                if parts.len() >= 2 {
                    let minutes: i32 = parts[0].parse().ok()?;
                    return Some(minutes);
                }
            }
            // Try to extract number
            d.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .ok()
        })
    }
}

#[async_trait]
impl MetadataProvider for BangumiProvider {
    fn id(&self) -> &'static str {
        "bangumi"
    }

    fn name(&self) -> &'static str {
        "Bangumi"
    }

    fn supported_types(&self) -> &[MediaType] {
        &[MediaType::Anime]
    }

    fn requires_api_key(&self) -> bool {
        false
    }

    fn priority_for(&self, media_type: MediaType) -> i32 {
        match media_type {
            MediaType::Anime => 80, // Good for anime, especially Chinese metadata
            _ => 0,
        }
    }

    async fn search(&self, query: &str, options: &SearchOptions) -> Result<Vec<MediaInfo>> {
        let encoded_query = urlencoding::encode(query);
        let limit = options.limit.unwrap_or(20);
        let endpoint = format!(
            "/search/subject/{}?type=2&responseGroup=small&max_results={}",
            encoded_query, limit
        );

        let response: SearchResponse = self.client.get(&endpoint).await?;

        let subjects = response.list.unwrap_or_default();

        if subjects.is_empty() {
            return Err(ScraperError::NotFound(format!(
                "No results found for: {query}"
            )));
        }

        // Filter by year if specified
        let results: Vec<MediaInfo> = subjects
            .iter()
            .filter(|s| {
                if let Some(year) = options.year {
                    let subject_year = s
                        .date
                        .as_ref()
                        .or(s.air_date.as_ref())
                        .and_then(|d| d.split('-').next())
                        .and_then(|y| y.parse::<i32>().ok());
                    subject_year == Some(year)
                } else {
                    true
                }
            })
            .map(|s| self.subject_to_info(s))
            .collect();

        if results.is_empty() {
            return Err(ScraperError::NotFound(format!(
                "No results found for: {query}"
            )));
        }

        Ok(results)
    }

    async fn get_metadata(&self, id: &str, _media_type: MediaType) -> Result<MediaMetadata> {
        let endpoint = format!("/v0/subjects/{id}");
        let subject: Subject = self.client.get(&endpoint).await?;

        Ok(self.subject_to_metadata(subject))
    }

    async fn get_episode(
        &self,
        series_id: &str,
        _season: i32,
        episode: i32,
    ) -> Result<EpisodeInfo> {
        let endpoint = format!("/v0/episodes?subject_id={series_id}&type=0&limit=100");
        let response: EpisodesResponse = self.client.get(&endpoint).await?;

        let ep = response
            .data
            .into_iter()
            .find(|e| e.ep.map(|n| n as i32) == Some(episode) || e.sort as i32 == episode)
            .ok_or_else(|| ScraperError::NotFound(format!("Episode {episode} not found")))?;

        let title = ep
            .name_cn
            .clone()
            .filter(|s| !s.is_empty())
            .or(ep.name.clone())
            .unwrap_or_else(|| format!("Episode {episode}"));

        Ok(EpisodeInfo {
            id: ep.id.to_string(),
            title,
            season: 1,
            episode: ep.ep.map(|n| n as i32).unwrap_or(ep.sort as i32),
            absolute_number: Some(ep.sort as i32),
            air_date: ep.airdate,
            overview: ep.desc,
            runtime: self.parse_duration(ep.duration.as_deref()),
            rating: None,
            still_url: None,
            provider: "bangumi".to_string(),
        })
    }
}
