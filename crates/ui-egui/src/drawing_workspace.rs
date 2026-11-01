//! V0.1.11 Drawing workspace: selectable projected views and optional labels.
use egui::{Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use solidcraft_geom::drawing_view::{NamedView, Scale, ViewFrame};
use solidcraft_geom::Vec2 as ModelVec2;
use solidcraft_engine::geom::{hlr, Seg2, Vec3};
use solidcraft_engine::kernel;
use std::collections::HashMap;


pub struct State {
    pub active: bool,
    orientation: NamedView,
    scale_index: usize,
    placed: Vec<PlacedView>,
    place_mode: bool,
    zoom: f32,
    pan: Vec2,
    selected: Option<u64>,
    next_view_id: u64,
    drag_view: Option<u64>,
    show_browser: bool,
    show_view_labels: bool,
    model_revision: Option<(u64, u64)>,
    projected: HashMap<NamedView, Vec<ProjectedStroke>> ,
    hlr_debug: HashMap<NamedView, Vec<DebugSegment>>,
    export_message: Option<String>,
    projection_error: Option<String>,
    edit_view: Option<u64>,
    sheet_settings: bool,
    sheet_size: usize,
    landscape: bool,
    show_border: bool,
    show_title: bool,
    show_hidden: bool,
    visible_only_debug: bool,
    projection_first_angle: bool,

}
#[derive(Clone)]
struct DebugSegment {
    line_index: usize,
    piece_index: usize,
    segment_index: usize,
    covered: bool,
    visibility: &'static str,
    points: Vec<ModelVec2>,
    curve: Seg2,
}
#[derive(Clone)]
struct ProjectedStroke { curve: Seg2, hidden: bool, uncertain: bool }
struct PlacedView { id: u64, frame: ViewFrame, orientation: NamedView, at: ModelVec2, show_label: bool }
impl Default for State {
    fn default() -> Self {
        Self { active: false, orientation: NamedView::Front, scale_index: 0, placed: Vec::new(), place_mode: false, zoom: 1.0, pan: Vec2::ZERO, selected: None, next_view_id: 1, drag_view: None, show_browser: true, show_view_labels: true, model_revision: None, projected: HashMap::new(), hlr_debug: HashMap::new(), export_message: None, projection_error: None, edit_view: None, sheet_settings: false, sheet_size: 1, landscape: true, show_border: true, show_title: true, show_hidden: true, visible_only_debug: false, projection_first_angle: false }
    }
}
const SCALES: [(f64,f64); 5] = [(1.0,1.0),(1.0,2.0),(1.0,5.0),(1.0,10.0),(2.0,1.0)];
const SHEETS: [(&str, f32, f32); 5] = [
    ("A4", 210.0, 297.0), ("A3", 297.0, 420.0), ("A2", 420.0, 594.0),
    ("A1", 594.0, 841.0), ("A0", 841.0, 1189.0),
];
fn sheet_dimensions(state: &State) -> (f32, f32) {
    let (_, short, long) = SHEETS[state.sheet_size];
    if state.landscape { (long, short) } else { (short, long) }
}
fn scale_index(scale: Scale) -> usize {
    SCALES.iter().position(|&(a,b)| Scale::new(a,b).ok().is_some_and(|candidate| (candidate.factor() - scale.factor()).abs() < 1e-9)).unwrap_or(0)
}



/// Commands used by the native Drawing ribbon. Keep the same state transitions
/// for ribbon actions and browser actions, rather than duplicating handlers.
#[derive(Clone, Copy)]
pub(crate) enum DrawingCommand { Base, Edit, Sheet, Fit, Delete, Cancel, ToggleBrowser }

pub(crate) fn command_enabled(state: &State, command: DrawingCommand) -> bool {
    match command {
        DrawingCommand::Edit | DrawingCommand::Delete => state.selected.is_some(),
        _ => true,
    }
}

pub(crate) fn command_active(state: &State, command: DrawingCommand) -> bool {
    match command {
        DrawingCommand::Base => state.place_mode,
        DrawingCommand::Edit => state.edit_view.is_some(),
        DrawingCommand::Sheet => state.sheet_settings,
        DrawingCommand::ToggleBrowser => state.show_browser,
        _ => false,
    }
}

pub(crate) fn run_command(state: &mut State, command: DrawingCommand) {
    match command {
        DrawingCommand::Base => {
            state.place_mode = true;
            state.edit_view = None;
            state.sheet_settings = false;
            state.selected = None;
        }
        DrawingCommand::Edit => {
            if let Some(id) = state.selected {
                state.edit_view = Some(id);
                if let Some(v) = state.placed.iter().find(|v| v.id == id) {
                    state.orientation = v.orientation;
                    state.scale_index = scale_index(v.frame.scale());
                }
                state.place_mode = false;
                state.sheet_settings = false;
            }
        }
        DrawingCommand::Sheet => {
            state.sheet_settings = true;
            state.place_mode = false;
            state.edit_view = None;
        }
        DrawingCommand::Fit => { state.zoom = 1.0; state.pan = Vec2::ZERO; }
        DrawingCommand::Delete => delete_selected(state),
        DrawingCommand::Cancel => {
            state.place_mode = false;
            state.edit_view = None;
            state.sheet_settings = false;
        }
        DrawingCommand::ToggleBrowser => state.show_browser = !state.show_browser,
    }
}

pub fn show(state: &mut State, ui: &mut egui::Ui, session: &solidcraft_engine::Session) {
    ui.horizontal(|ui| { ui.checkbox(&mut state.visible_only_debug, "Visible only (debug)"); });
    refresh_projection(state, session);
    // Drawing commands live in the native SolidCraft ribbon, not a second toolbar.
    // The browser and sheet canvas begin immediately below the ribbon.
    if state.place_mode {
        ui.label("Base View: click inside the sheet border to place an anchor. Press Escape to cancel.");
    } else {
        ui.label("Wheel: zoom at cursor  |  Middle-drag: pan  |  Click: select view  |  Delete: remove selected  |  Fit Sheet: reset view");
    }
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        if state.place_mode || state.edit_view.is_some() || state.sheet_settings { state.place_mode = false; state.edit_view = None; state.sheet_settings = false; } else { state.selected = None; }
        state.drag_view = None;
    }
    if !ui.ctx().egui_wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::Delete)) {
        delete_selected(state);
    }
    // Allocate the remaining drawing editor rectangle explicitly. Nested horizontal
    // layouts can shrink-wrap their height to one row, leaving a tiny canvas.
    let area = ui.available_rect_before_wrap();
    if area.width() < 40.0 || area.height() < 40.0 { return; }
    let browser_width = if state.show_browser { 205.0_f32.min(area.width() * 0.4) } else { 0.0 };
    if state.show_browser {
        let browser_rect = Rect::from_min_max(area.min, Pos2::new(area.min.x + browser_width, area.max.y));
        let mut browser = ui.new_child(egui::UiBuilder::new().max_rect(browser_rect).id_salt("drawing_browser_v015"));
        browser.painter().rect_filled(browser_rect, 0.0, Color32::from_rgb(36, 40, 49));
        browser.add_space(10.0);
        browser.heading("Drawing");
        browser.separator();
        browser.label(format!("Sheet 1 · {} {}", SHEETS[state.sheet_size].0,
            if state.landscape { "Landscape" } else { "Portrait" }));
        if browser.button("⚙ Sheet Settings").clicked() {
            state.sheet_settings = true; state.place_mode = false; state.edit_view = None;
        }
        browser.label("Views");
        if state.placed.is_empty() { browser.weak("No views placed"); }
        let mut open_edit = None;
        for (i, view) in state.placed.iter().enumerate() {
            let entry = browser.selectable_label(state.selected == Some(view.id),
                format!("V{} · {:?}", i + 1, view.orientation));
            if entry.clicked() { state.selected = Some(view.id); state.place_mode = false; }
            if entry.double_clicked() { open_edit = Some(view.id); }
        }
        if let Some(id) = open_edit {
            state.selected = Some(id); state.edit_view = Some(id); state.sheet_settings = false;
            if let Some(v) = state.placed.iter().find(|v| v.id == id) {
                state.orientation = v.orientation; state.scale_index = scale_index(v.frame.scale());
            }
        }
        if let Some(id) = state.selected {
            if browser.button("Edit Selected View").clicked() {
                state.edit_view = Some(id); state.sheet_settings = false;
                if let Some(v) = state.placed.iter().find(|v| v.id == id) {
                    state.orientation = v.orientation; state.scale_index = scale_index(v.frame.scale());
                }
            }
            if let Some(view) = state.placed.iter_mut().find(|v| v.id == id) {
                browser.checkbox(&mut view.show_label, "Selected view label");
            }
        }
        browser.separator();
        if browser.button("Export HLR Debug (CSV)").clicked() {
            state.export_message = Some(match export_hlr_debug(state) {
                Ok(path) => format!("Exported: {}", path.display()),
                Err(err) => format!("Export failed: {err}"),
            });
        }
        if browser.button("Export Selected View (DXF)").clicked() {
            state.export_message = Some(match export_hlr_dxf(state) {
                Ok(path) => format!("Exported DXF: {}", path.display()),
                Err(err) => format!("DXF export failed: {err}"),
            });
        }
        if let Some(message) = &state.export_message {
            browser.small(message);
        }
        if let Some(err) = &state.projection_error { browser.colored_label(Color32::LIGHT_RED, err); }
        else { browser.weak("Exact HLR model projection"); }

    }
    let canvas_rect = Rect::from_min_max(
        Pos2::new(area.min.x + browser_width, area.min.y), area.max);
    let mut canvas = ui.new_child(egui::UiBuilder::new().max_rect(canvas_rect).id_salt("drawing_canvas_v015"));
    draw_canvas(state, &mut canvas);
    if state.place_mode || state.edit_view.is_some() || state.sheet_settings {
        let palette_rect = Rect::from_min_size(
            Pos2::new(canvas_rect.max.x - 265.0, canvas_rect.min.y + 12.0),
            Vec2::new(253.0, 355.0));
        let mut palette = ui.new_child(egui::UiBuilder::new().max_rect(palette_rect).id_salt("drawing_properties_palette"));
        egui::Frame::new().fill(Color32::from_rgb(35, 39, 48))
            .inner_margin(12.0).show(&mut palette, |ui| {
                draw_properties(state, ui);
            });
    }
    ui.allocate_rect(area, Sense::hover());
}

