use regex::Regex;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedInfo {
    pub title: String,
    pub year: Option<i32>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub resolution: Option<String>,
}

pub struct Parser;

impl Parser {
    pub fn parse(path: &Path) -> ParsedInfo {
        let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        // Common regex patterns
        let year_regex = Regex::new(r"\b(19|20)\d{2}\b").unwrap();
        let s_e_regex = Regex::new(r"(?i)S(\d{1,2})E(\d{1,2})").unwrap();
        let resolution_regex = Regex::new(r"(?i)(480p|720p|1080p|2160p|4k)").unwrap();

        // Extract year
        let year = year_regex
            .find(filename)
            .map(|m| m.as_str().parse().unwrap_or(0));

        // Extract Season/Episode
        let (season, episode) = if let Some(caps) = s_e_regex.captures(filename) {
            (
                caps.get(1).map(|m| m.as_str().parse().unwrap_or(0)),
                caps.get(2).map(|m| m.as_str().parse().unwrap_or(0)),
            )
        } else {
            (None, None)
        };

        // Extract Resolution
        let resolution = resolution_regex
            .find(filename)
            .map(|m| m.as_str().to_string());

        // Clean Title
        // 1. Replace dots, underscores with spaces
        let mut title = filename.replace(['.', '_'], " ");

        // 2. Remove Year and everything after it
        if let Some(m) = year_regex.find(filename) {
            let index = title.find(m.as_str()).unwrap_or(title.len());
            title.truncate(index);
        } else if let Some(m) = s_e_regex.find(filename) {
            // If no year, but SxxExx, truncate before SxxExx
            // Note: we need to find where it is in the modified title (spaces)
            // This is tricky because indices change.
            // Simplification: Regard the match start in original string as cutoff point.
            let index = m.start();
            if index < title.len() {
                title.truncate(index);
            }
        }

        // 3. Trim parens and brackets
        let junk_regex = Regex::new(r"[\(\[\{].*?[\)\]\}]").unwrap();
        title = junk_regex.replace_all(&title, "").to_string();

        // 4. Trim whitespace
        title = title.trim().to_string();

        ParsedInfo {
            title,
            year: year.filter(|&y| y > 0),
            season,
            episode,
            resolution,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_movie() {
        let path = PathBuf::from("The.Matrix.1999.1080p.mkv");
        let info = Parser::parse(&path);
        assert_eq!(info.title, "The Matrix");
        assert_eq!(info.year, Some(1999));
    }

    #[test]
    fn test_parse_tv() {
        let path = PathBuf::from("Breaking.Bad.S01E01.720p.mkv");
        let info = Parser::parse(&path);
        assert_eq!(info.title, "Breaking Bad");
        assert_eq!(info.season, Some(1));
        assert_eq!(info.episode, Some(1));
    }
}
