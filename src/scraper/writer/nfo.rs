use crate::scraper::types::{EpisodeInfo, MediaMetadata, MediaType};
use anyhow::Result;
use quick_xml::se::to_string;
use serde::Serialize;
use std::path::Path;
use tokio::io::AsyncWriteExt;

/// NFO file writer for Kodi/Jellyfin/Emby compatibility
pub struct Writer;

impl Writer {
    /// Write movie NFO file
    pub async fn write_movie_nfo(path: &Path, metadata: &MediaMetadata) -> Result<()> {
        let nfo = MovieNfo::from(metadata);
        Self::write_nfo(path, &nfo).await
    }

    /// Write TV show NFO file
    pub async fn write_tvshow_nfo(path: &Path, metadata: &MediaMetadata) -> Result<()> {
        let nfo = TvShowNfo::from(metadata);
        Self::write_nfo(path, &nfo).await
    }

    /// Write episode NFO file
    pub async fn write_episode_nfo(path: &Path, episode: &EpisodeInfo) -> Result<()> {
        let nfo = EpisodeNfo::from(episode);
        Self::write_nfo(path, &nfo).await
    }

    /// Auto-detect type and write appropriate NFO
    pub async fn write_nfo_auto(path: &Path, metadata: &MediaMetadata) -> Result<()> {
        match metadata.media_type {
            MediaType::Movie => Self::write_movie_nfo(path, metadata).await,
            MediaType::Tv | MediaType::Anime => Self::write_tvshow_nfo(path, metadata).await,
            MediaType::Unknown => Self::write_movie_nfo(path, metadata).await,
        }
    }

    async fn write_nfo<T: Serialize>(path: &Path, nfo: &T) -> Result<()> {
        let xml = to_string(nfo)?;
        let content = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n{xml}"
        );

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut file = tokio::fs::File::create(path).await?;
        file.write_all(content.as_bytes()).await?;

        Ok(())
    }
}

// NFO structures for Kodi/Jellyfin/Emby compatibility

#[derive(Serialize)]
#[serde(rename = "movie")]
struct MovieNfo {
    title: String,
    originaltitle: Option<String>,
    sorttitle: Option<String>,
    tagline: Option<String>,
    plot: Option<String>,
    runtime: Option<i32>,
    year: Option<i32>,
    premiered: Option<String>,
    rating: Option<f64>,
    votes: Option<i32>,
    #[serde(rename = "uniqueid")]
    uniqueids: Vec<UniqueId>,
    genre: Vec<String>,
    tag: Vec<String>,
    studio: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    actor: Vec<ActorNfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    director: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    credits: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thumb: Option<ThumbNfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fanart: Option<FanartNfo>,
}

impl From<&MediaMetadata> for MovieNfo {
    fn from(m: &MediaMetadata) -> Self {
        let year = m
            .release_date
            .as_ref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok());

        let mut uniqueids = Vec::new();
        if let Some(ref imdb) = m.external_ids.imdb {
            uniqueids.push(UniqueId {
                id_type: "imdb".to_string(),
                default: true,
                value: imdb.clone(),
            });
        }
        if let Some(ref tmdb) = m.external_ids.tmdb {
            uniqueids.push(UniqueId {
                id_type: "tmdb".to_string(),
                default: false,
                value: tmdb.clone(),
            });
        }

        let directors: Vec<String> = m
            .crew
            .iter()
            .filter(|c| c.role.as_deref() == Some("Director"))
            .map(|c| c.name.clone())
            .collect();

        let writers: Vec<String> = m
            .crew
            .iter()
            .filter(|c| matches!(c.role.as_deref(), Some("Writer" | "Screenplay")))
            .map(|c| c.name.clone())
            .collect();

        Self {
            title: m.title.clone(),
            originaltitle: m.original_title.clone(),
            sorttitle: m.sort_title.clone(),
            tagline: m.tagline.clone(),
            plot: m.overview.clone(),
            runtime: m.runtime,
            year,
            premiered: m.release_date.clone(),
            rating: m.rating,
            votes: m.vote_count,
            uniqueids,
            genre: m.genres.clone(),
            tag: m.tags.clone(),
            studio: m.studios.clone(),
            actor: m.cast.iter().map(ActorNfo::from).collect(),
            director: directors,
            credits: writers,
            thumb: m.images.poster.as_ref().map(|url| ThumbNfo {
                aspect: "poster".to_string(),
                value: url.clone(),
            }),
            fanart: m.images.backdrop.as_ref().map(|url| FanartNfo {
                thumb: vec![ThumbNfo {
                    aspect: "fanart".to_string(),
                    value: url.clone(),
                }],
            }),
        }
    }
}