fn draw_canvas(state: &mut State, ui: &mut egui::Ui) {
    let available = ui.available_rect_before_wrap().size();
    let (response, painter) = ui.allocate_painter(available, Sense::click_and_drag());
    painter.rect_filled(response.rect, 0.0, Color32::from_gray(55));
    let (sheet_w, sheet_h) = sheet_dimensions(state);
    let base_factor = ((available.x - 40.0) / sheet_w).min((available.y - 40.0) / sheet_h).max(0.01);
    let factor = base_factor * state.zoom;
    let mut center = response.rect.center() + state.pan;
    let hovering = response.hovered();
    if hovering {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.1 {
            let before = factor;
            state.zoom = (state.zoom * (scroll * 0.0015).exp()).clamp(0.15, 12.0);
            let after = base_factor * state.zoom;
            if let Some(mouse) = ui.input(|i| i.pointer.hover_pos()) {
                let anchor = mouse - center;
                state.pan += anchor * (1.0 - after / before);
                center = response.rect.center() + state.pan;
            }
        }
    }
    if response.dragged_by(egui::PointerButton::Middle) {
        let delta = ui.input(|i| i.pointer.delta());
        state.pan += delta;
        center += delta;
    }
    let paper = Rect::from_center_size(center, Vec2::new(sheet_w * factor, sheet_h * factor));
    let border = paper.shrink(10.0 * factor);
    painter.rect_filled(paper, 0.0, Color32::WHITE);
    painter.rect_stroke(paper, 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
    if state.show_border {
        painter.rect_stroke(border, 0.0, Stroke::new(0.7_f32.max(factor * 0.5), Color32::DARK_GRAY), StrokeKind::Inside);
    }
    let title = Rect::from_min_max(Pos2::new(border.max.x - 120.0 * factor, border.max.y - 35.0 * factor), border.max);
    if state.show_title {
        painter.rect_stroke(title, 0.0, Stroke::new(1.0, Color32::DARK_GRAY), StrokeKind::Inside);
        painter.text(title.center(), egui::Align2::CENTER_CENTER,
            format!("SolidCraft · {}", SHEETS[state.sheet_size].0),
            egui::FontId::proportional(12.0), Color32::BLACK);
    }
    if response.clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            if state.place_mode && border.contains(pointer) {
                let at = ModelVec2::new(((pointer.x - paper.min.x) / factor) as f64,
                                         ((paper.max.y - pointer.y) / factor) as f64);
                let (a,b) = SCALES[state.scale_index];
                if let Ok(scale) = Scale::new(a,b) {
                    if let Ok(frame) = ViewFrame::new(state.orientation.basis(), scale, ModelVec2::ZERO, at) {
                        let id = state.next_view_id;
                        state.next_view_id += 1;
                        state.placed.push(PlacedView { id, frame, orientation: state.orientation, at, show_label: true });
                        state.selected = Some(id);
                        state.place_mode = false;
                    }
                }
            } else if !state.place_mode {
                state.selected = hit_view(state, paper, factor, pointer);
                if response.double_clicked() {
                    state.edit_view = state.selected; state.sheet_settings = false;
                    if let Some(v) = state.placed.iter().find(|v| Some(v.id) == state.edit_view) {
                        state.orientation = v.orientation; state.scale_index = scale_index(v.frame.scale());
                    }
                }
            }
        }
    }
    if response.drag_started_by(egui::PointerButton::Primary) && !state.place_mode {
        state.drag_view = response.interact_pointer_pos().and_then(|p| hit_view(state, paper, factor, p));
        if let Some(id) = state.drag_view { state.selected = Some(id); }
    }
    if response.dragged_by(egui::PointerButton::Primary) {
        if let Some(id) = state.drag_view {
            let delta = ui.input(|i| i.pointer.delta());
            if let Some(view) = state.placed.iter_mut().find(|v| v.id == id) {
                view.at.x += (delta.x / factor) as f64;
                view.at.y -= (delta.y / factor) as f64;
                // ViewFrame retains the original anchor until the next geometry integration.
            }
        }
    }
    if response.drag_stopped_by(egui::PointerButton::Primary) { state.drag_view = None; }
    for (i, view) in state.placed.iter().enumerate() {
        let p = Pos2::new(paper.min.x + view.at.x as f32 * factor, paper.max.y - view.at.y as f32 * factor);
        // HLR coordinates are model millimetres. Apply paper:model ratio once,
        // then the canvas pixels/mm zoom. Keep drawing measurements in model units.
        if let Some(strokes) = state.projected.get(&view.orientation) {
            let ratio = view.frame.scale().factor() as f32;
            // Draw hidden and uncertain geometry underneath visible contours.
            // HLR output order is not a paint order: a dashed hidden segment
            // painted after a solid contour can make it appear broken at 1:1.
            for pass in 0..2 {
                for stroke in strokes {
                    if state.visible_only_debug && (stroke.hidden || stroke.uncertain) { continue; }
                    if stroke.hidden && !state.show_hidden { continue; }
                    let is_visible = !stroke.hidden && !stroke.uncertain;
                    if (pass == 1) != is_visible { continue; }
                    let color = if stroke.uncertain { Color32::from_rgb(180, 100, 30) }
                        else if stroke.hidden { Color32::from_gray(145) }
                        else { Color32::BLACK };
                    let width = if stroke.hidden { 1.0 } else { 1.4 };
                    let sampled = adaptive_curve_points(&stroke.curve, ratio * factor, 0.20);
                    let screen_points: Vec<Pos2> = sampled.iter().map(|point| {
                        Pos2::new(p.x + point.x as f32 * ratio * factor,
                                  p.y - point.y as f32 * ratio * factor)
                    }).collect();
                    if stroke.hidden {
                        // Keep dash phase continuous along the entire polyline.
                        // Dash lengths are screen pixels, independent of model zoom.
                        let mut distance = 0.0_f32;
                        for pair in screen_points.windows(2) {
                            let (a, b) = (pair[0], pair[1]);
                            let delta = b - a;
                            let len = delta.length();
                            if len <= f32::EPSILON { continue; }
                            let dir = delta / len;
                            let mut t = 0.0_f32;
                            while t < len {
                                let phase = (distance + t) % 8.0;
                                let step = (if phase < 5.0 { 5.0 - phase } else { 8.0 - phase }).min(len - t);
                                if phase < 5.0 && step > 0.0 {
                                    painter.line_segment([a + dir * t, a + dir * (t + step)], Stroke::new(width, color));
                                }
                                t += step.max(0.0001);
                            }
                            distance += len;
                        }
                    } else if screen_points.len() >= 2 {
                        // A single AA path per original HLR curve; never merge separate edges.
                        painter.add(egui::Shape::line(screen_points, Stroke::new(width, color)));
                    }
                }
            }
        }
        if state.selected == Some(view.id) {
            let bounds = view_bounds(state, view, paper, factor);
            painter.rect_stroke(bounds.expand(7.0), 0.0,
                Stroke::new(1.8, Color32::from_rgb(35, 110, 215)), StrokeKind::Outside);
        }
        if !(state.show_view_labels && view.show_label) { continue; }
        let bounds = view_bounds(state, view, paper, factor);
        let label_at = Pos2::new(bounds.center().x, bounds.max.y + 12.0);
        let c = if state.selected == Some(view.id) { Color32::from_rgb(220, 110, 35) } else { Color32::from_rgb(35, 90, 165) };
        painter.text(label_at, egui::Align2::CENTER_TOP,
            format!("V{} {:?} {}", i+1, view.orientation, view.frame.scale()),
            egui::FontId::proportional(12.0), c);
    }
    // Live preview: use the exact cached HLR strokes at the cursor, not a box.
    if state.place_mode {
        if let Some(pointer) = response.hover_pos().filter(|p| border.contains(*p)) {
            if let Some(strokes) = state.projected.get(&state.orientation) {
                let ratio = (SCALES[state.scale_index].0 / SCALES[state.scale_index].1) as f32;
                for stroke in strokes {
                    if stroke.hidden && !state.show_hidden { continue; }
                    let color = if stroke.hidden { Color32::from_rgba_unmultiplied(45, 110, 210, 100) }
                        else { Color32::from_rgb(45, 110, 210) };
                    let preview_points: Vec<Pos2> =
                        adaptive_curve_points(&stroke.curve, ratio * factor, 0.20)
                            .into_iter()
                            .map(|point| Pos2::new(
                                pointer.x + point.x as f32 * ratio * factor,
                                pointer.y - point.y as f32 * ratio * factor,
                            ))
                            .collect();
                    if preview_points.len() >= 2 {
                        painter.add(egui::Shape::line(preview_points, Stroke::new(1.1, color)));
                    }
                }
            }
        }
    }
}

