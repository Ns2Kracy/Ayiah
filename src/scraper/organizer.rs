//! Media file organizer - organize media files into structured directories

use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use super::{MediaMetadata, MediaType, ParsedMedia, Parser, ScraperError, ScraperManager};

/// Organization method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrganizeMethod {
    /// Create symbolic links (default, safest)
    #[default]
    Symlink,
    /// Create hard links (same filesystem only)
    Hardlink,
    /// Move files (destructive)
    Move,
    /// Copy files
    Copy,
}

impl std::fmt::Display for OrganizeMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Symlink => write!(f, "symlink"),
            Self::Hardlink => write!(f, "hardlink"),
            Self::Move => write!(f, "move"),
            Self::Copy => write!(f, "copy"),
        }
    }
}

impl std::str::FromStr for OrganizeMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "symlink" | "sym" | "soft" => Ok(Self::Symlink),
            "hardlink" | "hard" => Ok(Self::Hardlink),
            "move" | "mv" => Ok(Self::Move),
            "copy" | "cp" => Ok(Self::Copy),
            _ => Err(format!("Unknown method: {s}")),
        }
    }
}

/// Naming template for organized files
#[derive(Debug, Clone)]
pub struct NamingTemplate {
    /// Movie folder: {title} ({year})
    pub movie_folder: String,
    /// Movie file: {title} ({year})
    pub movie_file: String,
    /// TV show folder: {title} ({year})
    pub tv_folder: String,
    /// Season folder: Season {season:02}
    pub season_folder: String,
    /// Episode file: {title} - S{season:02}E{episode:02}
    pub episode_file: String,
}

impl Default for NamingTemplate {
    fn default() -> Self {
        Self {
            movie_folder: "{title} ({year})".to_string(),
            movie_file: "{title} ({year})".to_string(),
            tv_folder: "{title} ({year})".to_string(),
            season_folder: "Season {season:02}".to_string(),
            episode_file: "{title} - S{season:02}E{episode:02}".to_string(),
        }
    }
}

/// Organizer configuration
#[derive(Debug, Clone)]
pub struct OrganizerConfig {
    /// Source directory to scan
    pub source_dir: PathBuf,
    /// Target directory for organized files
    pub target_dir: PathBuf,
    /// Organization method
    pub method: OrganizeMethod,
    /// Naming template
    pub template: NamingTemplate,
    /// Whether to create separate directories for Movies/TV/Anime
    pub separate_by_type: bool,
    /// Dry run mode (don't actually move/link files)
    pub dry_run: bool,
    /// Whether to overwrite existing files
    pub overwrite: bool,
}

impl Default for OrganizerConfig {
    fn default() -> Self {
        Self {
            source_dir: PathBuf::new(),
            target_dir: PathBuf::new(),
            method: OrganizeMethod::Symlink,
            template: NamingTemplate::default(),
            separate_by_type: true,
            dry_run: false,
            overwrite: false,
        }
    }
}

/// Result of organizing a single file
#[derive(Debug, Clone)]
pub struct OrganizeResult {
    /// Original file path
    pub source: PathBuf,
    /// Target file path
    pub target: PathBuf,
    /// Whether the operation succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Parsed media info
    pub parsed: ParsedMedia,
    /// Matched metadata (if any)
    pub metadata: Option<MediaMetadata>,
}

/// Batch organize result
#[derive(Debug, Default)]
pub struct BatchOrganizeResult {
    /// Successfully organized files
    pub success: Vec<OrganizeResult>,
    /// Failed files
    pub failed: Vec<OrganizeResult>,
    /// Skipped files (not video, already exists, etc.)
    pub skipped: Vec<(PathBuf, String)>,
}

impl BatchOrganizeResult {
    pub fn total(&self) -> usize {
        self.success.len() + self.failed.len() + self.skipped.len()
    }

    pub fn success_count(&self) -> usize {
        self.success.len()
    }

    pub fn failed_count(&self) -> usize {
        self.failed.len()
    }
}

/// Media file organizer
pub struct Organizer {
    config: OrganizerConfig,
    scraper: Option<ScraperManager>,
}

#[cfg(unix)]
fn create_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(windows)]
fn create_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(src, dst)
}

impl Organizer {
    /// Create a new organizer with configuration
    pub fn new(config: OrganizerConfig) -> Self {
        Self {
            config,
            scraper: None,
        }
    }

    /// Set scraper manager for metadata lookup
    pub fn with_scraper(mut self, scraper: ScraperManager) -> Self {
        self.scraper = Some(scraper);
        self
    }

    /// Organize all media files in the source directory
    pub async fn organize_all(&self) -> Result<BatchOrganizeResult, ScraperError> {
        let mut result = BatchOrganizeResult::default();

        // Scan source directory for video files
        let files = self.scan_video_files(&self.config.source_dir)?;

        info!(
            "Found {} video files in {:?}",
            files.len(),
            self.config.source_dir
        );

        for file in files {
            match self.organize_file(&file).await {
                Ok(r) => {
                    if r.success {
                        result.success.push(r);
                    } else {
                        result.failed.push(r);
                    }
                }
                Err(e) => {
                    result.skipped.push((file, e.to_string()));
                }
            }
        }

        info!(
            "Organize complete: {} success, {} failed, {} skipped",
            result.success_count(),
            result.failed_count(),
            result.skipped.len()
        );

        Ok(result)
    }

