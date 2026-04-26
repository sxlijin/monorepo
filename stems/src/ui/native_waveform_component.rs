use crate::constants::StemType;
use crate::ui::waveform_registry::{get_registry, StemWaveform};
use qmetaobject::*;

#[derive(QObject, Default)]
pub struct NativeWaveformComponent {
    base: qt_base_class!(trait QQuickPaintedItem),

    // Geometry tracked like latency component
    component_width: qt_property!(f64; NOTIFY props_changed),
    component_height: qt_property!(f64; NOTIFY props_changed),

    // Lightweight QML-facing props
    stem_index: qt_property!(i32; NOTIFY props_changed),
    zoom: qt_property!(f64; NOTIFY props_changed),
    scroll_offset: qt_property!(f64; NOTIFY props_changed),   // seconds to viewport start
    playhead_position: qt_property!(f64; NOTIFY props_changed),// seconds
    color: qt_property!(QColor; NOTIFY props_changed),

    // Methods
    set_zoom: qt_method!(fn(&mut self, z: f64)),
    set_scroll_offset: qt_method!(fn(&mut self, o: f64)),
    set_playhead_position: qt_method!(fn(&mut self, t: f64)),
    request_update: qt_method!(fn(&mut self)),

    // Single NOTIFY signal
    props_changed: qt_signal!(),

    // Internal epoch tracking (optional)
    last_epoch: u64,
}

impl NativeWaveformComponent {
    fn time_to_x(t: f32, zoom: f64, scroll: f64, w: i32) -> Option<i32> {
        if w <= 0 { return None; }
        let base = 5.0f64; // seconds at zoom=1
        let viewport = (base / zoom.max(0.1)).max(0.1);
        let start = scroll.max(0.0);
        let end = start + viewport;
        let tf = t as f64;
        if tf < start || tf > end { return None; }
        let norm = (tf - start) / (end - start);
        Some(((norm * w as f64).floor() as i32).clamp(0, w.saturating_sub(1)))
    }
}

// Implement QML-callable methods to update props and request repaint
impl NativeWaveformComponent {
    fn set_zoom(&mut self, z: f64) {
        if (self.zoom - z).abs() > f64::EPSILON {
            self.zoom = z;
            self.props_changed();
            (self as &dyn QQuickItem).update();
        }
    }
    fn set_scroll_offset(&mut self, o: f64) {
        if (self.scroll_offset - o).abs() > f64::EPSILON {
            self.scroll_offset = o;
            self.props_changed();
            (self as &dyn QQuickItem).update();
        }
    }
    fn set_playhead_position(&mut self, t: f64) {
        if (self.playhead_position - t).abs() > f64::EPSILON {
            self.playhead_position = t;
            self.props_changed();
            (self as &dyn QQuickItem).update();
        }
    }
    fn request_update(&mut self) {
        (self as &dyn QQuickItem).update();
    }
}

impl QQuickItem for NativeWaveformComponent {
    fn class_begin(&mut self) {}

    fn geometry_changed(&mut self, new: QRectF, _old: QRectF) {
        self.component_width = new.width;
        self.component_height = new.height;
        self.props_changed();
        (self as &dyn QQuickItem).update();
    }
}

impl QQuickPaintedItem for NativeWaveformComponent {
    fn paint(&mut self, painter: &mut QPainter) {
        let w = self.component_width as i32;
        let h = self.component_height as i32;
        if w <= 0 || h <= 0 { return; }

        // Background
        let rect = QRectF { x: 0.0, y: 0.0, width: self.component_width, height: self.component_height };
        painter.fill_rect(rect, QBrush::from_color(QColor::from_rgba(0,0,0,0)));

        // Read-only snapshots (no locks during iteration)
        let reg = get_registry();
        let epoch = reg.epoch();
        let stem_index = self.stem_index.max(0) as usize;
        let data = StemType::from_index(stem_index)
            .map(|stem| reg.get_stem(stem))
            .unwrap_or_else(StemWaveform::empty);
        let StemWaveform { peaks, beats } = data;

        // Draw center line
        let mid = self.component_height / 2.0;
        let center_pen = QPen::from_color(QColor::from_rgb(80,80,80));
        painter.set_pen(center_pen);
        let center_line = QLineF { pt1: QPointF { x: 0.0, y: mid }, pt2: QPointF { x: self.component_width, y: mid } };
        painter.draw_line(center_line);

        // Draw waveform columns from peaks
        let bar_brush = QBrush::from_color(self.color);
        if !peaks.is_empty() {
            // Map time window to columns. We assume peaks are uniform over duration.
            // For now, render a simple proportional mapping based on number of peaks.
            let total_cols = w.max(1) as usize;
            let len = peaks.len().max(1);
            let stride = (len as f64 / total_cols as f64).max(1.0);
            for x in 0..(w as usize) {
                let idx = ((x as f64) * stride).floor() as usize;
                if idx < len {
                    let (mn, mx) = peaks[idx];
                    let amp_min = (mn.clamp(-1.0, 1.0) as f64) * (self.component_height * 0.45);
                    let amp_max = (mx.clamp(-1.0, 1.0) as f64) * (self.component_height * 0.45);
                    let y_min = mid - amp_min;
                    let y_max = mid - amp_max;
                    let top = y_min.min(y_max);
                    let height = (y_max - y_min).abs().max(1.0);
                    let column = QRectF { x: x as f64, y: top, width: 1.0, height };
                    painter.fill_rect(column, bar_brush.clone());
                }
            }
        }

        // Draw beats
        let beat_pen = QPen::from_color(QColor::from_rgb(180,120,60));
        painter.set_pen(beat_pen);
        for &t in beats.iter() {
            if let Some(x) = Self::time_to_x(t, self.zoom, self.scroll_offset, w as i32) {
                let line = QLineF { pt1: QPointF { x: x as f64, y: 0.0 }, pt2: QPointF { x: x as f64, y: self.component_height } };
                painter.draw_line(line);
            }
        }

        // Fixed playhead cursor at 20%
        let cursor_x = self.component_width * 0.2;
        let cursor_pen = QPen::from_color(QColor::from_rgb(220,220,220));
        painter.set_pen(cursor_pen);
        let cursor_line = QLineF { pt1: QPointF { x: cursor_x, y: 0.0 }, pt2: QPointF { x: cursor_x, y: self.component_height } };
        painter.draw_line(cursor_line);

        self.last_epoch = epoch;
    }
}
