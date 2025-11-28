use serde::Deserialize;

// Search responses
#[derive(Debug, Deserialize)]
pub struct SearchResponse<T> {
    pub results: Vec<T>,
    pub page: i32,
    pub total_pages: i32,
    pub total_results: i32,
}

#[derive(Debug, Deserialize)]
pub struct MovieResult {
    pub id: i64,
    pub title: String,
    pub original_title: String,
    pub release_date: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub overview: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<i32>,
    pub popularity: Option<f64>,
    pub original_language: Option<String>,
    pub genre_ids: Option<Vec<i32>>,
}

#[derive(Debug, Deserialize)]
pub struct TvResult {
    pub id: i64,
    pub name: String,
    pub original_name: String,
    pub first_air_date: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub overview: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<i32>,
    pub popularity: Option<f64>,
    pub original_language: Option<String>,
    pub genre_ids: Option<Vec<i32>>,
}

// Detail responses
#[derive(Debug, Deserialize)]
pub struct MovieDetails {
    pub id: i64,
    pub title: String,
    pub original_title: String,
    pub tagline: Option<String>,
    pub overview: Option<String>,
    pub release_date: Option<String>,
    pub runtime: Option<i32>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<i32>,
    pub popularity: Option<f64>,
    pub status: Option<String>,
    pub original_language: String,
    pub genres: Vec<Genre>,
    pub production_companies: Vec<Company>,
    pub production_countries: Vec<Country>,
    pub external_ids: Option<ExternalIds>,
    pub credits: Option<Credits>,
}

#[derive(Debug, Deserialize)]
pub struct TvDetails {
    pub id: i64,
    pub name: String,
    pub original_name: String,
    pub tagline: Option<String>,
    pub overview: Option<String>,
    pub first_air_date: Option<String>,
    pub last_air_date: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<i32>,
    pub popularity: Option<f64>,
    pub status: Option<String>,
    pub original_language: String,
    pub genres: Vec<Genre>,
    pub production_companies: Vec<Company>,
    pub number_of_seasons: i32,
    pub number_of_episodes: i32,
    pub episode_run_time: Vec<i32>,
    pub seasons: Vec<Season>,
    pub external_ids: Option<ExternalIds>,
    pub credits: Option<Credits>,
}

#[derive(Debug, Deserialize)]
pub struct EpisodeDetails {
    pub id: i64,
    pub name: String,
    pub season_number: i32,
    pub episode_number: i32,
    pub air_date: Option<String>,
    pub overview: Option<String>,
    pub still_path: Option<String>,
    pub runtime: Option<i32>,
    pub vote_average: Option<f64>,
}

// Common types
#[derive(Debug, Deserialize)]
pub struct Genre {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Company {
    pub id: i64,
    pub name: String,
    pub logo_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Country {
    pub iso_3166_1: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Season {
    pub id: i64,
    pub season_number: i32,
    pub name: Option<String>,
    pub overview: Option<String>,
    pub air_date: Option<String>,
    pub episode_count: Option<i32>,
    pub poster_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExternalIds {
    pub imdb_id: Option<String>,
    pub tvdb_id: Option<i64>,
    pub facebook_id: Option<String>,
    pub instagram_id: Option<String>,
    pub twitter_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Credits {
    pub cast: Vec<CastMember>,
    pub crew: Vec<CrewMember>,
}

#[derive(Debug, Deserialize)]
pub struct CastMember {
    pub id: i64,
    pub name: String,
    pub character: Option<String>,
    pub profile_path: Option<String>,
    pub order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CrewMember {
    pub id: i64,
    pub name: String,
    pub job: Option<String>,
    pub department: Option<String>,
    pub profile_path: Option<String>,
}

// Find by external ID
#[derive(Debug, Deserialize)]
pub struct FindResponse {
    pub movie_results: Vec<MovieResult>,
    pub tv_results: Vec<TvResult>,
}
