use anyhow::{Context, Result};
use audio_visualizer::waveform::png_file::waveform_static_png_visualize;
use audio_visualizer::{ChannelInterleavement, Channels};
use clap::Parser;
use std::path::PathBuf;
use std::time::Instant;

use stems::audio::wav_loader::{MappedAudioFile, WavLoader};

#[derive(Parser)]
#[command(name = "waveform-analyzer")]
#[command(about = "Generate and analyze waveforms from WAV files")]
struct Args {
    /// Input WAV file(s)
    #[arg(required = true)]
    files: Vec<PathBuf>,

    /// Output directory for generated files
    #[arg(short, long, default_value = "./waveform_output")]
    output: PathBuf,

    /// Samples per pixel for waveform generation (mutually exclusive with time-based options)
    #[arg(short, long, default_value = "100")]
    samples_per_pixel: usize,

    /// Time resolution in milliseconds per peak (alternative to samples_per_pixel)
    #[arg(long, conflicts_with = "samples_per_pixel")]
    time_per_peak_ms: Option<f64>,

    /// Total number of peaks to generate (alternative to samples_per_pixel)
    #[arg(long, conflicts_with = "samples_per_pixel")]
    total_peaks: Option<usize>,

    /// Pixels per second for waveform display (alternative to samples_per_pixel)
    #[arg(long, conflicts_with = "samples_per_pixel")]
    pixels_per_second: Option<f64>,

    /// Generate reference PNG using audio-visualizer
    #[arg(long)]
    reference_png: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Clone)]
pub struct WaveformPeak {
    pub min: f32,
    pub max: f32,
    pub rms: f32,
}

/// Calculate samples per pixel based on different resolution options
pub fn calculate_samples_per_pixel(
    audio_file: &MappedAudioFile,
    args: &Args,
) -> Result<(usize, f64)> {
    let sample_rate = audio_file.spec.sample_rate as f64;
    let channels = audio_file.spec.channels as usize;
    let total_samples = audio_file.sample_count;
    let duration_seconds = total_samples as f64 / channels as f64 / sample_rate;

    let (samples_per_pixel, time_per_peak_ms) = if let Some(time_ms) = args.time_per_peak_ms {
        let time_per_peak_seconds = time_ms / 1000.0;
        let samples_per_peak = (time_per_peak_seconds * sample_rate) as usize;
        (samples_per_peak, time_ms)
    } else if let Some(total_peaks) = args.total_peaks {
        let samples_per_peak = (total_samples / channels) / total_peaks;
        let time_per_peak_ms = (samples_per_peak as f64 / sample_rate) * 1000.0;
        (samples_per_peak, time_per_peak_ms)
    } else if let Some(pps) = args.pixels_per_second {
        let peaks_per_second = pps;
        let time_per_peak_seconds = 1.0 / peaks_per_second;
        let samples_per_peak = (time_per_peak_seconds * sample_rate) as usize;
        let time_per_peak_ms = time_per_peak_seconds * 1000.0;
        (samples_per_peak, time_per_peak_ms)
    } else {
        // Default: use samples_per_pixel
        let time_per_peak_ms = (args.samples_per_pixel as f64 / sample_rate) * 1000.0;
        (args.samples_per_pixel, time_per_peak_ms)
    };

    if samples_per_pixel == 0 {
        return Err(anyhow::anyhow!(
            "Calculated samples_per_pixel is 0 - resolution too high for audio file"
        ));
    }

    Ok((samples_per_pixel, time_per_peak_ms))
}

/// Generate waveform peaks using our custom algorithm (inspired by audio-visualizer)
pub fn generate_waveform_peaks(
    audio_file: &MappedAudioFile,
    samples_per_pixel: usize,
) -> Result<Vec<WaveformPeak>> {
    let mut peaks = Vec::new();
    let total_samples = audio_file.sample_count;
    let channels = audio_file.spec.channels as usize;

    // Handle both mono and stereo files
    for i in (0..total_samples).step_by(samples_per_pixel * channels) {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        let mut sum_squares = 0.0f32;
        let mut sample_count = 0;

        // Process samples_per_pixel worth of data
        for j in 0..samples_per_pixel {
            let sample_idx = i + (j * channels);
            if sample_idx >= total_samples {
                break;
            }

            // For stereo, we'll mix both channels for the waveform
            let sample = if channels == 2 {
                // Average L and R channels
                let left = audio_file.get_sample(sample_idx);
                let right = audio_file.get_sample(sample_idx + 1);
                (left + right) / 2.0
            } else {
                audio_file.get_sample(sample_idx)
            };

            min = min.min(sample);
            max = max.max(sample);
            sum_squares += sample * sample;
            sample_count += 1;
        }

        let rms = if sample_count > 0 {
            (sum_squares / sample_count as f32).sqrt()
        } else {
            0.0
        };

        peaks.push(WaveformPeak { min, max, rms });
    }

    Ok(peaks)
}

