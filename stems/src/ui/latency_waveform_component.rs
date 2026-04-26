use qmetaobject::*;

/// High-performance waveform rendering component using QPainter
/// Designed for the UI latency reproduction test to measure QPainter vs Canvas performance
#[derive(QObject, Default)]
pub struct LatencyWaveformComponent {
    base: qt_base_class!(trait QQuickPaintedItem),

    // Component dimensions - use different names to avoid conflicts
    component_width: qt_property!(f64; NOTIFY component_width_changed),
    component_height: qt_property!(f64; NOTIFY component_height_changed),

    // Waveform data properties
    current_position: qt_property!(f64; NOTIFY current_position_changed),
    duration: qt_property!(f64; NOTIFY duration_changed),
    zoom_level: qt_property!(f64; NOTIFY zoom_level_changed),
    is_playing: qt_property!(bool; NOTIFY is_playing_changed),

    // Visual properties
    background_color: qt_property!(QColor; NOTIFY background_color_changed),
    waveform_color: qt_property!(QColor; NOTIFY waveform_color_changed),
    center_line_color: qt_property!(QColor; NOTIFY center_line_color_changed),
    cursor_color: qt_property!(QColor; NOTIFY cursor_color_changed),

    // Performance properties
    waveform_complexity: qt_property!(i32; NOTIFY waveform_complexity_changed),

    // Marquee animation properties
    marquee_enabled: qt_property!(bool; NOTIFY marquee_enabled_changed),
    marquee_speed: qt_property!(f64; NOTIFY marquee_speed_changed),
    marquee_offset: qt_property!(f64; NOTIFY marquee_offset_changed),

    // Methods exposed to QML
    set_position: qt_method!(fn(&mut self, position: f64)),
    set_duration: qt_method!(fn(&mut self, duration: f64)),
    set_zoom: qt_method!(fn(&mut self, zoom: f64)),
    set_complexity: qt_method!(fn(&mut self, complexity: i32)),
    set_marquee_enabled: qt_method!(fn(&mut self, enabled: bool)),
    set_marquee_speed: qt_method!(fn(&mut self, speed: f64)),
    set_marquee_offset: qt_method!(fn(&mut self, offset: f64)),
    request_update: qt_method!(fn(&mut self)),

    // Signals
    component_width_changed: qt_signal!(),
    component_height_changed: qt_signal!(),
    current_position_changed: qt_signal!(),
    duration_changed: qt_signal!(),
    zoom_level_changed: qt_signal!(),
    is_playing_changed: qt_signal!(),
    background_color_changed: qt_signal!(),
    waveform_color_changed: qt_signal!(),
    center_line_color_changed: qt_signal!(),
    cursor_color_changed: qt_signal!(),
    waveform_complexity_changed: qt_signal!(),
    marquee_enabled_changed: qt_signal!(),
    marquee_speed_changed: qt_signal!(),
    marquee_offset_changed: qt_signal!(),
}

impl LatencyWaveformComponent {
    pub fn new() -> Self {
        Self {
            base: Default::default(),
            component_width: 800.0,
            component_height: 200.0,
            current_position: 0.0,
            duration: 30.0,
            zoom_level: 1.0,
            is_playing: false,
            background_color: QColor::from_name("black"),
            waveform_color: QColor::from_name("#4CAF50"),
            center_line_color: QColor::from_name("#666666"),
            cursor_color: QColor::from_name("#FF0000"),
            waveform_complexity: 1000,
            marquee_enabled: true,
            marquee_speed: 50.0,
            marquee_offset: 0.0,
            set_position: Default::default(),
            set_duration: Default::default(),
            set_zoom: Default::default(),
            set_complexity: Default::default(),
            set_marquee_enabled: Default::default(),
            set_marquee_speed: Default::default(),
            set_marquee_offset: Default::default(),
            request_update: Default::default(),
            component_width_changed: Default::default(),
            component_height_changed: Default::default(),
            current_position_changed: Default::default(),
            duration_changed: Default::default(),
            zoom_level_changed: Default::default(),
            is_playing_changed: Default::default(),
            background_color_changed: Default::default(),
            waveform_color_changed: Default::default(),
            center_line_color_changed: Default::default(),
            cursor_color_changed: Default::default(),
            waveform_complexity_changed: Default::default(),
            marquee_enabled_changed: Default::default(),
            marquee_speed_changed: Default::default(),
            marquee_offset_changed: Default::default(),
        }
    }

    /// Set playback position
    fn set_position(&mut self, position: f64) {
        if (self.current_position - position).abs() > f64::EPSILON {
            self.current_position = position.clamp(0.0, self.duration);
            self.current_position_changed();
            (self as &dyn QQuickItem).update();
        }
    }

    /// Set total duration
    fn set_duration(&mut self, duration: f64) {
        if (self.duration - duration).abs() > f64::EPSILON {
            self.duration = duration.max(0.0);
            self.duration_changed();
            (self as &dyn QQuickItem).update();
        }
    }

    /// Set zoom level
    fn set_zoom(&mut self, zoom: f64) {
        if (self.zoom_level - zoom).abs() > f64::EPSILON {
            self.zoom_level = zoom.clamp(0.1, 10.0);
            self.zoom_level_changed();
            (self as &dyn QQuickItem).update();
        }
    }

    /// Set waveform complexity
    fn set_complexity(&mut self, complexity: i32) {
        if self.waveform_complexity != complexity {
            self.waveform_complexity = complexity.clamp(100, 10000);
            self.waveform_complexity_changed();
            (self as &dyn QQuickItem).update();
        }
    }

