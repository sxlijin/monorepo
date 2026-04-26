use crate::constants::StemType;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

pub type PeaksBuf = Arc<Vec<(f32, f32)>>; // (min, max) normalized to [-1,1]
pub type BeatsBuf = Arc<Vec<f32>>;        // seconds

#[derive(Clone)]
pub struct StemWaveform {
    pub peaks: PeaksBuf,
    pub beats: BeatsBuf,
}

impl StemWaveform {
    fn new(peaks: PeaksBuf, beats: BeatsBuf) -> Self {
        Self { peaks, beats }
    }

    pub(crate) fn empty() -> Self {
        Self {
            peaks: Arc::clone(&EMPTY_PEAKS),
            beats: Arc::clone(&EMPTY_BEATS),
        }
    }
}

struct Inner {
    stems: HashMap<StemType, StemWaveform>,
}

pub struct WaveformRegistry {
    inner: RwLock<Inner>,
    epoch: AtomicU64,
}

static EMPTY_PEAKS: Lazy<PeaksBuf> = Lazy::new(|| Arc::new(Vec::new()));
static EMPTY_BEATS: Lazy<BeatsBuf> = Lazy::new(|| Arc::new(Vec::new()));

static REGISTRY: Lazy<WaveformRegistry> = Lazy::new(|| WaveformRegistry {
    inner: RwLock::new(Inner { stems: HashMap::new() }),
    epoch: AtomicU64::new(0),
});

pub fn get_registry() -> &'static WaveformRegistry { &REGISTRY }

impl WaveformRegistry {
    pub fn set_stem_data(&self, stem: StemType, peaks: PeaksBuf, beats: BeatsBuf) {
        let mut g = self.inner.write().unwrap();
        g.stems.insert(stem, StemWaveform::new(peaks, beats));
        self.epoch.fetch_add(1, Ordering::SeqCst);
    }

    pub fn get_stem(&self, stem: StemType) -> StemWaveform {
        let g = self.inner.read().unwrap();
        g.stems.get(&stem).cloned().unwrap_or_else(StemWaveform::empty)
    }

    pub fn get_peaks(&self, stem: StemType) -> PeaksBuf {
        self.get_stem(stem).peaks
    }

    pub fn get_beats(&self, stem: StemType) -> BeatsBuf {
        self.get_stem(stem).beats
    }

    pub fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::SeqCst)
    }
}