fn draw_properties(state: &mut State, ui: &mut egui::Ui) {
    if state.sheet_settings {
        ui.heading("Sheet Settings");
        ui.separator();
        egui::ComboBox::from_id_salt("drawing_sheet_size")
            .selected_text(SHEETS[state.sheet_size].0)
            .show_ui(ui, |ui| {
                for (i, (name, _, _)) in SHEETS.iter().enumerate() {
                    ui.selectable_value(&mut state.sheet_size, i, *name);
                }
            });
        ui.horizontal(|ui| {
            ui.selectable_value(&mut state.landscape, true, "Landscape");
            ui.selectable_value(&mut state.landscape, false, "Portrait");
        });
        ui.label("Projection standard");
        ui.radio_value(&mut state.projection_first_angle, true, "First angle (ISO)");
        ui.radio_value(&mut state.projection_first_angle, false, "Third angle");
        ui.checkbox(&mut state.show_border, "Sheet border");
        ui.checkbox(&mut state.show_title, "Title block");
        if ui.button("Done").clicked() { state.sheet_settings = false; }
        return;
    }
    let editing = state.edit_view.is_some();
    ui.heading(if editing { "Edit Drawing View" } else { "Place Base View" });
    ui.separator();
    ui.label("Orientation");
    let mut orientation = state.orientation;
    egui::ComboBox::from_id_salt("drawing_palette_orientation")
        .selected_text(format!("{:?}", orientation))
        .show_ui(ui, |ui| {
            for v in NamedView::ALL { ui.selectable_value(&mut orientation, v, format!("{:?}", v)); }
        });
    ui.label("Scale");
    let mut scale_index = state.scale_index;
    egui::ComboBox::from_id_salt("drawing_palette_scale")
        .selected_text(format!("{}:{}", SCALES[scale_index].0, SCALES[scale_index].1))
        .show_ui(ui, |ui| {
            for (i, (a,b)) in SCALES.iter().enumerate() {
                ui.selectable_value(&mut scale_index, i, format!("{}:{}", a,b));
            }
        });
    state.orientation = orientation;
    state.scale_index = scale_index;
    ui.checkbox(&mut state.show_hidden, "Show hidden lines");
    if let Some(id) = state.edit_view {
        if let Some(view) = state.placed.iter_mut().find(|v| v.id == id) {
            ui.checkbox(&mut view.show_label, "Show view label");
            if let Ok(scale) = Scale::new(SCALES[scale_index].0, SCALES[scale_index].1) {
                if let Ok(frame) = ViewFrame::new(orientation.basis(), scale, ModelVec2::ZERO, view.at) {
                    view.frame = frame;
                    view.orientation = orientation;
                }
            }
        }
        if ui.button("Done").clicked() { state.edit_view = None; }
    } else {
        ui.label("Move over the sheet to preview; click to place.");
        if ui.button("Cancel").clicked() { state.place_mode = false; }
    }
}

