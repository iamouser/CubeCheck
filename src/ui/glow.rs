use std::f32::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

use eframe::egui;
use eframe::egui::epaint::{Mesh, Vertex, WHITE_UV};

use crate::config::GlowConfig;

pub fn request_repaint(ui: &egui::Ui, area: egui::Rect, glow: Option<&GlowConfig>) {
    let Some(glow) = glow else {
        return;
    };
    let pointer = ui.input(|i| i.pointer.hover_pos());
    if pointer.is_some_and(|p| area.expand(glow.radius.max(1.0)).contains(p)) {
        ui.ctx().request_repaint();
    }
}

pub fn widget_text(
    ui: &mut egui::Ui,
    text: impl Into<egui::WidgetText>,
    wrap: egui::TextWrapMode,
    glow: Option<&GlowConfig>,
    sense: egui::Sense,
) -> egui::Response {
    let width = match wrap {
        egui::TextWrapMode::Wrap => ui.available_width(),
        _ => f32::INFINITY,
    };
    let galley = text
        .into()
        .into_galley(ui, Some(wrap), width, egui::FontSelection::Default);
    let (rect, response) = ui.allocate_exact_size(galley.size(), sense);
    let pos = match wrap {
        egui::TextWrapMode::Wrap => rect.min,
        _ => egui::pos2(rect.left(), rect.center().y - galley.size().y * 0.5),
    };
    paint_at(ui, pos, galley.as_ref(), egui::Color32::WHITE, glow);
    response
}

pub fn label(
    ui: &mut egui::Ui,
    text: impl Into<egui::WidgetText>,
    glow: Option<&GlowConfig>,
) -> egui::Response {
    widget_text(ui, text, egui::TextWrapMode::Extend, glow, egui::Sense::hover())
}

pub fn wrapped(
    ui: &mut egui::Ui,
    text: impl Into<egui::WidgetText>,
    glow: Option<&GlowConfig>,
) -> egui::Response {
    widget_text(ui, text, egui::TextWrapMode::Wrap, glow, egui::Sense::hover())
}