#[derive(Serialize)]
#[serde(rename = "tvshow")]
struct TvShowNfo {
    title: String,
    originaltitle: Option<String>,
    sorttitle: Option<String>,
    plot: Option<String>,
    premiered: Option<String>,
    #[serde(rename = "enddate")]
    enddate: Option<String>,
    rating: Option<f64>,
    votes: Option<i32>,
    status: Option<String>,
    #[serde(rename = "uniqueid")]
    uniqueids: Vec<UniqueId>,
    genre: Vec<String>,
    tag: Vec<String>,
    studio: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    actor: Vec<ActorNfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thumb: Option<ThumbNfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fanart: Option<FanartNfo>,
}

impl From<&MediaMetadata> for TvShowNfo {
    fn from(m: &MediaMetadata) -> Self {
        let mut uniqueids = Vec::new();
        if let Some(ref imdb) = m.external_ids.imdb {
            uniqueids.push(UniqueId {
                id_type: "imdb".to_string(),
                default: true,
                value: imdb.clone(),
            });
        }
        if let Some(ref tmdb) = m.external_ids.tmdb {
            uniqueids.push(UniqueId {
                id_type: "tmdb".to_string(),
                default: false,
                value: tmdb.clone(),
            });
        }
        if let Some(ref tvdb) = m.external_ids.tvdb {
            uniqueids.push(UniqueId {
                id_type: "tvdb".to_string(),
                default: false,
                value: tvdb.clone(),
            });
        }
        if let Some(ref anilist) = m.external_ids.anilist {
            uniqueids.push(UniqueId {
                id_type: "anilist".to_string(),
                default: false,
                value: anilist.clone(),
            });
        }

        Self {
            title: m.title.clone(),
            originaltitle: m.original_title.clone(),
            sorttitle: m.sort_title.clone(),
            plot: m.overview.clone(),
            premiered: m.release_date.clone(),
            enddate: m.end_date.clone(),
            rating: m.rating,
            votes: m.vote_count,
            status: m.status.clone(),
            uniqueids,
            genre: m.genres.clone(),
            tag: m.tags.clone(),
            studio: m.studios.clone(),
            actor: m.cast.iter().map(ActorNfo::from).collect(),
            thumb: m.images.poster.as_ref().map(|url| ThumbNfo {
                aspect: "poster".to_string(),
                value: url.clone(),
            }),
            fanart: m.images.backdrop.as_ref().map(|url| FanartNfo {
                thumb: vec![ThumbNfo {
                    aspect: "fanart".to_string(),
                    value: url.clone(),
                }],
            }),
        }
    }
}

#[derive(Serialize)]
#[serde(rename = "episodedetails")]
struct EpisodeNfo {
    title: String,
    season: i32,
    episode: i32,
    plot: Option<String>,
    aired: Option<String>,
    runtime: Option<i32>,
    rating: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thumb: Option<String>,
}

impl From<&EpisodeInfo> for EpisodeNfo {
    fn from(e: &EpisodeInfo) -> Self {
        Self {
            title: e.title.clone(),
            season: e.season,
            episode: e.episode,
            plot: e.overview.clone(),
            aired: e.air_date.clone(),
            runtime: e.runtime,
            rating: e.rating,
            thumb: e.still_url.clone(),
        }
    }
}

#[derive(Serialize)]
struct UniqueId {
    #[serde(rename = "@type")]
    id_type: String,
    #[serde(rename = "@default")]
    default: bool,
    #[serde(rename = "$value")]
    value: String,
}

#[derive(Serialize)]
struct ActorNfo {
    name: String,
    role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thumb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    order: Option<i32>,
}

impl From<&crate::scraper::types::PersonInfo> for ActorNfo {
    fn from(p: &crate::scraper::types::PersonInfo) -> Self {
        Self {
            name: p.name.clone(),
            role: p.role.clone(),
            thumb: p.image_url.clone(),
            order: p.order,
        }
    }
}

#[derive(Serialize)]
struct ThumbNfo {
    #[serde(rename = "@aspect")]
    aspect: String,
    #[serde(rename = "$value")]
    value: String,
}

#[derive(Serialize)]
struct FanartNfo {
    thumb: Vec<ThumbNfo>,
}