fn delete_selected(state: &mut State) {
    if let Some(id) = state.selected.take() {
        state.placed.retain(|view| view.id != id);
    }
    state.drag_view = None;
}

fn view_bounds(state: &State, view: &PlacedView, paper: Rect, factor: f32) -> Rect {
    let p = Pos2::new(paper.min.x + view.at.x as f32 * factor,
                      paper.max.y - view.at.y as f32 * factor);
    let ratio = view.frame.scale().factor() as f32 * factor;
    let mut min = p;
    let mut max = p;
    let mut has_geometry = false;
    if let Some(strokes) = state.projected.get(&view.orientation) {
        for stroke in strokes {
            for point in adaptive_curve_points(&stroke.curve, ratio, 0.20) {
                let pos = Pos2::new(p.x + point.x as f32 * ratio,
                                    p.y - point.y as f32 * ratio);
                if !pos.x.is_finite() || !pos.y.is_finite() { continue; }
                if !has_geometry { min = pos; max = pos; has_geometry = true; }
                else {
                    min.x = min.x.min(pos.x); min.y = min.y.min(pos.y);
                    max.x = max.x.max(pos.x); max.y = max.y.max(pos.y);
                }
            }
        }
    }
    if !has_geometry { return Rect::from_center_size(p, Vec2::splat(24.0)); }
    Rect::from_min_max(min, max).expand(3.0)
}

