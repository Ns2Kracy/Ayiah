use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Supported video file extensions
const VIDEO_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "avi", "mov", "wmv", "flv", "webm", "m4v", "iso", "rmvb", "ts", "m2ts",
];

/// Scanner for finding media files
pub struct Scanner;

impl Scanner {
    /// Scan a directory for video files and disc structures
    pub fn scan<P: AsRef<Path>>(path: P) -> Vec<PathBuf> {
        let mut video_files = HashSet::new();

        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let fname = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();

            // Check for Blu-ray structure (BDMV)
            if fname == "index.bdmv" || fname == "movieobject.bdmv" {
                if let Some(grandparent) = path.parent().and_then(|p| p.parent()) {
                    video_files.insert(grandparent.to_path_buf());
                }
                continue;
            }

            // Check for DVD structure (VIDEO_TS)
            if fname == "video_ts.ifo" {
                if let Some(grandparent) = path.parent().and_then(|p| p.parent()) {
                    video_files.insert(grandparent.to_path_buf());
                }
                continue;
            }

            // Check regular video extensions
            if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|ext| VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                .unwrap_or(false)
            {
                // If file is part of a disc structure (inside BDMV or VIDEO_TS), ignore it
                // because we capture the root folder instead.
                if !Self::is_inside_disc_structure(path) {
                    video_files.insert(path.to_path_buf());
                }
            }
        }

        video_files.into_iter().collect()
    }

    /// Check if a path is part of a disc structure (BDMV or VIDEO_TS)
    fn is_inside_disc_structure(path: &Path) -> bool {
        path.components().any(|c| {
            c.as_os_str()
                .to_str()
                .map(|s| s.eq_ignore_ascii_case("BDMV") || s.eq_ignore_ascii_case("VIDEO_TS"))
                .unwrap_or(false)
        })
    }
}
