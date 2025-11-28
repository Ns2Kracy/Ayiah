use regex::Regex;
use std::sync::LazyLock;

/// Hint about what type of media this might be
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaHint {
    #[default]
    Unknown,
    Movie,
    TvShow,
    Anime,
}

/// Pre-compiled regex patterns for filename parsing
pub struct Patterns {
    // Year patterns
    pub year: Regex,
    pub year_in_parens: Regex,

    // Episode patterns (ordered by specificity)
    pub season_episode: Regex,   // S01E01, s1e1
    pub season_x_episode: Regex, // 1x01
    pub episode_only: Regex,     // E01, Ep01, EP01
    pub episode_dash: Regex,     // - 01, - 01v2
    pub episode_bracket: Regex,  // [01], [01v2]
    pub episode_number: Regex,   // 01 (at end, after title)

    // Resolution patterns
    pub resolution: Regex,

    // Quality/source patterns
    pub quality: Regex,

    // Codec patterns
    pub codec: Regex,

    // Release group patterns (at start or end)
    pub release_group_start: Regex, // [GroupName]
    pub release_group_end: Regex,   // -GroupName at end

    // Anime-specific patterns
    pub anime_episode: Regex, // [Group] Title - 01 [1080p]

    // Junk patterns to remove
    pub brackets: Regex,
    pub hash: Regex, // [ABCD1234] CRC32 hash
}

impl Patterns {
    pub fn new() -> Self {
        Self {
            // Year: 1900-2099
            year: Regex::new(r"\b(19|20)\d{2}\b").expect("Invalid year regex"),
            year_in_parens: Regex::new(r"\((\d{4})\)").expect("Invalid year_in_parens regex"),

            // Season/Episode patterns
            season_episode: Regex::new(r"(?i)[Ss](\d{1,2})[Ee](\d{1,3})")
                .expect("Invalid season_episode regex"),
            season_x_episode: Regex::new(r"(?i)(\d{1,2})[xX](\d{1,3})")
                .expect("Invalid season_x_episode regex"),
            episode_only: Regex::new(r"(?i)(?:E|EP|Ep)\.?(\d{1,3})")
                .expect("Invalid episode_only regex"),
            episode_dash: Regex::new(r"[-–]\s*(\d{2,3})(?:v\d)?(?:\s|$|\[)")
                .expect("Invalid episode_dash regex"),
            episode_bracket: Regex::new(r"\[(\d{2,3})(?:v\d)?\]")
                .expect("Invalid episode_bracket regex"),
            episode_number: Regex::new(r"(?:^|[\s._-])(\d{2,3})(?:v\d)?(?:[\s._\[\(-]|$)")
                .expect("Invalid episode_number regex"),

            // Resolution
            resolution: Regex::new(r"(?i)(480p|576p|720p|1080p|2160p|4[kK]|UHD)")
                .expect("Invalid resolution regex"),

            // Quality/Source
            quality: Regex::new(
                r"(?i)(HDTV|WEB[-.]?DL|WEB[-.]?Rip|BluRay|BDRip|BRRip|DVDRip|HDCAM|CAM|TS|TC|SCR|R5|DVDScr|DVDR|Remux)",
            )
            .expect("Invalid quality regex"),

            // Codec
            codec: Regex::new(r"(?i)(x264|x265|H\.?264|H\.?265|HEVC|AVC|XviD|DivX|VP9|AV1)")
                .expect("Invalid codec regex"),

            // Release groups
            release_group_start: Regex::new(r"^\[([^\]]+)\]").expect("Invalid release_group_start regex"),
            release_group_end: Regex::new(r"-([A-Za-z0-9]+)(?:\.[a-z]{2,4})?$")
                .expect("Invalid release_group_end regex"),

            // Anime episode pattern: [Group] Title - 01 or Title - 01
            anime_episode: Regex::new(r"(?:\[[^\]]+\]\s*)?(.+?)\s*[-–]\s*(\d{2,3})(?:v\d)?")
                .expect("Invalid anime_episode regex"),

            // Cleanup patterns
            brackets: Regex::new(r"\[[^\]]*\]|\([^)]*\)|\{[^}]*\}")
                .expect("Invalid brackets regex"),
            hash: Regex::new(r"\[[A-Fa-f0-9]{8}\]").expect("Invalid hash regex"),
        }
    }
}

impl Default for Patterns {
    fn default() -> Self {
        Self::new()
    }
}

/// Global singleton for patterns
pub static PATTERNS: LazyLock<Patterns> = LazyLock::new(Patterns::new);