fn hit_view(state: &State, paper: Rect, factor: f32, pointer: Pos2) -> Option<u64> {
    state.placed.iter().rev().find_map(|view| {
        view_bounds(state, view, paper, factor).expand(6.0)
            .contains(pointer).then_some(view.id)
    })
}

// Cached per active-document revision: the exact kernel HLR is expensive and
// must never be recomputed on every egui repaint.
fn refresh_projection(state: &mut State, session: &solidcraft_engine::Session) {
    let key = (session.uid, session.revision);
    if state.model_revision == Some(key) { return; }
    state.model_revision = Some(key);
    state.projected.clear();
    state.hlr_debug.clear();
    state.export_message = None;
    state.projection_error = None;
    let model = session.world_state();
    if model.bodies.is_empty() { return; }
    // Technical drawings omit smooth/tangent face junctions by default.
    // These are not physical creases: on a filleted plate they appear as
    // spurious vertical lines at the start/end of each corner radius.
    // True silhouette curves are generated independently by the HLR engine.
    let options = hlr::Options { tangent_edges: false, ..hlr::Options::default() };
    let mut defs = Vec::new();
    for body in &model.bodies {
        match kernel::portable_brep(&body.body).and_then(|b| {
            hlr::prepare(&b, &options).map_err(|e| kernel::KernelError::Invalid(e.to_string()))
        }) {
            Ok(prepared) => defs.push(prepared),
            Err(e) => { state.projection_error = Some(format!("Geometry: {e}")); return; }
        }
    }
    let placements: Vec<hlr::Placement> = (0..defs.len()).map(|body| hlr::Placement {
        body,
        rotation: [Vec3::X, Vec3::Y, Vec3::Z],
        translation: Vec3::ZERO,
    }).collect();
    for orientation in NamedView::ALL {
        let basis = orientation.basis();
        let view = hlr::View { x: basis.x, y: basis.y, d: basis.y.cross(basis.x) };
        let projection = hlr::hidden_lines(&defs, &placements, &view, &options, &|| false);
        match projection {
            Ok(result) => {
                let mut strokes = Vec::new();
                let mut debug_segments = Vec::new();
                for (line_index, line) in result.lines.into_iter().enumerate() {
                    for (piece_index, piece) in line.pieces.into_iter().enumerate() {
                        for (segment_index, seg) in piece.geometry.into_iter().enumerate() {
                            // All supported exact curve kinds are sampled for display only;
                            // geometry and dimensions remain in model millimetres.
                            // Debug samples are model-space and zoom independent.
                            // The renderer tessellates the original analytic curve adaptively.
                            let points = adaptive_curve_points(&seg, 1.0, 0.02);
                            debug_segments.push(DebugSegment {
                                line_index,
                                piece_index,
                                segment_index,
                                covered: piece.covered,
                                visibility: match piece.visibility {
                                    hlr::Visibility::Visible => "visible",
                                    hlr::Visibility::Hidden => "hidden",
                                    hlr::Visibility::Uncertain => "uncertain",
                                },
                                points: points.clone(),
                                curve: seg.clone(),
                            });
                            if piece.covered { continue; }
                            strokes.push(ProjectedStroke {
                                curve: seg,
                                hidden: piece.visibility == hlr::Visibility::Hidden,
                                uncertain: piece.visibility == hlr::Visibility::Uncertain,
                            });
                        }
                    }
                }
                state.projected.insert(orientation, strokes);
                state.hlr_debug.insert(orientation, debug_segments);
            }
            Err(e) => { state.projection_error = Some(format!("Projection: {e}")); break; }
        }
    }
}

