use std::collections::VecDeque;

use egui::{Color32, Sense, Ui, Vec2};

pub fn sparkline(ui: &mut Ui, data: &VecDeque<f64>, max_val: f64, color: Color32) {
    let desired_size = Vec2::new(60.0, 16.0);
    let (rect, _response) = ui.allocate_exact_size(desired_size, Sense::hover());

    if !ui.is_rect_visible(rect) || data.is_empty() {
        return;
    }

    let painter = ui.painter_at(rect);

    painter.rect_filled(
        rect,
        1.0,
        ui.visuals().extreme_bg_color,
    );

    let max = if max_val > 0.0 { max_val } else { 1.0 };
    let n = data.len();

    if n < 2 {
        return;
    }

    let points: Vec<egui::Pos2> = data
        .iter()
        .enumerate()
        .map(|(i, &val)| {
            let x = rect.left() + (i as f32 / (n - 1) as f32) * rect.width();
            let y = rect.bottom() - (val as f32 / max as f32).clamp(0.0, 1.0) * rect.height();
            egui::pos2(x, y)
        })
        .collect();

    for window in points.windows(2) {
        painter.line_segment([window[0], window[1]], egui::Stroke::new(1.0, color));
    }
}

pub fn sparkline_with_label(
    ui: &mut Ui,
    data: &VecDeque<f64>,
    max_val: f64,
    color: Color32,
    label: &str,
) {
    ui.horizontal(|ui| {
        sparkline(ui, data, max_val, color);
        ui.label(label);
    });
}
