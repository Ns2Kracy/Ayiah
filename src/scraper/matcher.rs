use crate::scraper::{
    parser::{MediaHint, ParsedMedia},
    types::{MediaInfo, MediaType},
};

/// Match confidence level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    /// No match
    None = 0,
    /// Low confidence - might be wrong
    Low = 1,
    /// Medium confidence - likely correct
    Medium = 2,
    /// High confidence - almost certainly correct
    High = 3,
    /// Exact match
    Exact = 4,
}

/// A scored match result
#[derive(Debug, Clone)]
pub struct ScoredMatch {
    /// The matched media info
    pub info: MediaInfo,
    /// Match score (0-100)
    pub score: i32,
    /// Confidence level
    pub confidence: Confidence,
    /// Breakdown of score components
    pub breakdown: ScoreBreakdown,
}

/// Breakdown of how the score was calculated
#[derive(Debug, Clone, Default)]
pub struct ScoreBreakdown {
    pub title_score: i32,
    pub year_score: i32,
    pub type_score: i32,
    pub provider_score: i32,
    pub popularity_score: i32,
}

/// Matcher for scoring and ranking search results
pub struct Matcher;

impl Matcher {
    /// Score and rank search results against parsed media info
    #[must_use] 
    pub fn rank(results: Vec<MediaInfo>, parsed: &ParsedMedia) -> Vec<ScoredMatch> {
        let mut scored: Vec<ScoredMatch> = results
            .into_iter()
            .map(|info| Self::score_match(&info, parsed))
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.score.cmp(&a.score));

