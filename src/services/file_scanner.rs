use crate::entities::{CreateMediaItem, LibraryFolder, MediaItem, MediaType};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

/// File scanner service for detecting media files
pub struct FileScanner {
    db: sqlx::SqlitePool,
}

/// Scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub total_files: usize,
    pub new_items: usize,
    pub existing_items: usize,
    pub errors: usize,
}

impl FileScanner {
    /// Create a new file scanner
    pub fn new(db: sqlx::SqlitePool) -> Self {
        Self { db }
    }

    /// Scan a library folder for media files
    pub async fn scan_library_folder(
        &self,
        folder: &LibraryFolder,
    ) -> Result<ScanResult, FileScannerError> {
        info!("Scanning library folder: {} ({})", folder.name, folder.path);

        let path = Path::new(&folder.path);
        if !path.exists() {
            return Err(FileScannerError::PathNotFound(folder.path.clone()));
        }

        if !path.is_dir() {
            return Err(FileScannerError::NotADirectory(folder.path.clone()));
        }

        let mut total_files = 0;
        let mut new_items = 0;
        let mut existing_items = 0;
        let mut errors = 0;

        // Get supported extensions for this media type
        let extensions = get_supported_extensions(folder.media_type);
        let mut processed_disc_roots: HashSet<PathBuf> = HashSet::new();

        // Walk through directory
        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let entry_path = entry.path();

            // Skip directories unless they represent disc structures
            if entry_path.is_dir() {
                continue;
            }

            // Handle Blu-ray/DVD disc structures by looking for indicator files
            if let Some(file_name) = entry_path.file_name().and_then(|n| n.to_str())
                && let Some(disc_type) = detect_disc_indicator(file_name)
            {
                if let Some(root) = entry_path.parent().and_then(|p| p.parent())
                    && processed_disc_roots.insert(root.to_path_buf())
                {
                    total_files += 1;
                    let file_path = root.to_string_lossy().to_string();
                    let file_size = calculate_directory_size(root);
                    let title = extract_title(root);

                    self.handle_media_entry(
                        folder,
                        title,
                        file_path,
                        file_size,
                        &mut existing_items,
                        &mut new_items,
                        &mut errors,
                    )
                    .await;
                }

                // We captured the disc root, skip files inside it
                continue;
            }

            if is_inside_disc_structure(entry_path) {
                continue;
            }

            // Check if file has supported extension
            if let Some(ext) = entry_path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if !extensions.contains(&ext_str.as_str()) {
                    continue;
                }
            } else {
                continue;
            }

            total_files += 1;

            // Get file metadata
            let file_path = entry_path.to_string_lossy().to_string();
            let file_size = match entry.metadata() {
                Ok(metadata) => metadata.len() as i64,
                Err(e) => {
                    error!("Failed to get metadata for {}: {}", file_path, e);
                    errors += 1;
                    continue;
                }
            };

            // Extract title from filename
            let title = extract_title(entry_path);

