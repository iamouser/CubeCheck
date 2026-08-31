use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};

use eframe::egui;
use serde::Deserialize;

/// Frozen pixel offsets for a few widgets. This is not an editor.
const BAKED_LAYOUT: &str = include_str!("baked_layout.json");

thread_local! {
    static OFFSETS: RefCell<HashMap<String, [f32; 2]>> = RefCell::new(HashMap::new());
}

#[derive(Deserialize)]
struct LayoutFile {
    items: BTreeMap<String, LayoutItem>,
}

#[derive(Deserialize)]
struct LayoutItem {
    x: f32,
    y: f32,
}

fn snap(value: f32) -> f32 {
    if value.is_finite() {
        value.round()
    } else {
        0.0
    }
}

const OFFSET_CLAMP: f32 = 240.0;

/// Overlays and the reset control must keep a 1:1 visual/hitbox match.
fn hitbox_locked(id: &str) -> bool {
    id == "reset.button" || id == "reset.undo" || id.starts_with("dialog.")
}

fn sanitize_offset(id: &str, x: f32, y: f32) -> [f32; 2] {
    let x = snap(x);
    let y = snap(y);
    if hitbox_locked(id) {
        return [0.0, 0.0];
    }
    if x.abs() > OFFSET_CLAMP || y.abs() > OFFSET_CLAMP {
        return [0.0, 0.0];
    }
    [x, y]
}

fn parse_layout(text: &str) -> HashMap<String, [f32; 2]> {
    let Ok(file) = serde_json::from_str::<LayoutFile>(text) else {
        return HashMap::new();
    };
    file.items
        .into_iter()
        .filter_map(|(id, item)| {
            let offset = sanitize_offset(&id, item.x, item.y);
            if offset == [0.0, 0.0] {
                None
            } else {
                Some((id, offset))
            }
        })
        .collect()
}

pub fn load() {
    let offsets = parse_layout(BAKED_LAYOUT);
    OFFSETS.with(|cell| {
        *cell.borrow_mut() = offsets;
    });
}

fn applied_offset(id: &str) -> [f32; 2] {
    if hitbox_locked(id) {
        return [0.0, 0.0];
    }
    OFFSETS.with(|cell| cell.borrow().get(id).copied().unwrap_or([0.0, 0.0]))
}

/// Places a widget group, applying baked pixel offsets when present.
pub fn layout_move<R>(ui: &mut egui::Ui, id: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let offset = applied_offset(id);
    if offset == [0.0, 0.0] {
        return add(ui);
    }

    // `next_widget_position()` is the *center* of a zero-size widget. Using it as
    // the top-left of a child Ui makes every offset item in a wrap/row step
    // down-and-right. Anchor to the remaining rect.
    let available = ui.available_rect_before_wrap();
    let origin = available.min;
    let avail = available.size().max(egui::vec2(1.0, 1.0));
    let child_min = origin + egui::vec2(offset[0], offset[1]);
    let child_rect = egui::Rect::from_min_size(child_min, avail);

    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(child_rect)
            .layout(egui::Layout::top_down(egui::Align::Min))
            .id_salt(("layout_move", id)),
    );
    let result = add(&mut child);
    let used = child.min_rect();

    if used.width() > 0.0 && used.height() > 0.0 {
        ui.advance_cursor_after_rect(egui::Rect::from_min_size(origin, used.size()));
    }

    result
}
