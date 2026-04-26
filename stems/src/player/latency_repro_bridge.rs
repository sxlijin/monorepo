/// Minimal bridge for UI latency reproduction testing
///
/// This bridge simulates playback state updates without actual audio playback
/// to isolate canvas rendering performance issues.
use qmetaobject::*;
use std::time::{Duration, Instant};

#[derive(QObject, Default)]
pub struct LatencyReproBridge {
    base: qt_base_class!(trait QObject),

    // Playback simulation properties
    pub is_playing: qt_property!(bool; NOTIFY playback_state_changed),
    pub current_position: qt_property!(f64; NOTIFY playback_state_changed),
    pub duration: qt_property!(f64; NOTIFY playback_state_changed),
    pub playback_speed: qt_property!(f64; NOTIFY playback_speed_changed),

    // Waveform properties
    pub waveform_complexity: qt_property!(i32; NOTIFY waveform_complexity_changed),
    pub zoom_level: qt_property!(f64; NOTIFY zoom_level_changed),

    // Performance monitoring
    pub target_fps: qt_property!(i32; NOTIFY target_fps_changed),
    pub actual_fps: qt_property!(f64; NOTIFY actual_fps_changed),
    pub frame_time_ms: qt_property!(f64; NOTIFY frame_time_ms_changed),

    // Methods for QML
    toggle_playback: qt_method!(fn(&mut self)),
    seek: qt_method!(fn(&mut self, position: f64)),
    set_playback_speed: qt_method!(fn(&mut self, speed: f64)),
    set_waveform_complexity: qt_method!(fn(&mut self, complexity: i32)),
    set_zoom_level: qt_method!(fn(&mut self, zoom: f64)),
    set_target_fps: qt_method!(fn(&mut self, fps: i32)),
    reset_to_start: qt_method!(fn(&mut self)),
    update_state: qt_method!(fn(&mut self)),
    set_canvas_dimensions: qt_method!(fn(&mut self, width: i32, height: i32)),
    get_pixel_data: qt_method!(fn(&self) -> QVariantList),

    // Signals
    playback_state_changed: qt_signal!(),
    playback_speed_changed: qt_signal!(),
    waveform_complexity_changed: qt_signal!(),
    zoom_level_changed: qt_signal!(),
    target_fps_changed: qt_signal!(),
    actual_fps_changed: qt_signal!(),
    frame_time_ms_changed: qt_signal!(),
    waveform_data_changed: qt_signal!(),
    pixel_data_changed: qt_signal!(),

    // Internal state
    last_update: Option<Instant>,
    frame_count: u64,
    frame_time_sum: Duration,
    canvas_width: i32,
    canvas_height: i32,
}

impl LatencyReproBridge {
    pub fn new() -> Self {
        let mut bridge = Self {
            base: Default::default(),
            is_playing: false,
            current_position: 0.0,
            duration: 30.0, // 30 second test duration
            playback_speed: 1.0,
            waveform_complexity: 1000, // Number of peaks
            zoom_level: 1.0,
            target_fps: 500, // Default 500 FPS target
            actual_fps: 0.0,
            frame_time_ms: 0.0,
            toggle_playback: Default::default(),
            seek: Default::default(),
            set_playback_speed: Default::default(),
            set_waveform_complexity: Default::default(),
            set_zoom_level: Default::default(),
            set_target_fps: Default::default(),
            reset_to_start: Default::default(),
            update_state: Default::default(),
            set_canvas_dimensions: Default::default(),
            get_pixel_data: Default::default(),
            playback_state_changed: Default::default(),
            playback_speed_changed: Default::default(),
            waveform_complexity_changed: Default::default(),
            zoom_level_changed: Default::default(),
            target_fps_changed: Default::default(),
            actual_fps_changed: Default::default(),
            frame_time_ms_changed: Default::default(),
            waveform_data_changed: Default::default(),
            pixel_data_changed: Default::default(),
            last_update: None,
            frame_count: 0,
            frame_time_sum: Duration::ZERO,
            canvas_width: 800,
            canvas_height: 200,
        };

        bridge
    }

    /// Toggle playback state
    fn toggle_playback(&mut self) {
        self.is_playing = !self.is_playing;
        self.playback_state_changed();
        tracing::info!(
            "Playback toggled: {}",
            if self.is_playing { "playing" } else { "paused" }
        );
    }

    /// Seek to specific position
    fn seek(&mut self, position: f64) {
        self.current_position = position.clamp(0.0, self.duration);
        self.playback_state_changed();
        self.pixel_data_changed();
        tracing::debug!("Seeked to: {:.2}s", self.current_position);
    }

    /// Set playback speed multiplier
    fn set_playback_speed(&mut self, speed: f64) {
        self.playback_speed = speed.clamp(0.1, 5.0); // 0.1x to 5x speed
        self.playback_speed_changed();
        tracing::info!("Playback speed set to: {:.1}x", self.playback_speed);
    }

