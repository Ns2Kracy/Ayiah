use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GraphQLResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLError {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchData {
    #[serde(rename = "Page")]
    pub page: Page,
}

#[derive(Debug, Deserialize)]
pub struct Page {
    pub media: Vec<Media>,
}

#[derive(Debug, Deserialize)]
pub struct MediaData {
    #[serde(rename = "Media")]
    pub media: Media,
}

#[derive(Debug, Deserialize)]
pub struct Media {
    pub id: i32,
    pub title: Title,
    pub format: Option<String>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub season: Option<String>,
    #[serde(rename = "seasonYear")]
    pub season_year: Option<i32>,
    pub episodes: Option<i32>,
    pub duration: Option<i32>,
    #[serde(rename = "coverImage")]
    pub cover_image: Option<CoverImage>,
    #[serde(rename = "bannerImage")]
    pub banner_image: Option<String>,
    #[serde(rename = "averageScore")]
    pub average_score: Option<i32>,
    pub popularity: Option<i32>,
    pub genres: Option<Vec<String>>,
    pub tags: Option<Vec<Tag>>,
    pub studios: Option<Studios>,
    #[serde(rename = "startDate")]
    pub start_date: Option<FuzzyDate>,
    #[serde(rename = "endDate")]
    pub end_date: Option<FuzzyDate>,
    #[serde(rename = "idMal")]
    pub id_mal: Option<i32>,
    pub synonyms: Option<Vec<String>>,
    pub characters: Option<Characters>,
    pub staff: Option<Staff>,
}

#[derive(Debug, Deserialize)]
pub struct Title {
    pub romaji: Option<String>,
    pub english: Option<String>,
    pub native: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CoverImage {
    pub large: Option<String>,
    #[serde(rename = "extraLarge")]
    pub extra_large: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Tag {
    pub name: String,
    pub rank: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct Studios {
    pub nodes: Vec<Studio>,
}

#[derive(Debug, Deserialize)]
pub struct Studio {
    pub name: String,
    #[serde(rename = "isAnimationStudio")]
    pub is_animation_studio: bool,
}

#[derive(Debug, Deserialize)]
pub struct FuzzyDate {
    pub year: Option<i32>,
    pub month: Option<i32>,
    pub day: Option<i32>,
}

impl FuzzyDate {
    pub fn to_string(&self) -> Option<String> {
        match (self.year, self.month, self.day) {
            (Some(y), Some(m), Some(d)) => Some(format!("{y:04}-{m:02}-{d:02}")),
            (Some(y), Some(m), None) => Some(format!("{y:04}-{m:02}")),
            (Some(y), None, None) => Some(format!("{y:04}")),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Characters {
    pub edges: Vec<CharacterEdge>,
}

#[derive(Debug, Deserialize)]
pub struct CharacterEdge {
    pub node: Character,
    pub role: Option<String>,
    #[serde(rename = "voiceActors")]
    pub voice_actors: Option<Vec<VoiceActor>>,
}

#[derive(Debug, Deserialize)]
pub struct Character {
    pub id: i32,
    pub name: CharacterName,
    pub image: Option<CharacterImage>,
}

#[derive(Debug, Deserialize)]
pub struct CharacterName {
    pub full: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CharacterImage {
    pub large: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VoiceActor {
    pub id: i32,
    pub name: CharacterName,
    pub image: Option<CharacterImage>,
    pub language: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Staff {
    pub edges: Vec<StaffEdge>,
}

#[derive(Debug, Deserialize)]
pub struct StaffEdge {
    pub node: StaffNode,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StaffNode {
    pub id: i32,
    pub name: CharacterName,
    pub image: Option<CharacterImage>,
}