pub fn clickable(
    ui: &mut egui::Ui,
    text: impl Into<egui::WidgetText>,
    glow: Option<&GlowConfig>,
) -> egui::Response {
    widget_text(ui, text, egui::TextWrapMode::Extend, glow, egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn paint_at(
    ui: &egui::Ui,
    pos: egui::Pos2,
    galley: &egui::Galley,
    fallback: egui::Color32,
    glow: Option<&GlowConfig>,
) {
    ui.painter()
        .galley(pos, Arc::new(galley.clone()), fallback);
    let pointer = ui.input(|i| i.pointer.hover_pos());
    paint_overlay(ui, galley, pos, pointer, glow);
}

pub fn paint_overlay(
    ui: &egui::Ui,
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
    pointer: Option<egui::Pos2>,
    glow: Option<&GlowConfig>,
) {
    let Some(glow) = glow else {
        return;
    };
    let Some(pointer) = pointer else {
        return;
    };
    let radius = glow.radius.max(1.0);
    let bounds = egui::Rect::from_min_size(galley_pos, galley.size()).expand(radius);
    if !bounds.contains(pointer) {
        return;
    }
    let time = ui.input(|i| i.time);
    if let Some(overlay) = apply_cursor_glow(galley, galley_pos, pointer, glow, time) {
        ui.painter()
            .galley(galley_pos, overlay, rgb(glow.color));
    }
}

pub fn paint_frame_glow(
    ui: &egui::Ui,
    rect: egui::Rect,
    rounding: f32,
    glow: Option<&GlowConfig>,
) {
    let Some(glow) = glow else {
        return;
    };
    let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let radius = glow.radius.max(1.0);
    if !rect.expand(radius).contains(pointer) {
        return;
    }
    let time = ui.input(|i| i.time);
    let intensity = glow.intensity.clamp(0.0, 3.0);
    let points = rounded_rect_outline(rect.shrink(0.5), rounding, 5.0);
    let mut mesh = Mesh {
        texture_id: egui::TextureId::default(),
        ..Default::default()
    };
    for pair in points.windows(2) {
        let a = pair[0];
        let b = pair[1];
        let da = a.distance(pointer);
        let db = b.distance(pointer);
        if da >= radius && db >= radius {
            continue;
        }
        let ta = smoothstep(1.0 - da / radius) * intensity;
        let tb = smoothstep(1.0 - db / radius) * intensity;
        if ta.max(tb) <= 0.02 {
            continue;
        }
        let ca = glow_color(glow, time, a).gamma_multiply(ta.clamp(0.0, 1.0));
        let cb = glow_color(glow, time, b).gamma_multiply(tb.clamp(0.0, 1.0));
        emit_line(&mut mesh, a, b, ca, cb, 1.2);
    }
    if !mesh.is_empty() {
        ui.painter().add(mesh);
    }
}

fn apply_cursor_glow(
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
    pointer: egui::Pos2,
    glow: &GlowConfig,
    time: f64,
) -> Option<Arc<egui::Galley>> {
    let radius = glow.radius.max(1.0);
    let r2 = radius * radius;
    let intensity = glow.intensity.clamp(0.0, 3.0);
    let mut overlay = galley.clone();
    let mut any = false;
    for row in &mut overlay.rows {
        for v in &mut row.visuals.mesh.vertices {
            let world = galley_pos + v.pos.to_vec2();
            let d2 = world.distance_sq(pointer);
            if d2 >= r2 {
                v.color = egui::Color32::TRANSPARENT;
                continue;
            }
            let t = smoothstep(1.0 - d2.sqrt() / radius) * intensity;
            if t > 0.02 {
                any = true;
            }
            v.color = glow_color(glow, time, world).gamma_multiply(t.clamp(0.0, 1.0));
        }
    }
    any.then(|| Arc::new(overlay))
}

fn glow_color(glow: &GlowConfig, time: f64, pos: egui::Pos2) -> egui::Color32 {
    if !glow.gradient {
        return rgb(glow.color);
    }
    let speed = f64::from(glow.gradient_speed);
    let phase = (f64::from(pos.x) * 0.018
        + f64::from(pos.y) * 0.012
        + time * speed * std::f64::consts::TAU)
        .sin()
        * 0.5
        + 0.5;
    lerp_rgb(glow.color, glow.color2, phase as f32)
}

fn rgb(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(c[0], c[1], c[2])
}

fn lerp_rgb(a: [u8; 3], b: [u8; 3], t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    egui::Color32::from_rgb(
        lerp_u8(a[0], b[0], t),
        lerp_u8(a[1], b[1], t),
        lerp_u8(a[2], b[2], t),
    )
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn rounded_rect_outline(rect: egui::Rect, rounding: f32, spacing: f32) -> Vec<egui::Pos2> {
    let r = rounding.clamp(0.0, rect.width().min(rect.height()) * 0.5);
    let spacing = spacing.max(2.0);
    let mut pts = Vec::new();
    let x0 = rect.left() + r;
    let x1 = rect.right() - r;
    let y0 = rect.top() + r;
    let y1 = rect.bottom() - r;

    add_arc(&mut pts, x0, y0, PI, PI + FRAC_PI_2, r, spacing);
    add_line(
        &mut pts,
        egui::pos2(x0, rect.top()),
        egui::pos2(x1, rect.top()),
        spacing,
    );
    add_arc(&mut pts, x1, y0, -FRAC_PI_2, 0.0, r, spacing);
    add_line(
        &mut pts,
        egui::pos2(rect.right(), y0),
        egui::pos2(rect.right(), y1),
        spacing,
    );
    add_arc(&mut pts, x1, y1, 0.0, FRAC_PI_2, r, spacing);
    add_line(
        &mut pts,
        egui::pos2(x1, rect.bottom()),
        egui::pos2(x0, rect.bottom()),
        spacing,
    );
    add_arc(&mut pts, x0, y1, FRAC_PI_2, PI, r, spacing);
    add_line(
        &mut pts,
        egui::pos2(rect.left(), y1),
        egui::pos2(rect.left(), y0),
        spacing,
    );
    if let Some(first) = pts.first().copied() {
        pts.push(first);
    }
    pts
}

fn add_arc(
    pts: &mut Vec<egui::Pos2>,
    cx: f32,
    cy: f32,
    a0: f32,
    a1: f32,
    r: f32,
    spacing: f32,
) {
    if r < 0.5 {
        return;
    }
    let arc_len = (a1 - a0).abs() * r;
    let n = ((arc_len / spacing).ceil() as usize).max(1);
    for i in 0..=n {
        let a = a0 + (a1 - a0) * (i as f32 / n as f32);
        let p = egui::pos2(cx + r * a.cos(), cy + r * a.sin());
        if pts.last().is_some_and(|q| q.distance(p) < 0.2) {
            continue;
        }
        pts.push(p);
    }
}

fn add_line(pts: &mut Vec<egui::Pos2>, a: egui::Pos2, b: egui::Pos2, spacing: f32) {
    let d = a.distance(b);
    if d < 0.2 {
        return;
    }
    let n = ((d / spacing).ceil() as usize).max(1);
    for i in 0..=n {
        let p = a.lerp(b, i as f32 / n as f32);
        if pts.last().is_some_and(|q| q.distance(p) < 0.2) {
            continue;
        }
        pts.push(p);
    }
}

fn emit_line(
    mesh: &mut Mesh,
    a: egui::Pos2,
    b: egui::Pos2,
    ca: egui::Color32,
    cb: egui::Color32,
    width: f32,
) {
    let dir = b - a;
    let len = dir.length();
    if len < 0.01 {
        return;
    }
    let n = dir / len;
    let perp = egui::vec2(-n.y, n.x) * (width * 0.5);
    let idx = mesh.vertices.len() as u32;
    mesh.vertices.extend_from_slice(&[
        Vertex {
            pos: a + perp,
            uv: WHITE_UV,
            color: ca,
        },
        Vertex {
            pos: a - perp,
            uv: WHITE_UV,
            color: ca,
        },
        Vertex {
            pos: b + perp,
            uv: WHITE_UV,
            color: cb,
        },
        Vertex {
            pos: b - perp,
            uv: WHITE_UV,
            color: cb,
        },
    ]);
    mesh.indices
        .extend_from_slice(&[idx, idx + 1, idx + 2, idx + 2, idx + 1, idx + 3]);
}
