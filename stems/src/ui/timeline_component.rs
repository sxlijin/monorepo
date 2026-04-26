use qmetaobject::*;

/// Interactive timeline component providing seek, zoom, and pan functionality
/// Works in conjunction with WaveformView to provide a complete timeline experience
#[derive(QObject)]
pub struct TimelineComponent {
    base: qt_base_class!(trait QQuickPaintedItem),
    
    // Timeline properties
    width: qt_property!(f64; NOTIFY width_changed),
    height: qt_property!(f64; NOTIFY height_changed),
    
    // Time properties
    current_position: qt_property!(f64; NOTIFY current_position_changed),
    duration: qt_property!(f64; NOTIFY duration_changed),
    zoom_level: qt_property!(f64; NOTIFY zoom_level_changed),
    scroll_offset: qt_property!(f64; NOTIFY scroll_offset_changed),
    
    // Visual properties
    background_color: qt_property!(QColor; NOTIFY background_color_changed),
    tick_color: qt_property!(QColor; NOTIFY tick_color_changed),
    text_color: qt_property!(QColor; NOTIFY text_color_changed),
    cursor_color: qt_property!(QColor; NOTIFY cursor_color_changed),
    
    // Interaction properties
    show_milliseconds: qt_property!(bool; NOTIFY show_milliseconds_changed),
    major_tick_interval: qt_property!(f64; NOTIFY major_tick_interval_changed),
    minor_tick_interval: qt_property!(f64; NOTIFY minor_tick_interval_changed),
    
    // Methods exposed to QML
    set_position: qt_method!(fn(&mut self, position: f64)),
    set_duration: qt_method!(fn(&mut self, duration: f64)),
    set_zoom: qt_method!(fn(&mut self, zoom: f64)),
    set_scroll: qt_method!(fn(&mut self, offset: f64)),
    zoom_in: qt_method!(fn(&mut self)),
    zoom_out: qt_method!(fn(&mut self)),
    zoom_to_fit: qt_method!(fn(&mut self)),
    
    // Mouse interaction
    mouse_position_to_time: qt_method!(fn(&self, x: f64) -> f64),
    
    // Signals
    width_changed: qt_signal!(),
    height_changed: qt_signal!(),
    current_position_changed: qt_signal!(),
    duration_changed: qt_signal!(),
    zoom_level_changed: qt_signal!(),
    scroll_offset_changed: qt_signal!(),
    background_color_changed: qt_signal!(),
    tick_color_changed: qt_signal!(),
    text_color_changed: qt_signal!(),
    cursor_color_changed: qt_signal!(),
    show_milliseconds_changed: qt_signal!(),
    major_tick_interval_changed: qt_signal!(),
    minor_tick_interval_changed: qt_signal!(),
    
    // User interaction signals
    seek_requested: qt_signal!(position: f64),
    zoom_changed: qt_signal!(zoom_level: f64),
    scroll_changed: qt_signal!(scroll_offset: f64),
}

impl Default for TimelineComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl TimelineComponent {
    pub fn new() -> Self {
        Self {
            base: Default::default(),
            width: 800.0,
            height: 40.0,
            current_position: 0.0,
            duration: 0.0,
            zoom_level: 1.0,
            scroll_offset: 0.0,
            background_color: QColor::from_name("#f0f0f0"),
            tick_color: QColor::from_name("#666666"),
            text_color: QColor::from_name("#333333"),
            cursor_color: QColor::from_name("#ff0000"),
            show_milliseconds: false,
            major_tick_interval: 1.0, // 1 second
            minor_tick_interval: 0.1, // 100ms
            set_position: Default::default(),
            set_duration: Default::default(),
            set_zoom: Default::default(),
            set_scroll: Default::default(),
            zoom_in: Default::default(),
            zoom_out: Default::default(),
            zoom_to_fit: Default::default(),
            mouse_position_to_time: Default::default(),
            width_changed: Default::default(),
            height_changed: Default::default(),
            current_position_changed: Default::default(),
            duration_changed: Default::default(),
            zoom_level_changed: Default::default(),
            scroll_offset_changed: Default::default(),
            background_color_changed: Default::default(),
            tick_color_changed: Default::default(),
            text_color_changed: Default::default(),
            cursor_color_changed: Default::default(),
            show_milliseconds_changed: Default::default(),
            major_tick_interval_changed: Default::default(),
            minor_tick_interval_changed: Default::default(),
            seek_requested: Default::default(),
            zoom_changed: Default::default(),
            scroll_changed: Default::default(),
        }
    }
    
    /// Set playback position
    fn set_position(&mut self, position: f64) {
        let new_position = position.clamp(0.0, self.duration);
        if (self.current_position - new_position).abs() > 0.001 {
            self.current_position = new_position;
            self.current_position_changed();
            self.update();
        }
    }
    
    /// Set total duration
    fn set_duration(&mut self, duration: f64) {
        if (self.duration - duration).abs() > 0.001 {
            self.duration = duration.max(0.0);
            self.duration_changed();
            
            // Auto-adjust zoom if we're zoomed beyond the new duration
            let visible_duration = self.duration / self.zoom_level;
            if visible_duration < 0.1 { // Minimum 100ms visible
                self.zoom_to_fit();
            }
            
            self.update();
        }
    }
    
    /// Set zoom level
    fn set_zoom(&mut self, zoom: f64) {
        let new_zoom = zoom.clamp(1.0, 1000.0);
        if (self.zoom_level - new_zoom).abs() > 0.01 {
            self.zoom_level = new_zoom;
            self.zoom_level_changed();
            self.zoom_changed(new_zoom);
            
            // Adjust scroll if needed
            let visible_duration = self.duration / self.zoom_level;
            if self.scroll_offset + visible_duration > self.duration {
                self.set_scroll((self.duration - visible_duration).max(0.0));
            }
            
            self.update();
        }
    }
    