            self.handle_media_entry(
                folder,
                title,
                file_path,
                file_size,
                &mut existing_items,
                &mut new_items,
                &mut errors,
            )
            .await;
        }

        info!(
            "Scan complete: {} total files, {} new, {} existing, {} errors",
            total_files, new_items, existing_items, errors
        );

        Ok(ScanResult {
            total_files,
            new_items,
            existing_items,
            errors,
        })
    }

    /// Scan all enabled library folders
    pub async fn scan_all_libraries(
        &self,
    ) -> Result<Vec<(LibraryFolder, ScanResult)>, FileScannerError> {
        let folders = LibraryFolder::list_enabled(&self.db)
            .await
            .map_err(|e| FileScannerError::DatabaseError(e.to_string()))?;

        let mut results = Vec::new();

        for folder in folders {
            match self.scan_library_folder(&folder).await {
                Ok(result) => {
                    results.push((folder, result));
                }
                Err(e) => {
                    warn!("Failed to scan folder {}: {}", folder.name, e);
                    results.push((
                        folder,
                        ScanResult {
                            total_files: 0,
                            new_items: 0,
                            existing_items: 0,
                            errors: 1,
                        },
                    ));
                }
            }
        }

        Ok(results)
    }

    async fn handle_media_entry(
        &self,
        folder: &LibraryFolder,
        title: String,
        file_path: String,
        file_size: i64,
        existing_items: &mut usize,
        new_items: &mut usize,
        errors: &mut usize,
    ) {
        match MediaItem::find_by_path(&self.db, &file_path).await {
            Ok(Some(_)) => {
                debug!("Media item already exists: {}", file_path);
                *existing_items += 1;
            }
            Ok(None) => {
                let create_item = CreateMediaItem {
                    library_folder_id: folder.id,
                    media_type: folder.media_type,
                    title: title.clone(),
                    file_path: file_path.clone(),
                    file_size,
                };

                match MediaItem::create(&self.db, create_item).await {
                    Ok(_) => {
                        info!("Added new media item: {}", title);
                        *new_items += 1;
                    }
                    Err(e) => {
                        error!("Failed to create media item for {}: {}", file_path, e);
                        *errors += 1;
                    }
                }
            }
            Err(e) => {
                error!("Database error while checking {}: {}", file_path, e);
                *errors += 1;
            }
        }
    }
}

/// Get supported file extensions for a media type
fn get_supported_extensions(media_type: MediaType) -> Vec<&'static str> {
    match media_type {
        MediaType::Movie | MediaType::Tv => vec![
            "mkv", "mp4", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "m2ts", "ts",
            "iso",
        ],
        MediaType::Comic => vec!["cbz", "cbr", "cb7", "cbt", "pdf"],
        MediaType::Book => vec!["epub", "mobi", "azw3", "pdf"],
    }
}

/// Extract title from file path
fn extract_title(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiscType {
    BluRay,
    Dvd,
}

fn detect_disc_indicator(file_name: &str) -> Option<DiscType> {
    match file_name.to_ascii_lowercase().as_str() {
        "index.bdmv" | "movieobject.bdmv" => Some(DiscType::BluRay),
        "video_ts.ifo" => Some(DiscType::Dvd),
        _ => None,
    }
}

fn is_inside_disc_structure(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|s| s.eq_ignore_ascii_case("BDMV") || s.eq_ignore_ascii_case("VIDEO_TS"))
            .unwrap_or(false)
    })
}

fn calculate_directory_size(path: &Path) -> i64 {
    let mut total: i64 = 0;
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.path().is_file() {
            match entry.metadata() {
                Ok(metadata) => total += metadata.len() as i64,
                Err(e) => warn!("Failed to read metadata for {:?}: {}", entry.path(), e),
            }
        }
    }
    total
}

/// File scanner errors
#[derive(Debug, thiserror::Error)]
pub enum FileScannerError {
    #[error("Path not found: {0}")]
    PathNotFound(String),

    #[error("Not a directory: {0}")]
    NotADirectory(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_disc_indicator() {
        assert_eq!(detect_disc_indicator("index.bdmv"), Some(DiscType::BluRay));
        assert_eq!(
            detect_disc_indicator("movieobject.bdmv"),
            Some(DiscType::BluRay)
        );
        assert_eq!(detect_disc_indicator("video_ts.ifo"), Some(DiscType::Dvd));
        assert!(detect_disc_indicator("random.mkv").is_none());
    }

    #[test]
    fn test_is_inside_disc_structure() {
        let bluray_path = Path::new("Movie")
            .join("BDMV")
            .join("STREAM")
            .join("00001.m2ts");
        assert!(is_inside_disc_structure(&bluray_path));

        let dvd_path = Path::new("Movie").join("VIDEO_TS").join("VIDEO_TS.IFO");
        assert!(is_inside_disc_structure(&dvd_path));

        let regular_file = Path::new("Movie.mkv");
        assert!(!is_inside_disc_structure(regular_file));
    }
}