        scored
    }

    /// Get the best match if confidence is high enough
    #[must_use] 
    pub fn best_match(results: Vec<MediaInfo>, parsed: &ParsedMedia) -> Option<ScoredMatch> {
        let ranked = Self::rank(results, parsed);
        ranked
            .into_iter()
            .next()
            .filter(|m| m.confidence >= Confidence::Medium)
    }

    /// Score a single match
    fn score_match(info: &MediaInfo, parsed: &ParsedMedia) -> ScoredMatch {
        let breakdown = ScoreBreakdown {
            // Title matching (0-40 points)
            title_score: Self::score_title(&info.all_titles(), &parsed.title),
            // Year matching (0-20 points)
            year_score: Self::score_year(info.year, parsed.year),
            // Type matching (0-20 points)
            type_score: Self::score_type(info.media_type, parsed.hint),
            // Provider priority (0-10 points)
            provider_score: Self::score_provider(&info.provider, info.media_type),
            // Popularity bonus (0-10 points)
            popularity_score: Self::score_popularity(info.popularity),
        };

        let total_score = breakdown.title_score
            + breakdown.year_score
            + breakdown.type_score
            + breakdown.provider_score
            + breakdown.popularity_score;

        let confidence = Self::calculate_confidence(total_score, &breakdown);

        ScoredMatch {
            info: info.clone(),
            score: total_score,
            confidence,
            breakdown,
        }
    }

    fn score_title(titles: &[&str], query: &str) -> i32 {
        let query_normalized = Self::normalize_title(query);

        let mut best_score = 0;

        for title in titles {
            let title_normalized = Self::normalize_title(title);

            // Exact match
            if title_normalized == query_normalized {
                return 40;
            }

            // Calculate similarity
            let similarity = Self::string_similarity(&title_normalized, &query_normalized);
            let score = (similarity * 40.0) as i32;

            best_score = best_score.max(score);
        }

        best_score
    }

    fn normalize_title(title: &str) -> String {
        title
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn string_similarity(a: &str, b: &str) -> f64 {
        if a == b {
            return 1.0;
        }
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        // Use Jaccard similarity on words
        let words_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
        let words_b: std::collections::HashSet<&str> = b.split_whitespace().collect();

        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();

        if union == 0 {
            return 0.0;
        }

        let jaccard = intersection as f64 / union as f64;

        // Also check if one contains the other
        let contains_bonus = if a.contains(b) || b.contains(a) {
            0.2
        } else {
            0.0
        };

        (jaccard + contains_bonus).min(1.0)
    }

    const fn score_year(info_year: Option<i32>, parsed_year: Option<i32>) -> i32 {
        match (info_year, parsed_year) {
            (Some(a), Some(b)) => {
                let diff = (a - b).abs();
                match diff {
                    0 => 20,
                    1 => 15, // Off by one year is common
                    2 => 10,
                    _ => 0,
                }
            }
            (None, None) => 10, // Both unknown, neutral
            _ => 5,             // One unknown, slight penalty
        }
    }

    const fn score_type(info_type: MediaType, hint: MediaHint) -> i32 {
        match (info_type, hint) {
            // Exact matches
            (MediaType::Movie, MediaHint::Movie) => 20,
            (MediaType::Tv, MediaHint::TvShow) => 20,
            (MediaType::Anime, MediaHint::Anime) => 20,

            // Compatible matches
            (MediaType::Tv, MediaHint::Anime) | (MediaType::Anime, MediaHint::TvShow) => 15,

            // Unknown hint, no penalty
            (_, MediaHint::Unknown) => 10,

            // Mismatches
            _ => 0,
        }
    }

    fn score_provider(provider: &str, media_type: MediaType) -> i32 {
        match (provider, media_type) {
            ("anilist", MediaType::Anime) => 10,
            ("bangumi", MediaType::Anime) => 8,
            ("tmdb", MediaType::Movie) => 10,
            ("tmdb", MediaType::Tv) => 9,
            ("tmdb", MediaType::Anime) => 5,
            _ => 5,
        }
    }

    fn score_popularity(popularity: Option<f64>) -> i32 {
        match popularity {
            Some(p) if p > 1000.0 => 10,
            Some(p) if p > 100.0 => 7,
            Some(p) if p > 10.0 => 5,
            Some(_) => 3,
            None => 5,
        }
    }

    const fn calculate_confidence(total_score: i32, breakdown: &ScoreBreakdown) -> Confidence {
        // Must have decent title match
        if breakdown.title_score < 20 {
            return Confidence::None;
        }

        match total_score {
            90..=100 => Confidence::Exact,
            75..=89 => Confidence::High,
            55..=74 => Confidence::Medium,
            35..=54 => Confidence::Low,
            _ => Confidence::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_title() {
        assert_eq!(
            Matcher::normalize_title("The Matrix (1999)"),
            "the matrix 1999"
        );
        assert_eq!(
            Matcher::normalize_title("Breaking Bad S01E01"),
            "breaking bad s01e01"
        );
    }

    #[test]
    fn test_string_similarity() {
        assert!((Matcher::string_similarity("the matrix", "the matrix") - 1.0).abs() < 0.01);
        assert!(Matcher::string_similarity("the matrix", "matrix") > 0.5);
        assert!(Matcher::string_similarity("the matrix", "inception") < 0.3);
    }

    #[test]
    fn test_score_year() {
        assert_eq!(Matcher::score_year(Some(1999), Some(1999)), 20);
        assert_eq!(Matcher::score_year(Some(1999), Some(2000)), 15);
        assert_eq!(Matcher::score_year(Some(1999), Some(2005)), 0);
    }

    fn create_test_info(title: &str, year: Option<i32>, media_type: MediaType) -> MediaInfo {
        MediaInfo::new("123", title, "test")
            .with_type(media_type)
            .with_year(year)
    }

    fn create_parsed(title: &str, year: Option<i32>, hint: MediaHint) -> ParsedMedia {
        ParsedMedia {
            title: title.to_string(),
            original_title: title.to_string(),
            year,
            hint,
            ..Default::default()
        }
    }

    #[test]
    fn test_exact_match_high_confidence() {
        let results = vec![create_test_info("The Matrix", Some(1999), MediaType::Movie)];
        let parsed = create_parsed("The Matrix", Some(1999), MediaHint::Movie);

        let ranked = Matcher::rank(results, &parsed);

        assert!(!ranked.is_empty());
        assert!(ranked[0].confidence >= Confidence::High);
    }

    #[test]
    fn test_year_mismatch_lowers_score() {
        let results = vec![
            create_test_info("The Matrix", Some(1999), MediaType::Movie),
            create_test_info("The Matrix", Some(2021), MediaType::Movie),
        ];
        let parsed = create_parsed("The Matrix", Some(1999), MediaHint::Movie);

        let ranked = Matcher::rank(results, &parsed);

        assert_eq!(ranked.len(), 2);
        // 1999 version should rank higher
        assert_eq!(ranked[0].info.year, Some(1999));
    }

    #[test]
    fn test_type_mismatch_lowers_score() {
        let results = vec![
            create_test_info("Breaking Bad", None, MediaType::Tv),
            create_test_info("Breaking Bad", None, MediaType::Movie),
        ];
        let parsed = create_parsed("Breaking Bad", None, MediaHint::TvShow);

        let ranked = Matcher::rank(results, &parsed);

        // TV version should rank higher
        assert_eq!(ranked[0].info.media_type, MediaType::Tv);
    }

    #[test]
    fn test_partial_title_match() {
        let results = vec![create_test_info(
            "Sousou no Frieren",
            Some(2023),
            MediaType::Anime,
        )];
        let parsed = create_parsed("Frieren", Some(2023), MediaHint::Anime);

        let ranked = Matcher::rank(results, &parsed);

        assert!(!ranked.is_empty());
        assert!(ranked[0].confidence >= Confidence::Medium);
    }

    #[test]
    fn test_best_match_filters_low_confidence() {
        let results = vec![create_test_info(
            "Completely Different Title",
            Some(2000),
            MediaType::Movie,
        )];
        let parsed = create_parsed("The Matrix", Some(1999), MediaHint::Movie);

        let best = Matcher::best_match(results, &parsed);

        // Should return None due to low confidence
        assert!(best.is_none());
    }

    #[test]
    fn test_anime_tv_compatibility() {
        let results = vec![create_test_info(
            "Attack on Titan",
            Some(2013),
            MediaType::Tv,
        )];
        let parsed = create_parsed("Attack on Titan", Some(2013), MediaHint::Anime);

        let ranked = Matcher::rank(results, &parsed);

        // Anime and TV should be compatible
        assert!(!ranked.is_empty());
        assert!(ranked[0].confidence >= Confidence::Medium);
    }
}
