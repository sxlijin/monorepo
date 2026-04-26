# Stem File Formats and Organization

## Overview

This document analyzes stem file formats and directory organization standards used across the audio industry. Understanding these formats is crucial for building a stem player that works with real-world audio workflows.

## Table of Contents
1. [What Are Stems?](#what-are-stems)
2. [Industry Standard Formats](#industry-standard-formats)
3. [AI Separation Tool Formats](#ai-separation-tool-formats)
4. [DJ Software Formats](#dj-software-formats)
5. [DAW Export Formats](#daw-export-formats)
6. [File Organization Patterns](#file-organization-patterns)
7. [Recommended Implementation Strategy](#recommended-implementation-strategy)

---

## What Are Stems?

**Stems** are individual audio tracks that, when played together, form a complete song. They typically represent different instrument groups or elements:

### Common Stem Types
- **Drums** - Kick, snare, hi-hats, percussion
- **Bass** - Bass guitar, synth bass, sub-bass
- **Vocals** - Lead vocals, backing vocals, harmonies  
- **Instruments/Other** - Guitars, keyboards, strings, effects

### Use Cases
- **Remixing** - Isolate elements for creative reworking
- **Karaoke** - Remove vocals while keeping instrumentation
- **Education** - Study individual instrument parts
- **Live Performance** - DJ mixing with isolated elements
- **Post-Production** - Film/video audio editing

---

## Industry Standard Formats

### 1. Native Instruments Stems Format (.stem)

**Overview**: The closest thing to an "official" stem standard, developed by Native Instruments.

**Technical Specs**:
- Container format based on MP4
- Contains up to 4 audio streams (typically: drums, bass, vocals, other)
- Metadata includes BPM, key, artwork
- Lossless or lossy compression options
- Designed for DJ software integration

**File Structure**:
```
song_name.stem
├── Stream 1: Drums (embedded)
├── Stream 2: Bass (embedded)  
├── Stream 3: Vocals (embedded)
├── Stream 4: Other (embedded)
└── Metadata (BPM, key, artwork)
```

**Adoption**: 
- Supported by Traktor, VirtualDJ, djay Pro
- Limited adoption due to proprietary nature
- Requires licensing from Native Instruments

**Pros**: 
- Single file format
- Industry backing
- Rich metadata support
- Efficient storage

**Cons**:
- Proprietary format
- Limited software support
- Licensing requirements
- Lossy compression options

### 2. Splice Stems Format

**Overview**: Online platform format, widely used in electronic music production.

**Technical Specs**:
- Individual WAV/AIFF files
- Standardized naming convention
- Metadata in separate JSON file
- Cloud-based distribution

**File Structure**:
```
artist_-_song_name_stems/
├── artist_-_song_name_drums.wav
├── artist_-_song_name_bass.wav
├── artist_-_song_name_vocals.wav
├── artist_-_song_name_other.wav
└── metadata.json
```

**Adoption**: Very high in electronic music community

---

## AI Separation Tool Formats

### 1. Demucs (Facebook/Meta)

**Overview**: State-of-the-art AI audio separation tool.

**Technical Specs**:
- Outputs WAV files (16-bit or 32-bit float)
- 4-stem separation: drums, bass, other, vocals
- Directory-based organization
- Multiple model variants (htdemucs, mdx, etc.)

**File Structure**:
```
separated/
└── htdemucs/
    └── song_name/
        ├── drums.wav
        ├── bass.wav
        ├── other.wav
        └── vocals.wav
```

**Pros**:
- Open source
- Excellent separation quality
- Standardized output format
- Active development

**Cons**:
- No embedded metadata
- Directory structure can be nested
- Multiple model naming conventions

### 2. Spleeter (Spotify)

**Overview**: Earlier AI separation tool, now mostly superseded.

**File Structure**:
```
audio_example/
├── vocals.wav
├── drums.wav
├── bass.wav
└── other.wav
```

**Status**: Legacy format, less common now

### 3. LALAL.AI Commercial Service

**File Structure**:
```
song_name_stems/
├── song_name_(Instrumental).wav
├── song_name_(Vocals).wav
├── song_name_(Drums).wav
├── song_name_(Bass).wav
└── song_name_(Piano).wav
```

**Features**: 
- Variable stem count (2-10 stems)
- Descriptive naming
- High-quality commercial processing

---

## DJ Software Formats

### 1. VirtualDJ Stems (.vdjstems)

**Overview**: VirtualDJ's proprietary stem format.

**Technical Specs**:
- Binary format with embedded audio streams
- Optimized for real-time DJ performance
- Includes beat grid and cue point data
- Variable compression options

**Adoption**: Limited to VirtualDJ ecosystem

### 2. Serato DJ Stems

**Overview**: Serato's approach to stem handling.

**Technical Specs**:
- Uses standard audio files with metadata tags
- Relies on file naming conventions
- Integrates with Serato's crate system

**File Structure**:
```
Artist - Song Name/
├── Artist - Song Name [Drums].wav
├── Artist - Song Name [Bass].wav
├── Artist - Song Name [Vocals].wav
└── Artist - Song Name [Melodic].wav
```

### 3. djay Pro AI Stems

**Overview**: Real-time AI separation within djay Pro.

**Technical Specs**:
- No file format (real-time processing)
- Integrates with Apple's Core ML
- Stems created on-demand from regular audio files

---

## DAW Export Formats

### 1. Ableton Live Stems Export

**File Structure**:
```
Song_Name_Stems/
├── Song_Name_Drums.wav
├── Song_Name_Bass.wav
├── Song_Name_Vocals.wav
├── Song_Name_Keys.wav
├── Song_Name_Guitar.wav
└── Song_Name_FX.wav
```

**Features**:
- Variable stem count
- Descriptive naming
- Includes tempo and time signature in metadata

### 2. Logic Pro Bounce to Stems

**File Structure**:
```
ProjectName/
├── Drums.wav
├── Bass.wav
├── Vocals.wav
├── Lead.wav
└── Pad.wav
```

**Features**:
- Uses track names as file names
- Maintains project tempo/sample rate
- Optional file format conversion

### 3. Pro Tools Stems Export

**File Structure**:
```
SessionName_Stems_YYMMDD/
├── 01_Kick.wav
├── 02_Snare.wav
├── 03_Bass.wav
├── 04_Vocals.wav
└── 05_Guitar.wav
```

**Features**:
- Numbered prefix for ordering
- Professional naming convention
- BWF metadata support

---

## File Organization Patterns

### Common Directory Structures

#### Pattern 1: Model-Based (AI Tools)
```
separated/
├── model_name/
│   ├── song1/
│   │   ├── drums.wav
│   │   ├── bass.wav
│   │   ├── other.wav
│   │   └── vocals.wav
│   └── song2/
│       ├── drums.wav
│       ├── bass.wav
│       ├── other.wav
│       └── vocals.wav
```

#### Pattern 2: Artist-Song Hierarchy
```
stems_library/
├── Artist_Name/
│   ├── Album_Name/
│   │   ├── 01_Song_Name/
│   │   │   ├── drums.wav
│   │   │   ├── bass.wav
│   │   │   ├── vocals.wav
│   │   │   └── other.wav
```

#### Pattern 3: Flat Collection
```
stems/
├── song1_drums.wav
├── song1_bass.wav
├── song1_vocals.wav
├── song1_other.wav
├── song2_drums.wav
├── song2_bass.wav
├── song2_vocals.wav
└── song2_other.wav
```

#### Pattern 4: Single-File Containers
```
stems_collection/
├── song1.stem (Native Instruments)
├── song2.vdjstems (VirtualDJ)
└── song3.stems (Custom format)
```

### Naming Conventions

#### Stem Type Identifiers
- **Standard**: drums, bass, vocals, other
- **Descriptive**: kick_snare, synth_bass, lead_vocal, instruments
- **Numbered**: stem1, stem2, stem3, stem4
- **Bracketed**: [Drums], [Bass], [Vocals], [Melodic]

#### File Naming Patterns
```
# Pattern 1: Prefix
Artist - Song Name_drums.wav
Artist - Song Name_bass.wav

# Pattern 2: Suffix  
drums_Artist - Song Name.wav
bass_Artist - Song Name.wav

# Pattern 3: Bracketed
Artist - Song Name [Drums].wav
Artist - Song Name [Bass].wav

# Pattern 4: Directory + Simple
Artist - Song Name/
├── drums.wav
├── bass.wav
```

---

## File Format Technical Details

### Audio Formats by Use Case

#### Professional/Studio Use
- **WAV** (BWF preferred): Uncompressed, metadata support
- **AIFF**: Mac-native uncompressed format
- **32-bit float WAV**: Maximum dynamic range

#### Distribution/Sharing
- **MP3**: Widely compatible, smaller files
- **AAC/M4A**: Better quality than MP3 at same bitrate
- **FLAC**: Lossless compression

#### Specialized Formats
- **.stem**: Native Instruments container
- **.vdjstems**: VirtualDJ proprietary
- **.stems**: Generic container proposals

### Metadata Standards

#### BWF (Broadcast Wave Format)
- Industry standard for professional audio
- Embedded metadata chunk
- Timestamp and originator information
- Compatible with most professional software

#### ID3 Tags
- Originally for MP3, now used in various formats
- Artist, title, album, track number
- Custom fields for stem type

#### Vorbis Comments
- Used in FLAC, OGG
- Flexible key-value metadata system
- Good for custom stem information

---

## Recommended Implementation Strategy

### Phase 1: Core Formats (Immediate Priority)

1. **Demucs Directory Structure** ✅ Already supported
   ```
   model_name/song_name/
   ├── drums.wav
   ├── bass.wav  
   ├── other.wav
   └── vocals.wav
   ```

2. **Simple Directory Structure**
   ```
   song_name/
   ├── drums.wav
   ├── bass.wav
   ├── vocals.wav
   └── other.wav (or instruments.wav)
   ```

3. **Flat File Structure with Naming Convention**
   ```
   song_name_drums.wav
   song_name_bass.wav
   song_name_vocals.wav
   song_name_other.wav
   ```

### Phase 2: Extended Format Support

1. **Native Instruments .stem Files**
   - High industry adoption
   - Single-file convenience
   - Rich metadata

2. **Splice-style Naming**
   ```
   artist_-_song_name_drums.wav
   artist_-_song_name_bass.wav
   ```

3. **Bracketed Naming (DJ Software)**
   ```
   Song Name [Drums].wav
   Song Name [Bass].wav
   ```

### Phase 3: Advanced Features

1. **VirtualDJ .vdjstems** (if reverse engineering is legal)
2. **Custom metadata extraction**
3. **Automatic stem type detection**
4. **Multi-format import/export**

### Implementation Notes

#### File Discovery Algorithm
```rust
// Pseudo-code for stem detection
fn detect_stem_format(path: &Path) -> Option<StemFormat> {
    if path.extension() == "stem" {
        return Some(StemFormat::NativeInstruments);
    }
    
    if path.is_dir() {
        let files = scan_directory(path);
        if has_standard_stem_names(&files) {
            return Some(StemFormat::DirectoryBased);
        }
    }
    
    // Check for flat naming patterns
    if detect_flat_naming_pattern(path.parent()?) {
        return Some(StemFormat::FlatNaming);
    }
    
    None
}
```

#### Stem Type Mapping
```rust
const STEM_ALIASES: &[(&str, StemType)] = &[
    ("drums", StemType::Drums),
    ("drum", StemType::Drums),
    ("kick", StemType::Drums),
    ("bass", StemType::Bass),
    ("vocals", StemType::Vocals),
    ("vocal", StemType::Vocals),
    ("voice", StemType::Vocals),
    ("other", StemType::Other),
    ("instruments", StemType::Other),
    ("melodic", StemType::Other),
    ("music", StemType::Other),
];
```

### Priority Matrix

| Format | Industry Adoption | Implementation Complexity | Priority |
|--------|------------------|---------------------------|----------|
| Demucs Directory | High (AI tools) | Low | 🟢 Phase 1 |
| Simple Directory | High (DAWs) | Low | 🟢 Phase 1 |
| Flat Naming | Medium | Low | 🟢 Phase 1 |
| .stem Files | Medium (DJ) | Medium | 🟡 Phase 2 |
| Splice Format | High (Electronic) | Low | 🟡 Phase 2 |
| .vdjstems | Low | High | 🔴 Phase 3 |

This analysis provides a roadmap for implementing broad stem format compatibility while prioritizing the most common and accessible formats first.