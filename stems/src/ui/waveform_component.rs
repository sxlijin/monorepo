use crate::analysis::RawWaveformData;
use crate::constants::StemType;
use crate::waveform_registry::WAVEFORM_REGISTRY;
use qmetaobject::*;
use std::sync::Arc;

/// Native waveform component for multi-player using QPainter
/// Replaces the Canvas-based implementation with hardware-accelerated rendering
#[derive(QObject, Default)]
pub struct WaveformComponent {
    base: qt_base_class!(trait QQuickPaintedItem),

    // Geometry properties
    component_width: qt_property!(f64; NOTIFY component_width_changed),
    component_height: qt_property!(f64; NOTIFY component_height_changed),

    // Waveform control properties
    stem_index: qt_property!(i32; NOTIFY stem_index_changed),
    current_position: qt_property!(f64; NOTIFY current_position_changed),
    duration: qt_property!(f64; NOTIFY duration_changed),
    zoom_level: qt_property!(f64; NOTIFY zoom_level_changed),
    is_playing: qt_property!(bool; NOTIFY is_playing_changed),
    current_volume: qt_property!(f64; NOTIFY current_volume_changed),

    // Visual properties
    waveform_color: qt_property!(QColor; NOTIFY waveform_color_changed),
    background_color: qt_property!(QColor; NOTIFY background_color_changed),
    beat_color: qt_property!(QColor; NOTIFY beat_color_changed),
    cursor_color: qt_property!(QColor; NOTIFY cursor_color_changed),

    // Methods
    set_position: qt_method!(fn(&mut self, position: f64)),
    set_stem_index: qt_method!(fn(&mut self, index: i32)),
    request_update: qt_method!(fn(&mut self)),

    // Signals
    component_width_changed: qt_signal!(),
    component_height_changed: qt_signal!(),
    stem_index_changed: qt_signal!(),
    current_position_changed: qt_signal!(),
    duration_changed: qt_signal!(),
    zoom_level_changed: qt_signal!(),
    is_playing_changed: qt_signal!(),
    current_volume_changed: qt_signal!(),
    waveform_color_changed: qt_signal!(),
    background_color_changed: qt_signal!(),
    beat_color_changed: qt_signal!(),
    cursor_color_changed: qt_signal!(),
}

impl WaveformComponent {
    pub fn new() -> Self {
        Self {
            base: Default::default(),
            component_width: 800.0,
            component_height: 200.0,
            stem_index: 0,
            current_position: 0.0,
            duration: 0.0,
            zoom_level: 10.0,
            is_playing: false,
            current_volume: 1.0,
            waveform_color: QColor::from_name("#4CAF50"),
            background_color: QColor::from_name("#0f0f0f"),
            beat_color: QColor::from_name("#666666"),
            cursor_color: QColor::from_name("#ffffff"),
            set_position: Default::default(),
            set_stem_index: Default::default(),
            request_update: Default::default(),
            component_width_changed: Default::default(),
            component_height_changed: Default::default(),
            stem_index_changed: Default::default(),
            current_position_changed: Default::default(),
            duration_changed: Default::default(),
            zoom_level_changed: Default::default(),
            is_playing_changed: Default::default(),
            current_volume_changed: Default::default(),
            waveform_color_changed: Default::default(),
            background_color_changed: Default::default(),
            beat_color_changed: Default::default(),
            cursor_color_changed: Default::default(),
        }
    }

    fn set_position(&mut self, position: f64) {
        if (self.current_position - position).abs() > f64::EPSILON {
            self.current_position = position.clamp(0.0, self.duration);
            self.current_position_changed();
            (self as &dyn QQuickItem).update();
        }
    }

    fn set_stem_index(&mut self, index: i32) {
        if self.stem_index != index {
            self.stem_index = index;
            self.stem_index_changed();
            (self as &dyn QQuickItem).update();
        }
    }

    fn request_update(&mut self) {
        (self as &dyn QQuickItem).update();
    }
}

impl QQuickItem for WaveformComponent {
    fn geometry_changed(&mut self, new: QRectF, _old: QRectF) {
        self.component_width = new.width;
        self.component_height = new.height;
        self.component_width_changed();
        self.component_height_changed();
        (self as &dyn QQuickItem).update();
    }
}

impl QQuickPaintedItem for WaveformComponent {
    fn paint(&mut self, painter: &mut QPainter) {
        self.paint_waveform(painter);
    }
}