    /// Organize a single file
    pub async fn organize_file(&self, source: &Path) -> Result<OrganizeResult, ScraperError> {
        // Parse filename
        let parsed = Parser::parse(source);

        // Try to get metadata from scraper
        let metadata = if let Some(ref scraper) = self.scraper {
            let media_type = match parsed.hint {
                super::MediaHint::Movie => Some(MediaType::Movie),
                super::MediaHint::TvShow => Some(MediaType::Tv),
                super::MediaHint::Anime => Some(MediaType::Anime),
                super::MediaHint::Unknown => None,
            };

            match scraper
                .search_ranked(&parsed.title, parsed.year, media_type)
                .await
            {
                Ok(results) => {
                    if let Some(best) = results.into_iter().next() {
                        match scraper.get_metadata(&best.info).await {
                            Ok(meta) => Some(meta),
                            Err(e) => {
                                warn!("Failed to get metadata for {:?}: {}", source, e);
                                None
                            }
                        }
                    } else {
                        None
                    }
                }
                Err(e) => {
                    warn!("Failed to search for {:?}: {}", source, e);
                    None
                }
            }
        } else {
            None
        };

        // Build target path
        let target = self.build_target_path(source, &parsed, metadata.as_ref())?;

        // Perform the organization
        let (success, error) = if self.config.dry_run {
            info!(
                "[DRY RUN] Would {} {:?} -> {:?}",
                self.config.method,
                source.file_name().unwrap_or_default(),
                target
            );
            (true, None)
        } else {
            self.perform_organize(source, &target)
        };

        Ok(OrganizeResult {
            source: source.to_path_buf(),
            target,
            success,
            error,
            parsed,
            metadata,
        })
    }

    /// Build target path based on parsed info and metadata
    fn build_target_path(
        &self,
        source: &Path,
        parsed: &ParsedMedia,
        metadata: Option<&MediaMetadata>,
    ) -> Result<PathBuf, ScraperError> {
        let mut target = self.config.target_dir.clone();

        // Get title and year from metadata or parsed info
        let title = metadata
            .map(|m| m.title.clone())
            .unwrap_or_else(|| sanitize_filename(&parsed.title));

        let year = metadata
            .and_then(|m| m.release_date.as_ref())
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse::<i32>().ok())
            .or(parsed.year);

        let media_type = metadata
            .map(|m| m.media_type)
            .unwrap_or_else(|| match parsed.hint {
                super::MediaHint::Movie => MediaType::Movie,
                super::MediaHint::TvShow => MediaType::Tv,
                super::MediaHint::Anime => MediaType::Anime,
                super::MediaHint::Unknown => MediaType::Unknown,
            });

        // Add type directory if configured
        if self.config.separate_by_type {
            let type_dir = match media_type {
                MediaType::Movie => "Movies",
                MediaType::Tv => "TV Shows",
                MediaType::Anime => "Anime",
                _ => "Other",
            };
            target.push(type_dir);
        }

        // Get file extension
        let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("mkv");

        // Build path based on media type
        match media_type {
            MediaType::Movie => {
                // Movies/{title} ({year})/{title} ({year}).ext
                let folder_name = self.format_template(
                    &self.config.template.movie_folder,
                    &title,
                    year,
                    None,
                    None,
                );
                let file_name = self.format_template(
                    &self.config.template.movie_file,
                    &title,
                    year,
                    None,
                    None,
                );
                target.push(sanitize_filename(&folder_name));
                target.push(format!("{}.{}", sanitize_filename(&file_name), ext));
            }
            _ => {
                // TV Shows/{title} ({year})/Season XX/{title} - SXXEXX.ext
                let folder_name =
                    self.format_template(&self.config.template.tv_folder, &title, year, None, None);
                target.push(sanitize_filename(&folder_name));

                let season = parsed.season.unwrap_or(1);
                let season_folder = self.format_template(
                    &self.config.template.season_folder,
                    &title,
                    year,
                    Some(season),
                    None,
                );
                target.push(sanitize_filename(&season_folder));

                let episode = parsed.episode.unwrap_or(1);
                let file_name = self.format_template(
                    &self.config.template.episode_file,
                    &title,
                    year,
                    Some(season),
                    Some(episode),
                );
                target.push(format!("{}.{}", sanitize_filename(&file_name), ext));
            }
        }

