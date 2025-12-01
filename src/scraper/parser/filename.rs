use super::patterns::{MediaHint, PATTERNS};
use std::path::Path;

/// Parsed information from a media filename
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMedia {
    /// Cleaned title for searching
    pub title: String,
    /// Original title (before cleaning)
    pub original_title: String,
    /// Release year if found
    pub year: Option<i32>,
    /// Season number (1-indexed)
    pub season: Option<i32>,
    /// Episode number (1-indexed)
    pub episode: Option<i32>,
    /// Video resolution (e.g., "1080p")
    pub resolution: Option<String>,
    /// Source quality (e.g., "`BluRay`", "WEB-DL")
    pub quality: Option<String>,
    /// Video codec (e.g., "x265", "HEVC")
    pub codec: Option<String>,
    /// Release group name
    pub release_group: Option<String>,
    /// Hint about media type based on filename patterns
    pub hint: MediaHint,
}

impl Default for ParsedMedia {
    fn default() -> Self {
        Self {
            title: String::new(),
            original_title: String::new(),
            year: None,
            season: None,
            episode: None,
            resolution: None,
            quality: None,
            codec: None,
            release_group: None,
            hint: MediaHint::Unknown,
        }
    }
}

pub struct Parser;

impl Parser {
    /// Parse a file path to extract media information
    #[must_use] 
    pub fn parse(path: &Path) -> ParsedMedia {
        let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        Self::parse_filename(filename)
    }

    /// Parse a filename string directly
    #[must_use] 
    pub fn parse_filename(filename: &str) -> ParsedMedia {
        let mut result = ParsedMedia {
            original_title: filename.to_string(),
            ..Default::default()
        };

        let patterns = &*PATTERNS;

        // Extract release group from start [GroupName]
        if let Some(caps) = patterns.release_group_start.captures(filename) {
            let group = caps.get(1).map(|m| m.as_str().to_string());
            // Only set if it's not a hash or resolution
            if let Some(ref g) = group
                && !patterns.hash.is_match(&format!("[{g}]"))
                && !patterns.resolution.is_match(g)
            {
                result.release_group = Some(g.clone());
            }
        }

        // Extract resolution
        if let Some(m) = patterns.resolution.find(filename) {
            result.resolution = Some(m.as_str().to_uppercase());
        }

        // Extract quality
        if let Some(m) = patterns.quality.find(filename) {
            result.quality = Some(m.as_str().to_string());
        }

        // Extract codec
        if let Some(m) = patterns.codec.find(filename) {
            result.codec = Some(m.as_str().to_uppercase());
        }

        // Try different episode patterns in order of specificity
        let (season, episode, title_end_pos) = Self::extract_episode_info(filename, patterns);
        result.season = season;
        result.episode = episode;

        // Extract year
        result.year = Self::extract_year(filename, patterns);

        // Determine media hint
        result.hint = Self::determine_hint(&result, filename, patterns);

        // Extract and clean title
        result.title = Self::extract_title(filename, title_end_pos, &result, patterns);

        result
    }

    fn extract_episode_info(
        filename: &str,
        patterns: &super::patterns::Patterns,
    ) -> (Option<i32>, Option<i32>, Option<usize>) {
        // Try S01E01 format first (most specific)
        if let Some(caps) = patterns.season_episode.captures(filename) {
            let season = caps.get(1).and_then(|m| m.as_str().parse().ok());
            let episode = caps.get(2).and_then(|m| m.as_str().parse().ok());
            let pos = caps.get(0).map(|m| m.start());
            return (season, episode, pos);
        }

        // Try 1x01 format
        if let Some(caps) = patterns.season_x_episode.captures(filename) {
            let season = caps.get(1).and_then(|m| m.as_str().parse().ok());
            let episode = caps.get(2).and_then(|m| m.as_str().parse().ok());
            let pos = caps.get(0).map(|m| m.start());
            return (season, episode, pos);
        }

        // Try anime format: Title - 01
        if let Some(caps) = patterns.episode_dash.captures(filename) {
            let episode = caps.get(1).and_then(|m| m.as_str().parse().ok());
            let pos = caps.get(0).map(|m| m.start());
            return (Some(1), episode, pos); // Assume season 1 for anime
        }

        // Try E01 format
        if let Some(caps) = patterns.episode_only.captures(filename) {
            let episode = caps.get(1).and_then(|m| m.as_str().parse().ok());
            let pos = caps.get(0).map(|m| m.start());
            return (Some(1), episode, pos);
        }

        // Try [01] format
        if let Some(caps) = patterns.episode_bracket.captures(filename) {
            let episode = caps.get(1).and_then(|m| m.as_str().parse().ok());
            let pos = caps.get(0).map(|m| m.start());
            return (Some(1), episode, pos);
        }

        (None, None, None)
    }

    fn extract_year(filename: &str, patterns: &super::patterns::Patterns) -> Option<i32> {
        // Prefer year in parentheses
        if let Some(caps) = patterns.year_in_parens.captures(filename)
            && let Some(year) = caps.get(1).and_then(|m| m.as_str().parse().ok())
            && (1900..=2099).contains(&year)
        {
            return Some(year);
        }

        // Fall back to any 4-digit year
        if let Some(m) = patterns.year.find(filename)
            && let Ok(year) = m.as_str().parse::<i32>()
            && (1900..=2099).contains(&year)
        {
            return Some(year);
        }

        None
    }