impl WaveformComponent {
    fn paint_waveform(&mut self, painter: &mut QPainter) {
        // Clear background
        let bg_rect = QRectF {
            x: 0.0,
            y: 0.0,
            width: self.component_width,
            height: self.component_height,
        };
        painter.fill_rect(bg_rect, QBrush::from_color(self.background_color));

        // Get raw waveform data from registry
        let waveform_data = {
            let registry = WAVEFORM_REGISTRY.lock().unwrap();
            let stem = StemType::from_index(self.stem_index.max(0) as usize);
            stem.and_then(|stem| registry.get_waveform_data(stem))
        };

        if let Some(data) = waveform_data {
            self.render_waveform_from_mono(painter, &data);
            if !data.beat_timestamps.is_empty() {
                self.render_beat_markers(painter, &data.beat_timestamps);
            }
        }

        self.render_playback_cursor(painter);
    }

    fn render_waveform_from_mono(&self, painter: &mut QPainter, data: &Arc<RawWaveformData>) {
        if data.mono.is_empty() || self.duration <= 0.0 {
            return;
        }

        painter.set_brush(QBrush::from_color(self.waveform_color));

        let center_y = self.component_height / 2.0;
        let max_amplitude = self.component_height * 0.45; // Leave space for center line

        // 50%-opacity "ghost" of the waveform color, used to overlay the
        // original (un-scaled) bar against the volume-scaled bar.
        let (wr, wg, wb, _wa) = self.waveform_color.get_rgba_f();
        let ghost_color = QColor::from_rgba_f(wr, wg, wb, 0.5);
        let volume = self.current_volume;
        let volume_is_unity = (volume - 1.0).abs() < 1e-9;

        // Visible time window with fixed cursor at 20%
        // Important: do NOT clamp start to 0.0. Allow negative start so the
        // viewport is initially offset such that the fixed playhead (20%) aligns
        // with t=0. This makes the waveform scroll immediately from t=0.
        let visible_duration = self.zoom_level.max(0.001);
        // Quantize viewport start to the pixel grid for stability
        let width_px = self.component_width.floor().max(1.0);
        let seconds_per_pixel = visible_duration / width_px;
        let visible_start_time_raw = self.current_position - (visible_duration * 0.2);
        let visible_start_time =
            (visible_start_time_raw / seconds_per_pixel).floor() * seconds_per_pixel;
        let sr = data.sample_rate as f64;
        let n = data.mono.len();
        let spp = (visible_duration * sr) / self.component_width.max(1.0);
        let use_lod = spp >= 128.0 && !data.lod.is_empty();
        let (lod_idx, bin_seconds_opt) = if use_lod {
            // Pick once per frame
            let mut best_idx = 0usize;
            let mut best_err = f64::INFINITY;
            for (i, lvl) in data.lod.iter().enumerate() {
                let err = ((lvl.samples_per_bin as f64) - spp).abs();
                if err < best_err {
                    best_err = err;
                    best_idx = i;
                }
            }
            let bin_seconds = (data.lod[best_idx].samples_per_bin as f64) / sr;
            (best_idx, Some(bin_seconds))
        } else {
            (0usize, None)
        };

        // Render bars across width (integer pixel columns)
        for x in 0..(width_px as i32) {
            let t0 = visible_start_time + (x as f64) * seconds_per_pixel;
            let t1 = t0 + seconds_per_pixel;

            // Skip columns entirely before t=0
            if t1 <= 0.0 {
                continue;
            }

            // Clamp sampling window to [0, duration]
            let ct0 = t0.max(0.0);
            let ct1 = t1.min(self.duration);
            if ct1 <= ct0 {
                continue;
            }

            // Compute symmetric envelope amplitude using either LOD or raw samples
            let amp_unit = if use_lod {
                let lvl = &data.lod[lod_idx];
                let bin_seconds = bin_seconds_opt.unwrap_or((lvl.samples_per_bin as f64) / sr);
                let b0 = (ct0 / bin_seconds).floor() as isize;
                let b1 = (ct1 / bin_seconds).ceil() as isize;
                let b1 = b1.max(b0 + 1);
                let len = lvl.len() as isize;
                let bb0 = b0.clamp(0, len - 1) as usize;
                let bb1 = b1.clamp(0, len) as usize;
                if bb0 >= bb1 {
                    0.0
                } else {
                    let mut amax: i32 = 0;
                    let mut j = bb0;
                    while j < bb1 {
                        let mn = unsafe { *lvl.min.get_unchecked(j) } as i32;
                        let mx = unsafe { *lvl.max.get_unchecked(j) } as i32;
                        let v = if (-mn) > mx { -mn } else { mx };
                        if v > amax {
                            amax = v;
                        }
                        j += 1;
                    }
                    (amax as f64) / 32768.0
                }
            } else {
                // Raw per-pixel reduction over exact frame range
                let mut f0 = (ct0 * sr).floor() as isize;
                let mut f1 = (ct1 * sr).ceil() as isize;
                if f1 <= f0 {
                    f1 = f0 + 1;
                }
                if f1 as usize > n {
                    f1 = n as isize;
                }
                if f0 as usize >= n {
                    continue;
                }
                let span = (f1 - f0) as usize;
                let stride = if span > 4096 { (span / 1024).max(2) } else { 1 };
                let mut abs_max: i32 = 0;
                let mut i = f0 as usize;
                while i < f1 as usize {
                    let v = unsafe { *data.mono.get_unchecked(i) } as i32;
                    let av = v.abs();
                    if av > abs_max {
                        abs_max = av;
                    }
                    i = i.saturating_add(stride);
                }
                (abs_max as f64) / 32768.0
            };

            // Scale symmetric envelope to pixels and draw centered bars.
            //
            // Volume is clamped to [0, 1]. At unity, draw the scaled bar
            // alone. Below unity, draw the un-scaled "orig" bar behind at
            // 50% alpha and the scaled bar in front at full color, so the
            // taller orig sticks out above and below the scaled bar.
            let scaled_amp_px = amp_unit * volume * max_amplitude;
            let scaled_rect = QRectF {
                x: x as f64,
                y: center_y - scaled_amp_px,
                width: 1.0,
                height: (scaled_amp_px * 2.0).max(1.0),
            };

            if volume_is_unity {
                painter.fill_rect(scaled_rect, QBrush::from_color(self.waveform_color));
                continue;
            }

            let orig_amp_px = amp_unit * max_amplitude;
            let orig_rect = QRectF {
                x: x as f64,
                y: center_y - orig_amp_px,
                width: 1.0,
                height: (orig_amp_px * 2.0).max(1.0),
            };

            painter.fill_rect(orig_rect, QBrush::from_color(ghost_color));
            painter.fill_rect(scaled_rect, QBrush::from_color(self.waveform_color));
        }

        // Draw center line
        let center_pen = QPen::from_color(QColor::from_rgba_f(0.4, 0.4, 0.4, 0.5));
        painter.set_pen(center_pen);
        let center_line = QLineF {
            pt1: QPointF {
                x: 0.0,
                y: center_y,
            },
            pt2: QPointF {
                x: self.component_width,
                y: center_y,
            },
        };
        painter.draw_line(center_line);
    }