        Ok(target)
    }

    /// Format a naming template
    fn format_template(
        &self,
        template: &str,
        title: &str,
        year: Option<i32>,
        season: Option<i32>,
        episode: Option<i32>,
    ) -> String {
        let mut result = template.to_string();

        result = result.replace("{title}", title);

        if let Some(y) = year {
            result = result.replace("{year}", &y.to_string());
        } else {
            // Remove year placeholder and surrounding parentheses if no year
            result = result.replace(" ({year})", "");
            result = result.replace("({year})", "");
            result = result.replace("{year}", "");
        }

        if let Some(s) = season {
            result = result.replace("{season:02}", &format!("{:02}", s));
            result = result.replace("{season}", &s.to_string());
        }

        if let Some(e) = episode {
            result = result.replace("{episode:02}", &format!("{:02}", e));
            result = result.replace("{episode}", &e.to_string());
        }

        result
    }

    /// Perform the actual file organization
    fn perform_organize(&self, source: &Path, target: &Path) -> (bool, Option<String>) {
        // Create parent directories
        if let Some(parent) = target.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            return (false, Some(format!("Failed to create directory: {e}")));
        }

        // Check if target already exists
        if target.exists() && !self.config.overwrite {
            return (false, Some("Target already exists".to_string()));
        }

        // Remove existing target if overwriting
        if target.exists()
            && self.config.overwrite
            && let Err(e) = fs::remove_file(target)
        {
            return (false, Some(format!("Failed to remove existing file: {e}")));
        }

        // Perform the operation
        let result = match self.config.method {
            OrganizeMethod::Symlink => {
                // Use absolute path for symlink source
                let abs_source = if source.is_absolute() {
                    source.to_path_buf()
                } else {
                    std::env::current_dir()
                        .map(|cwd| cwd.join(source))
                        .unwrap_or_else(|_| source.to_path_buf())
                };
                create_symlink(&abs_source, target)
            }
            OrganizeMethod::Hardlink => fs::hard_link(source, target),
            OrganizeMethod::Move => fs::rename(source, target),
            OrganizeMethod::Copy => fs::copy(source, target).map(|_| ()),
        };

        match result {
            Ok(()) => {
                info!(
                    "{} {:?} -> {:?}",
                    self.config.method,
                    source.file_name().unwrap_or_default(),
                    target
                );
                (true, None)
            }
            Err(e) => (false, Some(e.to_string())),
        }
    }

    /// Scan directory for video files
    fn scan_video_files(&self, dir: &Path) -> Result<Vec<PathBuf>, ScraperError> {
        let mut files = Vec::new();

        if !dir.is_dir() {
            return Err(ScraperError::Config(format!(
                "{:?} is not a directory",
                dir
            )));
        }

        Self::scan_recursive(dir, &mut files)?;

        Ok(files)
    }

    fn scan_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), ScraperError> {
        let entries = fs::read_dir(dir).map_err(|e| {
            ScraperError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read {:?}: {}", dir, e),
            ))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                Self::scan_recursive(&path, files)?;
            } else if is_video_file(&path) {
                files.push(path);
            }
        }

        Ok(())
    }
}

/// Check if a file is a video file
fn is_video_file(path: &Path) -> bool {
    const VIDEO_EXTENSIONS: &[&str] = &[
        "mkv", "mp4", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "ts", "m2ts",
    ];

    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| VIDEO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Sanitize a string for use as a filename
fn sanitize_filename(name: &str) -> String {
    // Characters not allowed in filenames on various systems
    const INVALID_CHARS: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

    let mut result: String = name
        .chars()
        .map(|c| if INVALID_CHARS.contains(&c) { '_' } else { c })
        .collect();

    // Trim whitespace and dots from ends
    result = result.trim().trim_matches('.').to_string();

    // Collapse multiple spaces/underscores
    while result.contains("  ") {
        result = result.replace("  ", " ");
    }
    while result.contains("__") {
        result = result.replace("__", "_");
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Movie: The Title"), "Movie_ The Title");
        assert_eq!(sanitize_filename("What?"), "What_");
        assert_eq!(sanitize_filename("A/B\\C"), "A_B_C");
        assert_eq!(sanitize_filename("  spaces  "), "spaces");
    }

    #[test]
    fn test_format_template() {
        let org = Organizer::new(OrganizerConfig::default());

        assert_eq!(
            org.format_template("{title} ({year})", "The Matrix", Some(1999), None, None),
            "The Matrix (1999)"
        );

        assert_eq!(
            org.format_template(
                "{title} - S{season:02}E{episode:02}",
                "Breaking Bad",
                None,
                Some(1),
                Some(5)
            ),
            "Breaking Bad - S01E05"
        );

        // No year
        assert_eq!(
            org.format_template("{title} ({year})", "Unknown Movie", None, None, None),
            "Unknown Movie"
        );
    }

    #[test]
    fn test_organize_method_parse() {
        assert_eq!(
            "symlink".parse::<OrganizeMethod>().unwrap(),
            OrganizeMethod::Symlink
        );
        assert_eq!(
            "hard".parse::<OrganizeMethod>().unwrap(),
            OrganizeMethod::Hardlink
        );
        assert_eq!(
            "move".parse::<OrganizeMethod>().unwrap(),
            OrganizeMethod::Move
        );
        assert_eq!(
            "copy".parse::<OrganizeMethod>().unwrap(),
            OrganizeMethod::Copy
        );
    }
}
