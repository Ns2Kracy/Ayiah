mod filename;
mod patterns;

pub use filename::{ParsedMedia, Parser};
pub use patterns::MediaHint;

#[cfg(test)]
mod test {
    use crate::scraper::parser::{MediaHint, Parser};
    use std::path::PathBuf;

    #[test]
    fn test_parse_standard_movie() {
        let path = PathBuf::from("The.Matrix.1999.1080p.BluRay.x264-GROUP.mkv");
        let info = Parser::parse(&path);

        assert_eq!(info.title, "The Matrix");
        assert_eq!(info.year, Some(1999));
        assert_eq!(info.resolution, Some("1080P".to_string()));
        assert_eq!(info.hint, MediaHint::Movie);
        assert!(info.episode.is_none());
    }

    #[test]
    fn test_parse_movie_with_spaces() {
        let path = PathBuf::from("Inception (2010) 2160p UHD BluRay.mkv");
        let info = Parser::parse(&path);

        assert_eq!(info.title, "Inception");
        assert_eq!(info.year, Some(2010));
        assert_eq!(info.hint, MediaHint::Movie);
    }

    #[test]
    fn test_parse_tv_show_sxxexx() {
        let path = PathBuf::from("Breaking.Bad.S01E01.Pilot.720p.BluRay.mkv");
        let info = Parser::parse(&path);

        assert_eq!(info.title, "Breaking Bad");
        assert_eq!(info.season, Some(1));
        assert_eq!(info.episode, Some(1));
        assert_eq!(info.hint, MediaHint::TvShow);
    }

    #[test]
    fn test_parse_tv_show_lowercase() {
        let path = PathBuf::from("game.of.thrones.s08e06.1080p.mkv");
        let info = Parser::parse(&path);

        assert_eq!(info.title, "game of thrones");
        assert_eq!(info.season, Some(8));
        assert_eq!(info.episode, Some(6));
    }

    #[test]
    fn test_parse_tv_show_x_format() {
        let path = PathBuf::from("Friends.1x01.The.One.Where.Monica.Gets.a.Roommate.mkv");
        let info = Parser::parse(&path);

        assert!(info.title.to_lowercase().contains("friends"));
        assert_eq!(info.season, Some(1));
        assert_eq!(info.episode, Some(1));
    }

    #[test]
    fn test_parse_anime_with_group() {
        let path = PathBuf::from("[SubsPlease] Sousou no Frieren - 01 (1080p) [ABCD1234].mkv");
        let info = Parser::parse(&path);

        assert!(info.title.contains("Frieren"));
        assert_eq!(info.episode, Some(1));
        assert_eq!(info.release_group, Some("SubsPlease".to_string()));
        assert_eq!(info.hint, MediaHint::Anime);
    }

    #[test]
    fn test_parse_anime_simple_dash() {
        let path = PathBuf::from("Bocchi the Rock! - 01.mkv");
        let info = Parser::parse(&path);

        assert!(info.title.contains("Bocchi"));
        assert_eq!(info.episode, Some(1));
    }

    #[test]
    fn test_parse_anime_with_version() {
        let path = PathBuf::from("[Erai-raws] Jujutsu Kaisen - 01v2 [1080p].mkv");
        let info = Parser::parse(&path);

        // Verify episode is parsed correctly
        assert_eq!(info.episode, Some(1));
        // Release group should be detected
        assert_eq!(info.release_group, Some("Erai-raws".to_string()));
        // Title should not be empty
        assert!(!info.title.is_empty());
    }

    #[test]
    fn test_parse_chinese_anime() {
        let path = PathBuf::from("[字幕组] 葬送的芙莉莲 - 01 [1080p].mkv");
        let info = Parser::parse(&path);

        assert!(info.title.contains("芙莉莲"));
        assert_eq!(info.episode, Some(1));
        assert_eq!(info.hint, MediaHint::Anime);
    }

    #[test]
    fn test_parse_extracts_codec() {
        let path = PathBuf::from("Movie.2023.2160p.UHD.BluRay.x265.HEVC.mkv");
        let info = Parser::parse(&path);

        assert!(info.codec.is_some());
        let codec = info.codec.unwrap().to_uppercase();
        assert!(codec.contains("X265") || codec.contains("HEVC"));
    }

    #[test]
    fn test_parse_extracts_quality() {
        let path = PathBuf::from("Movie.2023.WEB-DL.1080p.mkv");
        let info = Parser::parse(&path);

        assert!(info.quality.is_some());
    }
}
