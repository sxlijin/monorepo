/// WAV file paths available in the repository for testing and development
pub const WAV_FILES: &[&str] = &[
    // Test file
    "./demucs-sandbox/ta_test.wav",
    // Leikeli47 - Money (separated stems)
    "./demucs-sandbox/separated/htdemucs/Leikeli47 - Money/drums-hi.wav",
    "./demucs-sandbox/separated/htdemucs/Leikeli47 - Money/drums-lo.wav",
    "./demucs-sandbox/separated/htdemucs/Leikeli47 - Money/vocals.wav",
    "./demucs-sandbox/separated/htdemucs/Leikeli47 - Money/other.wav",
    "./demucs-sandbox/separated/htdemucs/Leikeli47 - Money/bass.wav",
    // Alannah Myles - Black Velvet 0 (separated stems)
    "./demucs-sandbox/separated/htdemucs/Alannah Myles - Black Velvet 0/drums-hi.wav",
    "./demucs-sandbox/separated/htdemucs/Alannah Myles - Black Velvet 0/drums-lo.wav",
    "./demucs-sandbox/separated/htdemucs/Alannah Myles - Black Velvet 0/vocals.wav",
    "./demucs-sandbox/separated/htdemucs/Alannah Myles - Black Velvet 0/other.wav",
    "./demucs-sandbox/separated/htdemucs/Alannah Myles - Black Velvet 0/bass.wav",
];

/// Stem types available in the separated audio files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StemType {
    Vocals,
    Bass,
    DrumsHi,
    DrumsLo,
    Other,
}

impl StemType {
    pub fn file_name(self) -> &'static str {
        match self {
            StemType::DrumsHi => "drums-hi",
            StemType::DrumsLo => "drums-lo",
            StemType::Vocals => "vocals",
            StemType::Other => "other",
            StemType::Bass => "bass",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            StemType::DrumsHi => "drums-hi",
            StemType::DrumsLo => "drums-lo",
            StemType::Vocals => "vocals",
            StemType::Other => "other",
            StemType::Bass => "bass",
        }
    }

    pub const DISPLAY_ORDER: [StemType; 5] = [
        StemType::Vocals,
        StemType::Bass,
        StemType::DrumsHi,
        StemType::DrumsLo,
        StemType::Other,
    ];

    pub fn into_index(self) -> usize {
        match self {
            StemType::Vocals => 0,
            StemType::Bass => 1,
            StemType::DrumsHi => 2,
            StemType::DrumsLo => 3,
            StemType::Other => 4,
        }
    }

    pub fn from_index(index: usize) -> Option<Self> {
        StemType::DISPLAY_ORDER.get(index).copied()
    }
}

/// Available song datasets
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SongDataset {
    Leikeli47Money,
    AlannahMylesBlackVelvet,
}

impl SongDataset {
    pub fn as_str(&self) -> &'static str {
        match self {
            SongDataset::Leikeli47Money => "Leikeli47 - Money",
            SongDataset::AlannahMylesBlackVelvet => "Alannah Myles - Black Velvet 0",
        }
    }

    pub fn get_stem_path(&self, stem: StemType) -> &'static str {
        match self {
            SongDataset::Leikeli47Money => match stem {
                StemType::Vocals => {
                    "./demucs-sandbox/separated/htdemucs/Leikeli47 - Money/vocals.wav"
                }
                StemType::Other => {
                    "./demucs-sandbox/separated/htdemucs/Leikeli47 - Money/other.wav"
                }
                StemType::Bass => "./demucs-sandbox/separated/htdemucs/Leikeli47 - Money/bass.wav",
                StemType::DrumsHi => {
                    "./demucs-sandbox/separated/htdemucs/Leikeli47 - Money/drums-hi.wav"
                }
                StemType::DrumsLo => {
                    "./demucs-sandbox/separated/htdemucs/Leikeli47 - Money/drums-lo.wav"
                }
            },
            SongDataset::AlannahMylesBlackVelvet => match stem {
                StemType::Vocals => {
                    "./demucs-sandbox/separated/htdemucs/Alannah Myles - Black Velvet 0/vocals.wav"
                }
                StemType::Other => {
                    "./demucs-sandbox/separated/htdemucs/Alannah Myles - Black Velvet 0/other.wav"
                }
                StemType::Bass => {
                    "./demucs-sandbox/separated/htdemucs/Alannah Myles - Black Velvet 0/bass.wav"
                }
                StemType::DrumsHi => {
                    "./demucs-sandbox/separated/htdemucs/Alannah Myles - Black Velvet 0/drums-hi.wav"
                }
                StemType::DrumsLo => {
                    "./demucs-sandbox/separated/htdemucs/Alannah Myles - Black Velvet 0/drums-lo.wav"
                }
            },
        }
    }
}

/// All available songs
pub const SONGS: &[SongDataset] = &[
    SongDataset::Leikeli47Money,
    SongDataset::AlannahMylesBlackVelvet,
];

/// All available stem types
pub const STEM_TYPES: &[StemType] = &StemType::DISPLAY_ORDER;