    /// Set waveform complexity (affects detail level)
    fn set_waveform_complexity(&mut self, complexity: i32) {
        self.waveform_complexity = complexity.clamp(100, 10000);
        self.waveform_complexity_changed();
        self.pixel_data_changed();
        tracing::info!(
            "Waveform complexity set to: {} peaks",
            self.waveform_complexity
        );
    }

    /// Set zoom level
    fn set_zoom_level(&mut self, zoom: f64) {
        self.zoom_level = zoom.clamp(0.1, 10.0);
        self.zoom_level_changed();
        self.pixel_data_changed();
        tracing::debug!("Zoom level set to: {:.1}x", self.zoom_level);
    }

    /// Set target FPS for performance testing
    fn set_target_fps(&mut self, fps: i32) {
        self.target_fps = fps.clamp(1, 500);
        self.target_fps_changed();
        tracing::info!("Target FPS set to: {}", self.target_fps);
    }

    /// Reset playback to start
    fn reset_to_start(&mut self) {
        self.current_position = 0.0;
        self.is_playing = false;
        self.actual_fps = 0.0;
        self.frame_time_ms = 0.0;
        self.frame_count = 0;
        self.frame_time_sum = Duration::ZERO;
        self.last_update = None;
        self.playback_state_changed();
        self.actual_fps_changed();
        self.frame_time_ms_changed();
        tracing::info!("Reset to start");
    }

    /// Update simulation state (called by QML timer)
    fn update_state(&mut self) {
        let now = Instant::now();

        // Track frame timing for performance monitoring
        if let Some(last_update) = self.last_update {
            let frame_time = now.duration_since(last_update);
            self.frame_time_sum += frame_time;
            self.frame_count += 1;

            // Update metrics every 30 frames for smooth display
            if self.frame_count % 30 == 0 {
                let avg_frame_time = self.frame_time_sum / 30;
                self.frame_time_ms = avg_frame_time.as_secs_f64() * 1000.0;
                self.actual_fps = 1000.0 / self.frame_time_ms;

                self.actual_fps_changed();
                self.frame_time_ms_changed();

                // Reset for next measurement period
                self.frame_time_sum = Duration::ZERO;
            }
        }

        self.last_update = Some(now);

        // Update playback position if playing
        if self.is_playing {
            if let Some(_last_update) = self.last_update {
                // Calculate time delta since last update (estimate from target FPS)
                let target_interval_ms = 1000.0 / self.target_fps as f64;
                let delta_seconds = (target_interval_ms / 1000.0) * self.playback_speed;

                self.current_position += delta_seconds;

                // Loop back to start if we exceed duration
                if self.current_position >= self.duration {
                    self.current_position = 0.0;
                }

                self.playback_state_changed();
                self.pixel_data_changed();
            }
        }
    }

    /// Set canvas dimensions from QML
    fn set_canvas_dimensions(&mut self, width: i32, height: i32) {
        self.canvas_width = width.max(1);
        self.canvas_height = height.max(1);
        self.pixel_data_changed();
        tracing::debug!(
            "Canvas dimensions set to: {}x{}",
            self.canvas_width,
            self.canvas_height
        );
    }

    /// Generate pixel data for the current viewport - hardcoded waveform patterns
    fn get_pixel_data(&self) -> QVariantList {
        let mut pixel_data = QVariantList::default();

        let width = self.canvas_width as usize;
        let max_amplitude = self.canvas_height as f32 * 0.8; // Use more of the height

        // Calculate viewport based on current position and zoom
        let viewport_duration = 5.0 / self.zoom_level; // Base 5 seconds of audio
        let start_time = (self.current_position - viewport_duration / 2.0).max(0.0);
        let end_time = (start_time + viewport_duration).min(self.duration);

        // Generate hardcoded waveform pattern for each pixel column
        for x in 0..width {
            let time_progress = x as f64 / width as f64;
            let sample_time = start_time + time_progress * (end_time - start_time);

            // Create interesting hardcoded waveform patterns
            let time_factor = sample_time * 2.0 * std::f64::consts::PI;

            // Combine multiple sine waves for interesting patterns
            let low_freq = (time_factor * 0.5).sin() * 0.6; // Low frequency component
            let mid_freq = (time_factor * 2.0).sin() * 0.3; // Mid frequency component
            let high_freq = (time_factor * 8.0).sin() * 0.1; // High frequency detail

            // Add some rhythmic elements
            let beat_pattern = if (sample_time * 2.0) % 1.0 < 0.1 {
                0.4
            } else {
                0.0
            };

            // Combine all components
            let amplitude = (low_freq + mid_freq + high_freq + beat_pattern).clamp(-1.0, 1.0);

            // Add some variation based on complexity setting
            let complexity_factor = self.waveform_complexity as f64 / 1000.0;
            let noise = (time_factor * 20.0 * complexity_factor).sin() * 0.05;

            let final_amplitude = (amplitude + noise).clamp(-1.0, 1.0);

            // Convert to pixel height (always positive)
            let waveform_height = (final_amplitude.abs() * max_amplitude as f64).max(1.0);

            pixel_data.push(QVariant::from(waveform_height));
        }

        pixel_data
    }
}
