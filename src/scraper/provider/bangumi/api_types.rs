use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub list: Option<Vec<Subject>>,
    pub results: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct Subject {
    pub id: i32,
    #[serde(rename = "type")]
    pub subject_type: i32,
    pub name: String,
    pub name_cn: Option<String>,
    pub summary: Option<String>,
    pub date: Option<String>,
    #[serde(rename = "air_date")]
    pub air_date: Option<String>,
    pub images: Option<Images>,
    pub eps: Option<i32>,
    pub rating: Option<Rating>,
    pub tags: Option<Vec<Tag>>,
    pub infobox: Option<Vec<InfoBox>>,
}

#[derive(Debug, Deserialize)]
pub struct Images {
    pub large: Option<String>,
    pub common: Option<String>,
    pub medium: Option<String>,
    pub small: Option<String>,
    pub grid: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Rating {
    pub score: Option<f64>,
    pub total: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct Tag {
    pub name: String,
    pub count: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct InfoBox {
    pub key: String,
    pub value: InfoBoxValue,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum InfoBoxValue {
    String(String),
    Array(Vec<InfoBoxItem>),
}

#[derive(Debug, Deserialize)]
pub struct InfoBoxItem {
    pub v: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Episode {
    pub id: i32,
    #[serde(rename = "type")]
    pub episode_type: i32,
    pub name: Option<String>,
    pub name_cn: Option<String>,
    pub sort: f64,
    pub ep: Option<f64>,
    pub airdate: Option<String>,
    pub duration: Option<String>,
    pub desc: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EpisodesResponse {
    pub data: Vec<Episode>,
    pub total: i32,
}

// Subject types
pub const SUBJECT_TYPE_ANIME: i32 = 2;
pub const SUBJECT_TYPE_MOVIE: i32 = 6;
