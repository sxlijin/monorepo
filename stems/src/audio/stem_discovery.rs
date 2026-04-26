use crate::constants::StemType;
use std::path::Path;

pub struct StemDiscovery;

impl StemDiscovery {
    /// Discover stems from a single original file using hardcoded pattern
    /// Expected structure: original_file.wav + stems/Original File/[vocals.wav, bass.wav, drums.wav, other.wav]
    pub fn discover_stems_from_file(original_path: &str) -> Result<Vec<StemMatch>, String> {
        let original_file = Path::new(original_path);

        // Extract basename without extension
        let song_name = original_file
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or("Could not extract song name from file path")?;

        // Construct stems directory path
        let parent_dir = original_file
            .parent()
            .ok_or("Could not determine parent directory")?;
        let stems_dir = parent_dir.join("stems").join(song_name);

        // Hardcoded stem resolution in UI rendering order
        let stem_types = [
            StemType::Vocals,
            StemType::Bass,
            StemType::DrumsHi,
            StemType::DrumsLo,
            StemType::Other,
        ];
        let mut found_stems = Vec::new();

        for stem_type in stem_types.iter() {
            let stem_path = stems_dir.join(format!("{}.wav", stem_type.file_name()));
            let exists = stem_path.exists();

            found_stems.push(StemMatch {
                file_path: stem_path.to_string_lossy().to_string(),
                stem_type: *stem_type,
                exists,
            });
        }

        Ok(found_stems)
    }
}

pub struct StemMatch {
    pub file_path: String,
    pub stem_type: StemType,
    pub exists: bool,
}
