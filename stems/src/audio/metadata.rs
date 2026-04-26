use anyhow::{Context, Result};
use std::path::Path;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey};
use symphonia::core::probe::Hint;

/// Song metadata extracted from audio files
#[derive(Debug, Clone)]
pub struct SongMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<f64>,
    pub bpm: Option<f64>,
}

impl Default for SongMetadata {
    fn default() -> Self {
        Self {
            title: None,
            artist: None,
            album: None,
            duration: None,
            bpm: None,
        }
    }
}

impl SongMetadata {
    /// Get the display title, falling back to a placeholder if none available
    pub fn display_title(&self) -> String {
        self.title
            .clone()
            .unwrap_or_else(|| "Placeholder Title".to_string())
    }

    /// Get the display artist, falling back to a placeholder if none available
    pub fn display_artist(&self) -> String {
        self.artist
            .clone()
            .unwrap_or_else(|| "Placeholder Artist".to_string())
    }

    /// Get the display BPM, formatted as "120 BPM" if available
    pub fn display_bpm(&self) -> Option<String> {
        self.bpm.map(|bpm| format!("{:.0} BPM", bpm.round()))
    }
}

/// Extract metadata from an audio file using Symphonia
pub fn extract_metadata<P: AsRef<Path>>(file_path: P) -> Result<SongMetadata> {
    let path = file_path.as_ref();

    tracing::debug!("Extracting metadata from: {}", path.display());

    // Open the media source
    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open file: {}", path.display()))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create a probe hint with the file extension
    let mut hint = Hint::new();
    if let Some(extension) = path.extension() {
        if let Some(extension_str) = extension.to_str() {
            hint.with_extension(extension_str);
        }
    }

    // Use the default options
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    // Probe the media source
    let mut probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts)
        .with_context(|| format!("Failed to probe audio format for: {}", path.display()))?;

    let mut metadata = SongMetadata::default();

    // Extract metadata from the format
    if let Some(metadata_rev) = probed.format.metadata().current() {
        tracing::debug!("Found {} metadata tags", metadata_rev.tags().len());

        for tag in metadata_rev.tags() {
            match &tag.std_key {
                Some(StandardTagKey::TrackTitle) => {
                    metadata.title = Some(tag.value.to_string());
                    tracing::debug!("Found title: {}", tag.value);
                }
                Some(StandardTagKey::Artist) => {
                    metadata.artist = Some(tag.value.to_string());
                    tracing::debug!("Found artist: {}", tag.value);
                }
                Some(StandardTagKey::Album) => {
                    metadata.album = Some(tag.value.to_string());
                    tracing::debug!("Found album: {}", tag.value);
                }
                Some(StandardTagKey::Bpm) => {
                    if let Ok(bpm_value) = tag.value.to_string().parse::<f64>() {
                        metadata.bpm = Some(bpm_value);
                        tracing::debug!("Found BPM: {}", bpm_value);
                    }
                }
                _ => {
                    // Log other tags for debugging
                    tracing::debug!("Other tag: {:?} = {}", tag.std_key, tag.value);
                }
            }
        }
    } else {
        tracing::debug!("No metadata found in format");
    }

    // Try to get duration from the format if available
    if let Some(track) = probed.format.tracks().iter().next() {
        if let Some(time_base) = track.codec_params.time_base {
            if let Some(n_frames) = track.codec_params.n_frames {
                let duration_secs =
                    (n_frames as f64) / (time_base.numer as f64 / time_base.denom as f64);
                metadata.duration = Some(duration_secs);
                tracing::debug!("Found duration: {:.2} seconds", duration_secs);
            }
        }
    }

    tracing::debug!("Extracted metadata: {:?}", metadata);
    Ok(metadata)
}

/// Extract metadata from the first file in a list, used for getting song info from stems
pub fn extract_metadata_from_first_file(file_paths: &[String]) -> Result<SongMetadata> {
    if file_paths.is_empty() {
        return Ok(SongMetadata::default());
    }

    // Try the first file
    match extract_metadata(&file_paths[0]) {
        Ok(metadata) => {
            tracing::debug!("Successfully extracted metadata from first file");
            Ok(metadata)
        }
        Err(e) => {
            tracing::warn!("Failed to extract metadata from first file: {}", e);

            // If first file fails, try other files in case stems are in different order
            for (i, path) in file_paths.iter().enumerate().skip(1) {
                match extract_metadata(path) {
                    Ok(metadata) => {
                        tracing::debug!("Successfully extracted metadata from file {}", i);
                        return Ok(metadata);
                    }
                    Err(e) => {
                        tracing::debug!("Failed to extract metadata from file {}: {}", i, e);
                    }
                }
            }

            tracing::warn!("Failed to extract metadata from any file, using defaults");
            Ok(SongMetadata::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_methods() {
        let metadata = SongMetadata {
            title: Some("Test Song".to_string()),
            artist: Some("Test Artist".to_string()),
            album: None,
            duration: None,
            bpm: None,
        };

        assert_eq!(metadata.display_title(), "Test Song");
        assert_eq!(metadata.display_artist(), "Test Artist");
    }

    #[test]
    fn test_display_methods_with_fallbacks() {
        let metadata = SongMetadata::default();

        assert_eq!(metadata.display_title(), "Placeholder Title");
        assert_eq!(metadata.display_artist(), "Placeholder Artist");
    }
}
