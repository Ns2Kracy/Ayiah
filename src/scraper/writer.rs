use super::types::{EpisodeMetadata, MovieMetadata, TvMetadata};
use anyhow::Result;
use quick_xml::se::to_string;
use serde::Serialize;
use std::path::Path;
use tokio::io::AsyncWriteExt;

pub struct Writer;

impl Writer {
    pub async fn write_movie_nfo(path: &Path, metadata: &MovieMetadata) -> Result<()> {
        let nfo = MovieNfo::from(metadata);
        let xml = to_string(&nfo)?;
        // Kodi expects header? Usually not strictly required but good to have
        let content = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\" ?>\n{}",
            xml
        );

        let mut file = tokio::fs::File::create(path).await?;
        file.write_all(content.as_bytes()).await?;
        Ok(())
    }

    pub async fn write_tv_show_nfo(path: &Path, metadata: &TvMetadata) -> Result<()> {
        let nfo = TvShowNfo::from(metadata);
        let xml = to_string(&nfo)?;
        let content = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\" ?>\n{}",
            xml
        );

        let mut file = tokio::fs::File::create(path).await?;
        file.write_all(content.as_bytes()).await?;
        Ok(())
    }

    pub async fn write_episode_nfo(path: &Path, metadata: &EpisodeMetadata) -> Result<()> {
        let nfo = EpisodeNfo::from(metadata);
        let xml = to_string(&nfo)?;
        let content = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\" ?>\n{}",
            xml
        );

        let mut file = tokio::fs::File::create(path).await?;
        file.write_all(content.as_bytes()).await?;
        Ok(())
    }
}

// --- NFO DTOs ---

#[derive(Serialize)]
#[serde(rename = "movie")]
struct MovieNfo {
    title: String,
    originaltitle: Option<String>,
    plot: Option<String>,
    runtime: Option<i32>,
    year: Option<i32>,
    premiered: Option<String>,
    rating: Option<f64>,
    votes: Option<i32>,
    id: String,
    genre: Vec<String>,
    studio: Vec<String>,
    // Add more fields as needed
}

impl From<&MovieMetadata> for MovieNfo {
    fn from(m: &MovieMetadata) -> Self {
        Self {
            title: m.title.clone(),
            originaltitle: m.original_title.clone(),
            plot: m.overview.clone(),
            runtime: m.runtime,
            year: m
                .release_date
                .as_ref()
                .and_then(|d| d.split('-').next().and_then(|y| y.parse().ok())),
            premiered: m.release_date.clone(),
            rating: m.vote_average,
            votes: m.vote_count,
            id: m.id.clone(),
            genre: m.genres.clone(),
            studio: m.production_companies.clone(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename = "tvshow")]
struct TvShowNfo {
    title: String,
    originaltitle: Option<String>,
    plot: Option<String>,
    premiered: Option<String>,
    rating: Option<f64>,
    votes: Option<i32>,
    id: String,
    genre: Vec<String>,
    studio: Vec<String>,
}

impl From<&TvMetadata> for TvShowNfo {
    fn from(m: &TvMetadata) -> Self {
        Self {
            title: m.name.clone(),
            originaltitle: m.original_name.clone(),
            plot: m.overview.clone(),
            premiered: m.first_air_date.clone(),
            rating: m.vote_average,
            votes: m.vote_count,
            id: m.id.clone(),
            genre: m.genres.clone(),
            studio: m.production_companies.clone(),
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
    rating: Option<f64>,
    id: String,
}

impl From<&EpisodeMetadata> for EpisodeNfo {
    fn from(m: &EpisodeMetadata) -> Self {
        Self {
            title: m.name.clone(),
            season: m.season_number,
            episode: m.episode_number,
            plot: m.overview.clone(),
            aired: m.air_date.clone(),
            rating: m.vote_average,
            id: m.id.clone(),
        }
    }
}