    fn render_beat_markers(&self, painter: &mut QPainter, beats: &[f64]) {
        // Create more visible beat marker pen
        let mut beat_pen = QPen::from_color(self.beat_color);
        beat_pen.set_width_f(1.5); // Make beat lines slightly thicker
        painter.set_pen(beat_pen);

        // Use same viewport as waveform rendering (quantized to pixel grid)
        let visible_duration = self.zoom_level.max(0.001);
        let width_px = self.component_width.floor().max(1.0);
        let seconds_per_pixel = visible_duration / width_px;
        let visible_start_time_raw = self.current_position - (visible_duration * 0.2);
        let visible_start_time =
            (visible_start_time_raw / seconds_per_pixel).floor() * seconds_per_pixel;

        for &beat_time in beats {
            if beat_time >= visible_start_time && beat_time <= visible_start_time + visible_duration
            {
                let time_ratio = (beat_time - visible_start_time) / visible_duration;
                let x = time_ratio * width_px;

                let beat_line = QLineF {
                    pt1: QPointF { x, y: 0.0 },
                    pt2: QPointF {
                        x,
                        y: self.component_height,
                    },
                };
                painter.draw_line(beat_line);
            }
        }
    }

    fn render_playback_cursor(&self, painter: &mut QPainter) {
        // Create a more visible cursor pen
        let mut cursor_pen = QPen::from_color(self.cursor_color);
        cursor_pen.set_width_f(2.0); // Make cursor thicker
        painter.set_pen(cursor_pen);

        // Fixed cursor position at 20% from left (static reference point)
        let cursor_x = self.component_width * 0.2;

        let cursor_line = QLineF {
            pt1: QPointF {
                x: cursor_x,
                y: 0.0,
            },
            pt2: QPointF {
                x: cursor_x,
                y: self.component_height,
            },
        };
        painter.draw_line(cursor_line);
    }
}
