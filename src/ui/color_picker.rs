use std::hash::Hash;

use eframe::egui::{
    self, ecolor::Hsva, pos2, vec2, Align2, Area, Color32, Frame, Id, Key, Mesh, Order, Rect,
    Rounding, Sense, Shadow, Shape, Stroke, Ui, UiKind, Vec2,
};

const PANEL_FILL: Color32 = Color32::from_rgb(0x1a, 0x1a, 0x24);
const PANEL_STROKE: Color32 = Color32::from_rgb(0x3c, 0x3c, 0x4e);
const SV_SIZE: f32 = 188.0;
const HUE_BAR_W: f32 = 16.0;
const HUE_HIT_W: f32 = 24.0;
const HUE_STEPS: u32 = 36;

/// Color swatch that opens a theme-independent HSV popover (SV square + vertical hue).
pub fn color_edit_button_srgb(
    ui: &mut Ui,
    id_salt: impl Hash,
    srgb: &mut [u8; 3],
) -> egui::Response {
    let popup_id = ui.make_persistent_id((id_salt, "glow_hsv_popup"));
    let hsva_id = popup_id.with("hsva");
    let mut hsva = load_hsva(ui.ctx(), hsva_id, *srgb);

    let open = ui.memory(|mem| mem.is_popup_open(popup_id));
    let mut response = color_swatch(ui, Color32::from_rgb(srgb[0], srgb[1], srgb[2]), open);

    if response.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
    }

    if ui.memory(|mem| mem.is_popup_open(popup_id)) {
        let mut changed = false;
        let mut interacting = false;

        let area_response = Area::new(popup_id)
            .kind(UiKind::Picker)
            .order(Order::Foreground)
            .fixed_pos(response.rect.left_bottom() + vec2(0.0, 6.0))
            .pivot(Align2::LEFT_TOP)
            .constrain(true)
            .show(ui.ctx(), |ui| {
                Frame::none()
                    .fill(PANEL_FILL)
                    .stroke(Stroke::new(1.0, PANEL_STROKE))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(12.0))
                    .shadow(Shadow {
                        offset: vec2(0.0, 6.0),
                        blur: 18.0,
                        spread: 0.0,
                        color: Color32::from_black_alpha(150),
                    })
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = vec2(12.0, 0.0);
                        ui.horizontal(|ui| {
                            let sv = sat_value_square(ui, &mut hsva);
                            let hue = hue_slider(ui, &mut hsva.h, SV_SIZE);
                            changed = sv.changed() || hue.changed();
                            interacting = sv.dragged()
                                || hue.dragged()
                                || sv.drag_stopped()
                                || hue.drag_stopped();
                        });
                    });
            })
            .response;

        if changed {
            *srgb = hsva.to_srgb();
            response.mark_changed();
        }

        let close = !response.clicked()
            && !interacting
            && (ui.input(|i| i.key_pressed(Key::Escape)) || area_response.clicked_elsewhere());
        if close {
            ui.memory_mut(|mem| mem.close_popup());
        }
    }

    store_hsva(ui.ctx(), hsva_id, hsva);
    response
}

fn color_swatch(ui: &mut Ui, color: Color32, open: bool) -> egui::Response {
    let size = {
        let h = ui.spacing().interact_size.y.max(18.0);
        vec2(h + 4.0, h)
    };
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let rounding = Rounding::same(4.0);
        ui.painter().rect_filled(rect, rounding, color);
        let visuals = ui.visuals();
        let border = if open {
            visuals.selection.stroke.color
        } else {
            visuals.widgets.inactive.bg_stroke.color
        };
        ui.painter()
            .rect_stroke(rect, rounding, Stroke::new(1.5, border));
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn sat_value_square(ui: &mut Ui, hsva: &mut Hsva) -> egui::Response {
    let desired = Vec2::splat(SV_SIZE);
    let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click_and_drag());

    if let Some(pos) = response.interact_pointer_pos() {
        hsva.s = remap01(pos.x, rect.left(), rect.right());
        hsva.v = 1.0 - remap01(pos.y, rect.top(), rect.bottom());
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        paint_sv_square(ui, rect, hsva.h);
        let knob = pos2(
            lerp(rect.left(), rect.right(), hsva.s),
            lerp(rect.top(), rect.bottom(), 1.0 - hsva.v),
        );
        let current = Color32::from(*hsva);
        ui.painter()
            .circle_filled(knob, 6.0, current);
        ui.painter()
            .circle_stroke(knob, 7.0, Stroke::new(2.0, Color32::WHITE));
        ui.painter()
            .circle_stroke(knob, 8.2, Stroke::new(1.0, Color32::from_black_alpha(180)));
        ui.painter()
            .rect_stroke(rect, Rounding::same(3.0), Stroke::new(1.0, PANEL_STROKE));
    }

    if response.dragged() {
        ui.ctx().request_repaint();
    }
    response
}