/// Generate colored PNG waveform from spectral peaks
fn generate_colored_waveform_png(
    peaks: &[WaveformPeak],
    filename: &str,
    output_dir: &str,
) -> Result<()> {
    const IMAGE_HEIGHT: usize = 400;
    const PIXELS_PER_PEAK: usize = 4; // Width per peak for ultra high resolution

    if peaks.is_empty() {
        return Ok(());
    }

    // Calculate image width based on number of peaks
    let min_width = 1500; // Minimum width for readability
    let desired_width = peaks.len() * PIXELS_PER_PEAK;
    let actual_width = desired_width.max(min_width);

    println!("  Generating {}x{} PNG", actual_width, IMAGE_HEIGHT);

    // RGB image data (default to white background)
    let mut image = vec![vec![(255, 255, 255); actual_width]; IMAGE_HEIGHT];

    let height_scale = IMAGE_HEIGHT as f64 / 4.0; // Scale for -1.0 to 1.0 range
    let center_y = IMAGE_HEIGHT / 2;

    for (peak_index, peak) in peaks.iter().enumerate() {
        let x = (peak_index as f64 * actual_width as f64 / peaks.len() as f64) as usize;
        if x >= actual_width {
            break;
        }

        // Use blue color for all waveforms
        let color = (0, 100, 200);

        // Draw waveform using min/max
        let min_y = (center_y as f64 - peak.min as f64 * height_scale) as usize;
        let max_y = (center_y as f64 - peak.max as f64 * height_scale) as usize;

        // Ensure we don't go out of bounds
        let min_y = min_y.min(IMAGE_HEIGHT - 1);
        let max_y = max_y.min(IMAGE_HEIGHT - 1);

        // Draw vertical line from min to max
        let start_y = min_y.min(max_y);
        let end_y = min_y.max(max_y);

        for y in start_y..=end_y {
            if x < actual_width && y < IMAGE_HEIGHT {
                image[y][x] = color;
            }

            // Draw a few pixels wide for better visibility
            if x + 1 < actual_width {
                image[y][x + 1] = color;
            }
        }
    }

    // Write PNG file
    std::fs::create_dir_all(output_dir)?;
    let path = std::path::Path::new(output_dir).join(filename);
    write_png_file_rgb_tuples(&path, &image);

    Ok(())
}

/// Write RGB tuples to PNG file (simple implementation)
fn write_png_file_rgb_tuples(path: &std::path::Path, image: &[Vec<(u8, u8, u8)>]) {
    use std::fs::File;
    use std::io::BufWriter;

    let file = File::create(path).expect("Failed to create PNG file");
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, image[0].len() as u32, image.len() as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header().expect("Failed to write PNG header");

    // Convert to flat RGB array
    let mut rgb_data = Vec::new();
    for row in image {
        for &(r, g, b) in row {
            rgb_data.push(r);
            rgb_data.push(g);
            rgb_data.push(b);
        }
    }

    writer
        .write_image_data(&rgb_data)
        .expect("Failed to write PNG data");
}

