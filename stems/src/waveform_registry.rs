use crate::analysis::RawWaveformData;
use crate::constants::StemType;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

pub struct WaveformRegistry {
    stems: HashMap<StemType, Arc<RawWaveformData>>,
}

impl WaveformRegistry {
    pub fn new() -> Self {
        Self {
            stems: HashMap::new(),
        }
    }

    pub fn set_waveform_data(&mut self, stem: StemType, data: RawWaveformData) {
        self.stems.insert(stem, Arc::new(data));
    }

    pub fn get_waveform_data(&self, stem: StemType) -> Option<Arc<RawWaveformData>> {
        self.stems.get(&stem).cloned()
    }

    pub fn clear(&mut self) {
        self.stems.clear();
    }

    pub fn is_ready(&self, stem: StemType) -> bool {
        self.stems.contains_key(&stem)
    }

    pub fn ready_count(&self) -> usize {
        self.stems.len()
    }
}

pub static WAVEFORM_REGISTRY: LazyLock<Mutex<WaveformRegistry>> =
    LazyLock::new(|| Mutex::new(WaveformRegistry::new()));