// V0.1.19: Export raw HLR segment data without changing the drawing renderer.
// All coordinates are in projected model millimetres, before view scale/zoom.
fn export_hlr_debug(state: &State) -> std::io::Result<std::path::PathBuf> {
    use std::io::Write;
    let dir = std::env::current_dir()?.join("drawing_debug");
    std::fs::create_dir_all(&dir)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default().as_millis();
    let path = dir.join(format!("hlr_debug_{stamp}.csv"));
    let mut out = std::io::BufWriter::new(std::fs::File::create(&path)?);
    writeln!(out, "view,line_index,piece_index,segment_index,covered,visibility,point_index,x_mm,y_mm,canvas_zoom,view_scale,visible_only,show_hidden")?;
    for orientation in NamedView::ALL {
        let Some(segments) = state.hlr_debug.get(&orientation) else { continue; };
        let view_scale = state.placed.iter()
            .find(|view| view.orientation == orientation)
            .map(|view| view.frame.scale().factor())
            .unwrap_or(1.0);
        for segment in segments {
            for (point_index, point) in segment.points.iter().enumerate() {
                writeln!(out, "{:?},{},{},{},{},{},{},{:.12},{:.12},{:.8},{:.8},{},{}",
                    orientation, segment.line_index, segment.piece_index,
                    segment.segment_index, segment.covered, segment.visibility,
                    point_index, point.x, point.y, state.zoom, view_scale,
                    state.visible_only_debug, state.show_hidden)?;
            }
        }
    }
    out.flush()?;
    Ok(path)
}