/// Convert our f32 samples to i16 for audio-visualizer compatibility
fn convert_f32_to_i16_samples(audio_file: &MappedAudioFile) -> Vec<i16> {
    let mut samples = Vec::new();

    for i in 0..audio_file.sample_count {
        let sample_f32 = audio_file.get_sample(i);
        // Convert f32 (-1.0 to 1.0) to i16 (-32768 to 32767)
        let sample_i16 = (sample_f32.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        samples.push(sample_i16);
    }

    samples
}

fn analyze_file(file_path: &PathBuf, args: &Args) -> Result<()> {
    println!("Analyzing: {}", file_path.display());

    // Load the WAV file using our existing system
    let audio_file = WavLoader::load_file_mapped(file_path.to_str().unwrap())
        .context("Failed to load WAV file")?;

    if args.verbose {
        println!(
            "  Sample rate: {}Hz, Channels: {}, Samples: {}",
            audio_file.spec.sample_rate, audio_file.spec.channels, audio_file.sample_count
        );
    }

    let duration_seconds = audio_file.sample_count as f64
        / audio_file.spec.channels as f64
        / audio_file.spec.sample_rate as f64;

    // Calculate resolution based on arguments
    let (samples_per_pixel, time_per_peak_ms) = calculate_samples_per_pixel(&audio_file, args)
        .context("Failed to calculate waveform resolution")?;

    if args.verbose {
        println!(
            "  Resolution: {} samples/peak ({:.2}ms per peak)",
            samples_per_pixel, time_per_peak_ms
        );
    }

    // Generate reference PNG using audio-visualizer if requested
    if args.reference_png {
        let i16_samples = convert_f32_to_i16_samples(&audio_file);
        let channels = if audio_file.spec.channels == 2 {
            Channels::Stereo(ChannelInterleavement::LRLR)
        } else {
            Channels::Mono
        };

        let output_dir = args.output.to_str().unwrap();
        let filename = format!(
            "reference_{}.png",
            file_path.file_stem().unwrap().to_str().unwrap()
        );

        if args.verbose {
            println!("  Generating reference PNG: {}/{}", output_dir, filename);
        }

        waveform_static_png_visualize(&i16_samples, channels, output_dir, &filename);
    }

    // Generate our custom waveform data
    let start_time = Instant::now();
    let peaks = generate_waveform_peaks(&audio_file, samples_per_pixel)
        .context("Failed to generate waveform peaks")?;
    let generation_time = start_time.elapsed();

    // Always generate colored PNG with unique naming
    let parent_name = file_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let file_stem = file_path.file_stem().unwrap().to_str().unwrap();
    let colored_filename = format!("colored_{}_{}.png", parent_name, file_stem);

    if args.verbose {
        println!(
            "  Generating colored PNG: {}/{}",
            args.output.display(),
            colored_filename
        );
    }

    generate_colored_waveform_png(&peaks, &colored_filename, args.output.to_str().unwrap())
        .context("Failed to generate colored waveform PNG")?;

    if args.verbose {
        println!(
            "  Generated {} peaks in {}ms ({:.2} peaks/sec)",
            peaks.len(),
            generation_time.as_millis(),
            peaks.len() as f64 / (generation_time.as_secs_f64())
        );
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Validation: ensure at least one resolution method is specified
    let resolution_methods = [
        args.time_per_peak_ms.is_some(),
        args.total_peaks.is_some(),
        args.pixels_per_second.is_some(),
    ]
    .iter()
    .filter(|&&x| x)
    .count();

    if resolution_methods > 1 {
        return Err(anyhow::anyhow!(
            "Only one time-based resolution method can be specified at a time"
        ));
    }

    // Create output directory
    std::fs::create_dir_all(&args.output).context("Failed to create output directory")?;

    println!("Waveform Analyzer - Phase 3A");
    println!("Output directory: {}", args.output.display());

    // Display resolution settings
    if let Some(time_ms) = args.time_per_peak_ms {
        println!("Resolution: {:.2}ms per peak", time_ms);
    } else if let Some(total_peaks) = args.total_peaks {
        println!("Resolution: {} total peaks", total_peaks);
    } else if let Some(pps) = args.pixels_per_second {
        println!("Resolution: {:.2} pixels per second", pps);
    } else {
        println!("Resolution: {} samples per pixel", args.samples_per_pixel);
    }

    println!("Colored PNG generation: ENABLED");
    println!();

    // Process each file
    for file_path in &args.files {
        match analyze_file(file_path, &args) {
            Ok(_) => {
                // File processed successfully
            }
            Err(e) => {
                eprintln!("Error processing {}: {}", file_path.display(), e);
            }
        }
    }

    println!("Analysis complete!");

    Ok(())
}