fn hue_slider(ui: &mut Ui, hue: &mut f32, height: f32) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(vec2(HUE_HIT_W, height), Sense::click_and_drag());
    let bar = Rect::from_center_size(rect.center(), vec2(HUE_BAR_W, height));

    if let Some(pos) = response.interact_pointer_pos() {
        *hue = remap01(pos.y, bar.top(), bar.bottom());
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let mut mesh = Mesh::default();
        for i in 0..=HUE_STEPS {
            let t = i as f32 / HUE_STEPS as f32;
            let color = Color32::from(Hsva::new(t, 1.0, 1.0, 1.0));
            let y = lerp(bar.top(), bar.bottom(), t);
            mesh.colored_vertex(pos2(bar.left(), y), color);
            mesh.colored_vertex(pos2(bar.right(), y), color);
            if i < HUE_STEPS {
                let i = 2 * i;
                mesh.add_triangle(i, i + 1, i + 2);
                mesh.add_triangle(i + 1, i + 2, i + 3);
            }
        }
        ui.painter().add(Shape::mesh(mesh));
        ui.painter()
            .rect_stroke(bar, Rounding::same(3.0), Stroke::new(1.0, PANEL_STROKE));

        let y = lerp(bar.top(), bar.bottom(), *hue);
        let thumb = Rect::from_center_size(pos2(bar.center().x, y), vec2(HUE_BAR_W + 8.0, 10.0));
        ui.painter()
            .rect_filled(thumb, Rounding::same(3.0), Color32::from_rgb(0xf2, 0xf2, 0xf6));
        ui.painter().rect_stroke(
            thumb,
            Rounding::same(3.0),
            Stroke::new(1.0, Color32::from_rgb(0x18, 0x18, 0x22)),
        );
    }

    if response.dragged() {
        ui.ctx().request_repaint();
    }
    response
}

fn paint_sv_square(ui: &Ui, rect: Rect, hue: f32) {
    let hue_color = Color32::from(Hsva::new(hue, 1.0, 1.0, 1.0));
    let mut base = Mesh::default();
    base.colored_vertex(rect.left_top(), Color32::WHITE);
    base.colored_vertex(rect.right_top(), hue_color);
    base.colored_vertex(rect.left_bottom(), Color32::WHITE);
    base.colored_vertex(rect.right_bottom(), hue_color);
    base.add_triangle(0, 1, 2);
    base.add_triangle(1, 2, 3);
    ui.painter().add(Shape::mesh(base));

    let mut shade = Mesh::default();
    shade.colored_vertex(rect.left_top(), Color32::TRANSPARENT);
    shade.colored_vertex(rect.right_top(), Color32::TRANSPARENT);
    shade.colored_vertex(rect.left_bottom(), Color32::BLACK);
    shade.colored_vertex(rect.right_bottom(), Color32::BLACK);
    shade.add_triangle(0, 1, 2);
    shade.add_triangle(1, 2, 3);
    ui.painter().add(Shape::mesh(shade));
}

fn load_hsva(ctx: &egui::Context, id: Id, srgb: [u8; 3]) -> Hsva {
    let fresh = Hsva::from_srgb(srgb);
    match ctx.data(|d| d.get_temp::<Hsva>(id)) {
        Some(cached) if cached.to_srgb() == srgb => cached,
        Some(cached) if fresh.s < 1.0 / 255.0 => Hsva {
            h: cached.h,
            ..fresh
        },
        _ => fresh,
    }
}

fn store_hsva(ctx: &egui::Context, id: Id, hsva: Hsva) {
    ctx.data_mut(|d| d.insert_temp(id, hsva));
}

fn remap01(v: f32, a: f32, b: f32) -> f32 {
    if (b - a).abs() < f32::EPSILON {
        0.0
    } else {
        ((v - a) / (b - a)).clamp(0.0, 1.0)
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