    /// Set scroll offset
    fn set_scroll(&mut self, offset: f64) {
        let visible_duration = self.duration / self.zoom_level;
        let max_offset = (self.duration - visible_duration).max(0.0);
        let new_offset = offset.clamp(0.0, max_offset);
        
        if (self.scroll_offset - new_offset).abs() > 0.001 {
            self.scroll_offset = new_offset;
            self.scroll_offset_changed();
            self.scroll_changed(new_offset);
            self.update();
        }
    }
    
    /// Zoom in by 2x
    fn zoom_in(&mut self) {
        self.set_zoom(self.zoom_level * 2.0);
    }
    
    /// Zoom out by 2x
    fn zoom_out(&mut self) {
        self.set_zoom(self.zoom_level / 2.0);
    }
    
    /// Reset zoom to show full duration
    fn zoom_to_fit(&mut self) {
        self.set_zoom(1.0);
        self.set_scroll(0.0);
    }
    
    /// Convert mouse X position to time in seconds
    fn mouse_position_to_time(&self, x: f64) -> f64 {
        if self.width <= 0.0 || self.duration <= 0.0 {
            return 0.0;
        }
        
        let visible_duration = self.duration / self.zoom_level;
        let time_per_pixel = visible_duration / self.width;
        let relative_time = x * time_per_pixel;
        
        (self.scroll_offset + relative_time).clamp(0.0, self.duration)
    }
    
    /// Format time for display
    fn format_time(&self, time_seconds: f64) -> String {
        let total_seconds = time_seconds as i32;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        
        if self.show_milliseconds {
            let milliseconds = ((time_seconds * 1000.0) as i32) % 1000;
            format!("{:02}:{:02}.{:03}", minutes, seconds, milliseconds)
        } else {
            format!("{:02}:{:02}", minutes, seconds)
        }
    }
    
    /// Calculate appropriate tick intervals based on zoom level
    fn calculate_tick_intervals(&self) -> (f64, f64) {
        let visible_duration = self.duration / self.zoom_level;
        let pixels_per_second = self.width / visible_duration;
        
        // Adjust intervals based on zoom level for optimal display
        let (major, minor) = if pixels_per_second > 200.0 {
            // Very zoomed in - show 100ms ticks
            (1.0, 0.1)
        } else if pixels_per_second > 50.0 {
            // Moderately zoomed - show 1s ticks
            (5.0, 1.0)
        } else if pixels_per_second > 10.0 {
            // Normal zoom - show 5s ticks
            (10.0, 5.0)
        } else {
            // Zoomed out - show 30s ticks
            (30.0, 10.0)
        };
        
        (major, minor)
    }
}

// Implement QQuickPaintedItem for custom painting
impl QQuickPaintedItem for TimelineComponent {
    fn paint(&mut self, painter: &mut QPainter) {
        self.paint_timeline(painter);
    }
}

impl TimelineComponent {
    /// Core timeline painting implementation
    fn paint_timeline(&self, painter: &mut QPainter) {
        let rect = QRectF { x: 0.0, y: 0.0, width: self.width, height: self.height };
        
        // Clear background
        painter.fill_rect(rect, self.background_color);
        
        if self.duration <= 0.0 || self.width <= 0.0 {
            return;
        }
        
        // Calculate visible time range
        let visible_duration = self.duration / self.zoom_level;
        let start_time = self.scroll_offset;
        let end_time = (start_time + visible_duration).min(self.duration);
        
        let (major_interval, minor_interval) = self.calculate_tick_intervals();
        
        // Set up pens
        let tick_pen = QPen::from_color(self.tick_color);
        let text_pen = QPen::from_color(self.text_color);
        
        // Draw minor ticks
        painter.set_pen(tick_pen);
        let mut time = (start_time / minor_interval).floor() * minor_interval;
        while time <= end_time {
            if time >= start_time {
                let x = ((time - start_time) / visible_duration) * self.width;
                let tick_height = self.height * 0.3;
                
                let tick_line = QLineF { 
                    x1: x, y1: self.height - tick_height, 
                    x2: x, y2: self.height 
                };
                painter.draw_line(tick_line);
            }
            time += minor_interval;
        }
        
        // Draw major ticks with labels
        time = (start_time / major_interval).floor() * major_interval;
        while time <= end_time {
            if time >= start_time {
                let x = ((time - start_time) / visible_duration) * self.width;
                let tick_height = self.height * 0.6;
                
                // Draw tick
                painter.set_pen(tick_pen);
                let major_tick_line = QLineF { 
                    x1: x, y1: self.height - tick_height, 
                    x2: x, y2: self.height 
                };
                painter.draw_line(major_tick_line);
                
                // Draw label
                painter.set_pen(text_pen);
                let time_text = self.format_time(time);
                let text_pos = QPointF { x: x, y: self.height - tick_height - 5.0 };
                painter.draw_text(text_pos, QString::from(time_text));
            }
            time += major_interval;
        }
        
        // Draw playback position cursor
        if self.current_position >= start_time && self.current_position <= end_time {
            let cursor_pen = QPen::from_color(self.cursor_color);
            painter.set_pen(cursor_pen);
            
            let relative_position = (self.current_position - start_time) / visible_duration;
            let cursor_x = relative_position * self.width;
            
            let cursor_line = QLineF { 
                x1: cursor_x, y1: 0.0, 
                x2: cursor_x, y2: self.height 
            };
            painter.draw_line(cursor_line);
        }
    }
}