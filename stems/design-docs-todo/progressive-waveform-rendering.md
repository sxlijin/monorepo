# Progressive Waveform Rendering Design

## Problem Statement

Currently, waveform loading blocks the UI until the entire waveform is generated. Users must wait for complete waveform analysis before seeing any visualization, which creates a poor user experience especially for longer audio files.

**Key Issues:**
- Full blocking during waveform generation (`WaveformAnalyzer::generate_peaks_from_file`)
- UI shows "Loading waveform..." until 100% complete
- No visual feedback during the generation process
- Poor perceived performance for large audio files

## Current Implementation Analysis

### Waveform Generation Flow

1. **File Loading** (`multi_bridge.rs:406-407`):
   - `start_background_waveform_generation()` called after audio engine initialization
   - Spawns background thread for waveform processing

2. **Background Processing** (`multi_bridge.rs:593-623`):
   ```rust
   thread::spawn(move || {
       for (index, file_path) in file_paths.iter().enumerate() {
           match WaveformAnalyzer::generate_peaks_from_file(file_path, SAMPLES_PER_PIXEL) {
               Ok(waveform_data) => {
                   // Store complete waveform in cache
                   cache[index] = Some(waveform_data);
               }
               Err(e) => { /* error handling */ }
           }
       }
   });
   ```

3. **Peak Generation** (`waveform.rs:34-122`):
   - `generate_peaks_from_file()` processes entire audio file at once
   - `generate_waveform_peaks()` iterates through all samples sequentially
   - Returns complete `WaveformData` structure only when finished

4. **UI Updates** (`multi_player.qml:142-143`):
   - Timer polls `check_waveform_progress()` every 50ms
   - Canvas shows "Loading waveform..." until `is_waveform_ready()` returns true

### Bottlenecks Identified

1. **Monolithic Processing**: Entire waveform must be generated before any data is available
2. **No Incremental Updates**: UI cannot render partial waveforms
3. **Memory Allocation**: Large peak arrays allocated all at once
4. **Beat Detection Blocking**: Python beat detection runs synchronously per file
5. **No Progress Granularity**: Progress tracking is binary (0% or 100% per file)

## Proposed Solution: Chunked Progressive Rendering

### Architecture Overview

Transform the waveform generation from a monolithic process to a chunked, progressive system that can render partial waveforms as they become available.

### Core Design Changes

#### 1. Chunked Peak Generation

**Replace monolithic generation with chunk-based approach:**

```rust
// New chunk-based API
pub struct WaveformChunk {
    pub chunk_id: usize,
    pub peaks: Vec<WaveformPeak>,
    pub start_sample: usize,
    pub end_sample: usize,
    pub total_chunks: usize,
}

impl WaveformAnalyzer {
    pub fn generate_peaks_chunked(
        file_path: &str,
        samples_per_pixel: usize,
        chunk_size_seconds: f64,
        callback: impl Fn(WaveformChunk) + Send + Sync,
    ) -> Result<()> {
        // Generate chunks progressively
        // Call callback for each completed chunk
    }
}
```

#### 2. Progressive Cache Management

**Modify waveform cache to support partial data:**

```rust
// In multi_bridge.rs
pub struct ProgressiveWaveformData {
    pub chunks: Vec<Option<WaveformChunk>>,
    pub total_chunks: usize,
    pub completed_chunks: usize,
    pub duration_seconds: f64,
    pub beat_timestamps: Vec<f64>, // Added when available
    pub tempo: Option<f64>,
}

// Replace Vec<Option<WaveformData>> with Vec<Option<ProgressiveWaveformData>>
waveform_cache: Arc<Mutex<Vec<Option<ProgressiveWaveformData>>>>
```

#### 3. Real-time UI Updates

**Enable Canvas to render partial waveforms:**

```javascript
// In multi_player.qml Canvas component
function updateWaveformChunk(fileIndex, chunkData) {
    if (!progressiveWaveformData[fileIndex]) {
        progressiveWaveformData[fileIndex] = {
            chunks: new Array(chunkData.total_chunks).fill(null),
            completed: 0,
            total: chunkData.total_chunks
        };
    }
    
    // Update specific chunk
    progressiveWaveformData[fileIndex].chunks[chunkData.chunk_id] = chunkData.peaks;
    progressiveWaveformData[fileIndex].completed++;
    
    // Trigger immediate repaint
    requestPaint();
}
```

#### 4. Progressive Rendering Logic

**Canvas renders available chunks while showing placeholders for pending ones:**

```javascript
function renderProgressiveWaveform(ctx) {
    let waveformData = progressiveWaveformData[stemIndex];
    if (!waveformData) return;
    
    for (let chunkId = 0; chunkId < waveformData.total; chunkId++) {
        let chunk = waveformData.chunks[chunkId];
        
        if (chunk) {
            // Render completed chunk with full fidelity
            renderWaveformChunk(ctx, chunk, chunkId);
        } else {
            // Show loading placeholder for this chunk
            renderChunkPlaceholder(ctx, chunkId);
        }
    }
    
    // Show progress indicator
    let progress = waveformData.completed / waveformData.total;
    renderProgressIndicator(ctx, progress);
}
```

### Implementation Strategy

#### Phase 1: Chunked Generation Backend

1. **Modify `WaveformAnalyzer`:**
   - Add chunk size parameter (default: 2 seconds per chunk)
   - Implement callback-based chunk delivery
   - Maintain sample-accurate chunk boundaries

2. **Update `MultiBridge`:**
   - Replace monolithic waveform cache with progressive cache
   - Add `update_waveform_chunk()` Qt method for chunk updates
   - Emit chunk-specific signals to QML

#### Phase 2: Progressive UI Rendering

1. **Canvas Enhancements:**
   - Implement chunk-based rendering pipeline
   - Add visual progress indicators
   - Maintain smooth scrolling/zooming with partial data

2. **Loading State Improvements:**
   - Show percentage progress per file
   - Display estimated time remaining
   - Provide visual feedback for chunk completion

#### Phase 3: Optimizations

1. **Smart Chunk Prioritization:**
   - Generate visible chunks first based on current playback position
   - Load chunks near current position with higher priority

2. **Memory Management:**
   - Implement chunk LRU cache for large files
   - Stream chunks for files larger than available memory

### Benefits

1. **Improved User Experience:**
   - Immediate visual feedback as chunks become available
   - No more full blocking during waveform generation
   - Progressive loading with clear progress indication

2. **Better Performance:**
   - Reduced memory allocation spikes
   - Parallelizable chunk processing
   - Efficient handling of large audio files

3. **Enhanced Responsiveness:**
   - UI remains interactive during waveform generation
   - Real-time progress updates
   - Smooth playback can start before full waveform is ready

### Risks and Mitigations

1. **Complexity Increase:**
   - *Risk:* More complex state management
   - *Mitigation:* Careful chunk boundary management, comprehensive testing

2. **Memory Overhead:**
   - *Risk:* Additional chunk metadata storage
   - *Mitigation:* Efficient chunk compaction, configurable chunk sizes

3. **Rendering Performance:**
   - *Risk:* Frequent canvas repaints during chunk loading
   - *Mitigation:* Debounced repaint requests, efficient rendering pipeline

### Chunk Size Recommendations

- **Default chunk size:** 2 seconds of audio
- **Rationale:** Balance between UI responsiveness and processing overhead
- **Configurable:** Allow adjustment based on file size and system performance

This design transforms the blocking waveform generation into a smooth, progressive experience that provides immediate user feedback while maintaining full waveform fidelity.