    /// Set marquee enabled state
    fn set_marquee_enabled(&mut self, enabled: bool) {
        if self.marquee_enabled != enabled {
            self.marquee_enabled = enabled;
            self.marquee_enabled_changed();
            (self as &dyn QQuickItem).update();
        }
    }

    /// Set marquee speed (pixels per second)
    fn set_marquee_speed(&mut self, speed: f64) {
        if (self.marquee_speed - speed).abs() > f64::EPSILON {
            self.marquee_speed = speed.max(0.0);
            self.marquee_speed_changed();
            (self as &dyn QQuickItem).update();
        }
    }

    /// Set marquee offset (pixel position)
    fn set_marquee_offset(&mut self, offset: f64) {
        if (self.marquee_offset - offset).abs() > f64::EPSILON {
            self.marquee_offset = offset;
            self.marquee_offset_changed();
            (self as &dyn QQuickItem).update();
        }
    }

    /// Request a manual update
    fn request_update(&mut self) {
        (self as &dyn QQuickItem).update();
    }

    /// Generate waveform data for the current viewport
    fn generate_waveform_data(&self, width: i32) -> Vec<f64> {
        let mut waveform_data = Vec::with_capacity(width as usize);

        let height = self.component_height;
        let max_amplitude = height * 0.8; // Use 80% of height

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
            let waveform_height = (final_amplitude.abs() * max_amplitude).max(1.0);

            waveform_data.push(waveform_height);
        }

        waveform_data
    }
}

// Implement QQuickItem first (required for QQuickPaintedItem)
impl QQuickItem for LatencyWaveformComponent {
    fn class_begin(&mut self) {
        // Initialize default values
        self.background_color = QColor::from_name("#000000");
        self.waveform_color = QColor::from_name("#4CAF50");
        self.center_line_color = QColor::from_name("#666666");
        self.cursor_color = QColor::from_name("#FF0000");
    }

    fn geometry_changed(&mut self, new: QRectF, old: QRectF) {
        // Update our internal dimensions when geometry changes
        self.component_width = new.width;
        self.component_height = new.height;
        self.component_width_changed();
        self.component_height_changed();

        tracing::debug!("Geometry changed from {:?} to {:?}", old, new);
        (self as &dyn QQuickItem).update();
    }
}

// Implement QQuickPaintedItem for custom painting
impl QQuickPaintedItem for LatencyWaveformComponent {
    fn paint(&mut self, painter: &mut QPainter) {
        tracing::debug!(
            "Paint called with dimensions: {}x{}",
            self.component_width,
            self.component_height
        );
        self.paint_waveform(painter);
    }
}

impl LatencyWaveformComponent {
    /// Core waveform painting implementation using QPainter
    fn paint_waveform(&self, painter: &mut QPainter) {
        let width = self.component_width;
        let height = self.component_height;

        let rect = QRectF {
            x: 0.0,
            y: 0.0,
            width: width,
            height: height,
        };

        // Clear background
        painter.fill_rect(rect, QBrush::from_color(self.background_color));

        if self.duration <= 0.0 || width <= 0.0 || height <= 0.0 {
            tracing::debug!(
                "Skipping paint: duration={}, width={}, height={}",
                self.duration,
                width,
                height
            );
            return;
        }

        let width_i32 = width as i32;
        let center_y = height / 2.0;

        // Generate waveform data
        let waveform_data = self.generate_waveform_data(width_i32);

        // Set up brush for waveform drawing
        let waveform_brush = QBrush::from_color(self.waveform_color);
        painter.set_brush(waveform_brush);

        // Apply marquee offset if enabled
        if self.marquee_enabled {
            let offset_x = -self.marquee_offset;
            painter.translate(QPointF {
                x: offset_x,
                y: 0.0,
            });
        }

        // Draw waveform using QPainter rectangles
        for (x, &waveform_height) in waveform_data.iter().enumerate() {
            if waveform_height > 0.0 {
                let waveform_rect = QRectF {
                    x: x as f64,
                    y: center_y - waveform_height / 2.0,
                    width: 1.0,
                    height: waveform_height,
                };
                painter.fill_rect(waveform_rect, QBrush::from_color(self.waveform_color));
            }
        }

        // Draw a second pass for seamless looping (when content wraps around)
        if self.marquee_enabled {
            painter.translate(QPointF { x: width, y: 0.0 });
            for (x, &waveform_height) in waveform_data.iter().enumerate() {
                if waveform_height > 0.0 {
                    let waveform_rect = QRectF {
                        x: x as f64,
                        y: center_y - waveform_height / 2.0,
                        width: 1.0,
                        height: waveform_height,
                    };
                    painter.fill_rect(waveform_rect, QBrush::from_color(self.waveform_color));
                }
            }
            // Reset transform for drawing center line and cursor
            painter.translate(QPointF {
                x: -width + self.marquee_offset,
                y: 0.0,
            });
        }

        // Draw center line
        let center_pen = QPen::from_color(self.center_line_color);
        painter.set_pen(center_pen);
        let center_line = QLineF {
            pt1: QPointF {
                x: 0.0,
                y: center_y,
            },
            pt2: QPointF {
                x: width,
                y: center_y,
            },
        };
        painter.draw_line(center_line);

        // Draw playback cursor (fixed at center)
        let cursor_x = width / 2.0;
        let cursor_pen = QPen::from_color(self.cursor_color);
        painter.set_pen(cursor_pen);
        let cursor_line = QLineF {
            pt1: QPointF {
                x: cursor_x,
                y: 0.0,
            },
            pt2: QPointF {
                x: cursor_x,
                y: height,
            },
        };
        painter.draw_line(cursor_line);
    }
}
