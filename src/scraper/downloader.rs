use anyhow::Result;
use std::path::Path;
use tokio::io::AsyncWriteExt;

/// Downloader for media assets
pub struct Downloader;

impl Downloader {
    /// Download an image from a URL to a specific path
    pub async fn download_image(url: &str, output_path: &Path) -> Result<()> {
        if url.is_empty() {
            return Ok(());
        }

        let response = reqwest::get(url).await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download image: {}",
                response.status()
            ));
        }

        let bytes = response.bytes().await?;

        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut file = tokio::fs::File::create(output_path).await?;
        file.write_all(&bytes).await?;

        Ok(())
    }
}