// V0.1.22: ASCII DXF R2000 export of raw HLR projected geometry.
// Uses the same cached geometry as CSV, without display consolidation or zoom.
// Every sampled curve segment becomes a LINE entity, preserving its classification.
/// Adaptive subdivision in screen space (rendering) or millimetres (DXF).
/// Midpoint + quarter-point tests detect curved and reversing spans that a
/// single midpoint test would incorrectly classify as straight.
fn adaptive_curve_points(curve: &Seg2, pixels_per_mm: f32, tolerance_px: f64) -> Vec<ModelVec2> {
    if let Seg2::Line { a, b } = curve { return vec![*a, *b]; }
    let scale = (pixels_per_mm as f64).max(1e-9);
    let tolerance = tolerance_px.max(0.001) / scale;
    let mut out = vec![curve.point_at(0.0)];
    fn deviation(p: ModelVec2, a: ModelVec2, b: ModelVec2) -> f64 {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let l2 = dx * dx + dy * dy;
        if l2 <= 1e-24 { return ((p.x-a.x).powi(2)+(p.y-a.y).powi(2)).sqrt(); }
        let t = (((p.x-a.x)*dx+(p.y-a.y)*dy)/l2).clamp(0.0,1.0);
        ((p.x-a.x-t*dx).powi(2)+(p.y-a.y-t*dy).powi(2)).sqrt()
    }
    fn visit(curve: &Seg2, t0: f64, a: ModelVec2, t1: f64, b: ModelVec2,
             tolerance: f64, depth: u32, out: &mut Vec<ModelVec2>) {
        let dt = t1-t0;
        let q1 = curve.point_at(t0+dt*0.25);
        let mid = curve.point_at(t0+dt*0.5);
        let q3 = curve.point_at(t0+dt*0.75);
        let err = deviation(q1,a,b).max(deviation(mid,a,b)).max(deviation(q3,a,b));
        if depth >= 16 || (err <= tolerance && depth >= 1) {
            out.push(b);
        } else {
            visit(curve,t0,a,t0+dt*0.5,mid,tolerance,depth+1,out);
            visit(curve,t0+dt*0.5,mid,t1,b,tolerance,depth+1,out);
        }
    }
    visit(curve,0.0,curve.point_at(0.0),1.0,curve.point_at(1.0),tolerance,0,&mut out);
    out
}

fn write_dxf_line(out: &mut impl std::io::Write, layer: &str, a: ModelVec2, b: ModelVec2) -> std::io::Result<()> {
    if (a.x-b.x).abs()+(a.y-b.y).abs() < 1e-12 { return Ok(()); }
    write!(out,"0\nLINE\n8\n{layer}\n10\n{:.12}\n20\n{:.12}\n30\n0\n11\n{:.12}\n21\n{:.12}\n31\n0\n",a.x,a.y,b.x,b.y)
}