    fn determine_hint(
        result: &ParsedMedia,
        filename: &str,
        patterns: &super::patterns::Patterns,
    ) -> MediaHint {
        // Check for anime indicators
        let has_anime_group =
            result.release_group.is_some() && patterns.release_group_start.is_match(filename);
        let has_dash_episode = patterns.episode_dash.is_match(filename);
        let has_japanese = filename.chars().any(|c| {
            ('\u{3040}'..='\u{309F}').contains(&c)  // Hiragana
                || ('\u{30A0}'..='\u{30FF}').contains(&c)  // Katakana
                || ('\u{4E00}'..='\u{9FFF}').contains(&c) // CJK
        });

        if has_anime_group && has_dash_episode {
            return MediaHint::Anime;
        }
        if has_japanese {
            return MediaHint::Anime;
        }

        // Check for TV show indicators
        if result.season.is_some() && result.episode.is_some() {
            if result.season == Some(1) && has_dash_episode {
                return MediaHint::Anime;
            }
            return MediaHint::TvShow;
        }

        // Check for movie indicators
        if result.year.is_some() && result.episode.is_none() {
            return MediaHint::Movie;
        }

        MediaHint::Unknown
    }

    fn extract_title(
        filename: &str,
        title_end_pos: Option<usize>,
        result: &ParsedMedia,
        patterns: &super::patterns::Patterns,
    ) -> String {
        let mut title = filename.to_string();

        // Remove release group from start
        if patterns.release_group_start.is_match(&title) {
            title = patterns.release_group_start.replace(&title, "").to_string();
        }

        // Truncate at episode info position if available
        if let Some(pos) = title_end_pos {
            // Adjust position after removing release group
            let adjusted_pos = if result.release_group.is_some() {
                pos.saturating_sub(
                    patterns
                        .release_group_start
                        .find(filename)
                        .map_or(0, |m| m.end()),
                )
            } else {
                pos
            };
            if adjusted_pos < title.len() {
                title.truncate(adjusted_pos);
            }
        } else if let Some(year) = result.year {
            // Truncate at year for movies
            // First try to find year in parentheses like "(2010)"
            let year_in_parens = format!("({year})");
            if let Some(pos) = title.find(&year_in_parens) {
                title.truncate(pos);
            } else {
                // Fall back to just the year
                let year_str = year.to_string();
                if let Some(pos) = title.find(&year_str) {
                    title.truncate(pos);
                }
            }
        }

        // Remove brackets and their contents
        title = patterns.brackets.replace_all(&title, " ").to_string();

        // Remove resolution, quality, codec
        title = patterns.resolution.replace_all(&title, " ").to_string();
        title = patterns.quality.replace_all(&title, " ").to_string();
        title = patterns.codec.replace_all(&title, " ").to_string();

        // Replace separators with spaces
        title = title.replace(['.', '_', '-'], " ");

        // Collapse multiple spaces and trim
        let mut prev_space = false;
        title = title
            .chars()
            .filter(|c| {
                if c.is_whitespace() {
                    if prev_space {
                        return false;
                    }
                    prev_space = true;
                } else {
                    prev_space = false;
                }
                true
            })
            .collect();

        title.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_movie() {
        let path = PathBuf::from("The.Matrix.1999.1080p.BluRay.x264.mkv");
        let info = Parser::parse(&path);
        assert_eq!(info.title, "The Matrix");
        assert_eq!(info.year, Some(1999));
        assert_eq!(info.resolution, Some("1080P".to_string()));
        assert_eq!(info.hint, MediaHint::Movie);
    }

    #[test]
    fn test_parse_tv_show() {
        let path = PathBuf::from("Breaking.Bad.S01E01.720p.BluRay.mkv");
        let info = Parser::parse(&path);
        assert_eq!(info.title, "Breaking Bad");
        assert_eq!(info.season, Some(1));
        assert_eq!(info.episode, Some(1));
        assert_eq!(info.hint, MediaHint::TvShow);
    }

    #[test]
    fn test_parse_anime_with_group() {
        let path = PathBuf::from("[SubsPlease] Frieren - 01 (1080p) [ABCD1234].mkv");
        let info = Parser::parse(&path);
        assert_eq!(info.title, "Frieren");
        assert_eq!(info.episode, Some(1));
        assert_eq!(info.release_group, Some("SubsPlease".to_string()));
        assert_eq!(info.hint, MediaHint::Anime);
    }

    #[test]
    fn test_parse_anime_simple() {
        let path = PathBuf::from("Sousou no Frieren - 01.mkv");
        let info = Parser::parse(&path);
        assert!(info.title.contains("Frieren"));
        assert_eq!(info.episode, Some(1));
    }

    #[test]
    fn test_parse_movie_with_parens_year() {
        let path = PathBuf::from("Inception (2010) 2160p UHD BluRay.mkv");
        let info = Parser::parse(&path);
        assert_eq!(info.title, "Inception");
        assert_eq!(info.year, Some(2010));
        assert_eq!(info.hint, MediaHint::Movie);
    }
}