fn write_dxf_polyline(out: &mut impl std::io::Write, layer: &str, points: &[ModelVec2]) -> std::io::Result<()> {
    if points.len() < 2 { return Ok(()); }
    write!(out,"0\nLWPOLYLINE\n8\n{layer}\n90\n{}\n70\n0\n",points.len())?;
    for p in points { write!(out,"10\n{:.12}\n20\n{:.12}\n",p.x,p.y)?; }
    Ok(())
}

fn export_hlr_dxf(state: &State) -> std::io::Result<std::path::PathBuf> {
    use std::io::Write;
    let orientation = state.selected
        .and_then(|id| state.placed.iter().find(|view| view.id == id))
        .map(|view| view.orientation)
        .or_else(|| state.placed.first().map(|view| view.orientation))
        .unwrap_or(state.orientation);
    let segments = state.hlr_debug.get(&orientation).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "No projected HLR geometry for selected view")
    })?;
    let dir = std::env::current_dir()?.join("drawing_debug");
    std::fs::create_dir_all(&dir)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default().as_millis();
    let path = dir.join(format!("hlr_{orientation:?}_{stamp}.dxf"));
    let mut out = std::io::BufWriter::new(std::fs::File::create(&path)?);
    // Standard DXF layers: visible black, hidden red dashed, covered magenta,
    // uncertain orange. Covered entities remain inspectable but are frozen by
    // default so they cannot obscure the normal drawing.
    write!(out, "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1015\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n")?;
    write!(out, "0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLTYPE\n70\n2\n")?;
    write!(out, "0\nLTYPE\n2\nCONTINUOUS\n70\n0\n3\nSolid\n72\n65\n73\n0\n40\n0.0\n")?;
    write!(out, "0\nLTYPE\n2\nHIDDEN\n70\n0\n3\nHidden\n72\n65\n73\n2\n40\n4.0\n49\n3.0\n74\n0\n49\n-1.0\n74\n0\n")?;
    write!(out, "0\nENDTAB\n0\nTABLE\n2\nLAYER\n70\n4\n")?;
    for (name, color, line_type, flags) in [
        ("VISIBLE", 7, "CONTINUOUS", 0),
        ("HIDDEN", 1, "HIDDEN", 0),
        ("UNCERTAIN", 30, "CONTINUOUS", 0),
        ("COVERED", 6, "CONTINUOUS", 1),
    ] {
        write!(out, "0\nLAYER\n2\n{name}\n70\n{flags}\n62\n{color}\n6\n{line_type}\n")?;
    }
    write!(out, "0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n")?;
    for segment in segments {
        let layer = if segment.covered { "COVERED" } else {
            match segment.visibility {
                "visible" => "VISIBLE",
                "hidden" => "HIDDEN",
                _ => "UNCERTAIN",
            }
        };
        match &segment.curve {
            Seg2::Line { a, b } => {
                write_dxf_line(&mut out, layer, *a, *b)?;
            }
            Seg2::Arc { center, radius, start, sweep } if *radius > 0.0 && radius.is_finite() => {
                // DXF ARC is counterclockwise; reverse negative sweeps by swapping endpoints.
                // Full turns are CIRCLE entities; other intervals remain exact ARC entities.
                let a0 = if *sweep >= 0.0 { *start } else { *start + *sweep };
                let a1 = a0 + sweep.abs();
                if sweep.abs() >= std::f64::consts::TAU - 1e-9 {
                    write!(out, "0\nCIRCLE\n8\n{layer}\n10\n{:.12}\n20\n{:.12}\n30\n0\n40\n{:.12}\n", center.x, center.y, radius)?;
                } else {
                    write!(out, "0\nARC\n8\n{layer}\n10\n{:.12}\n20\n{:.12}\n30\n0\n40\n{:.12}\n50\n{:.12}\n51\n{:.12}\n", center.x, center.y, radius, a0.to_degrees().rem_euclid(360.0), a1.to_degrees().rem_euclid(360.0))?;
                }
            }
            _ => {
                // Non-circular conics / splines: adaptive, bounded model-space
                // approximation until native rational spline DXF export is available.
                let points = adaptive_curve_points(&segment.curve, 1.0, 0.01);
                write_dxf_polyline(&mut out, layer, &points)?;
            }
        }
    }
    write!(out, "0\nENDSEC\n0\nEOF\n")?;
    out.flush()?;
    Ok(path)
}
