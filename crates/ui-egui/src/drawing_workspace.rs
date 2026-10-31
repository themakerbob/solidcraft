//! V0.1.8 Drawing workspace: selectable projected views and optional labels.
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
    projected: HashMap<NamedView, Vec<ProjectedStroke>>,
    projection_error: Option<String>,
}
#[derive(Clone)]
struct ProjectedStroke { points: Vec<ModelVec2>, hidden: bool, uncertain: bool }
struct PlacedView { id: u64, frame: ViewFrame, orientation: NamedView, at: ModelVec2, show_label: bool }
impl Default for State {
    fn default() -> Self {
        Self { active: false, orientation: NamedView::Front, scale_index: 0, placed: Vec::new(), place_mode: false, zoom: 1.0, pan: Vec2::ZERO, selected: None, next_view_id: 1, drag_view: None, show_browser: true, show_view_labels: true, model_revision: None, projected: HashMap::new(), projection_error: None }
    }
}
const SCALES: [(f64,f64); 5] = [(1.0,1.0),(1.0,2.0),(1.0,5.0),(1.0,10.0),(2.0,1.0)];
const A3_W: f32 = 420.0;
const A3_H: f32 = 297.0;

pub fn show(state: &mut State, ui: &mut egui::Ui, session: &solidcraft_engine::Session) {
    refresh_projection(state, session);
    ui.horizontal(|ui| {
        if ui.selectable_label(state.show_browser, "Views").clicked() { state.show_browser = !state.show_browser; }
        ui.separator();
        ui.label("A3 landscape");
        ui.checkbox(&mut state.show_view_labels, "Show View Labels");
        egui::ComboBox::from_id_salt("drawing_orientation")
            .selected_text(format!("{:?}", state.orientation))
            .show_ui(ui, |ui| {
                for v in NamedView::ALL { ui.selectable_value(&mut state.orientation, v, format!("{:?}", v)); }
            });
        egui::ComboBox::from_id_salt("drawing_scale")
            .selected_text(format!("{}:{}", SCALES[state.scale_index].0, SCALES[state.scale_index].1))
            .show_ui(ui, |ui| {
                for (i, (paper, model)) in SCALES.iter().enumerate() {
                    ui.selectable_value(&mut state.scale_index, i, format!("{}:{}", paper, model));
                }
            });
        if ui.button("Base View").clicked() { state.place_mode = true; state.selected = None; }
        if ui.button("Projected View").on_hover_text("Requires model projection; coming in a later revision").clicked() {
            // No placeholder geometry: projected views require actual model geometry.
        }
        if ui.button("Fit Sheet").clicked() { state.zoom = 1.0; state.pan = Vec2::ZERO; }
        if ui.add_enabled(state.selected.is_some(), egui::Button::new("Delete View")).clicked() { delete_selected(state); }
        if ui.button("Cancel").clicked() { state.place_mode = false; }
    });
    ui.separator();
    if state.place_mode {
        ui.label("Base View: click inside the sheet border to place an anchor. Press Escape to cancel.");
    } else {
        ui.label("Wheel: zoom at cursor  |  Middle-drag: pan  |  Click: select view  |  Delete: remove selected  |  Fit Sheet: reset view");
    }
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        if state.place_mode { state.place_mode = false; } else { state.selected = None; }
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
        browser.label("Sheet 1 · A3 Landscape");
        browser.label("Views");
        if state.placed.is_empty() { browser.weak("No views placed"); }
        for (i, view) in state.placed.iter().enumerate() {
            if browser.selectable_label(state.selected == Some(view.id),
                format!("V{} · {:?}", i + 1, view.orientation)).clicked() {
                state.selected = Some(view.id);
                state.place_mode = false;
            }
        }
        if let Some(id) = state.selected {
            if let Some(view) = state.placed.iter_mut().find(|v| v.id == id) {
                browser.checkbox(&mut view.show_label, "Selected view label");
            }
        }
        browser.separator();
        if let Some(err) = &state.projection_error { browser.colored_label(Color32::LIGHT_RED, err); }
        else { browser.weak("Exact HLR model projection"); }
    }
    let canvas_rect = Rect::from_min_max(
        Pos2::new(area.min.x + browser_width, area.min.y), area.max);
    let mut canvas = ui.new_child(egui::UiBuilder::new().max_rect(canvas_rect).id_salt("drawing_canvas_v015"));
    draw_canvas(state, &mut canvas);
    ui.allocate_rect(area, Sense::hover());
}

fn draw_canvas(state: &mut State, ui: &mut egui::Ui) {
    let available = ui.available_rect_before_wrap().size();
    let (response, painter) = ui.allocate_painter(available, Sense::click_and_drag());
    painter.rect_filled(response.rect, 0.0, Color32::from_gray(55));
    let base_factor = ((available.x - 40.0) / A3_W).min((available.y - 40.0) / A3_H).max(0.01);
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
    let paper = Rect::from_center_size(center, Vec2::new(A3_W * factor, A3_H * factor));
    let border = paper.shrink(10.0 * factor);
    painter.rect_filled(paper, 0.0, Color32::WHITE);
    painter.rect_stroke(paper, 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
    painter.rect_stroke(border, 0.0, Stroke::new(0.7_f32.max(factor * 0.5), Color32::DARK_GRAY), StrokeKind::Inside);
    let title = Rect::from_min_max(Pos2::new(border.max.x - 120.0 * factor, border.max.y - 35.0 * factor), border.max);
    painter.rect_stroke(title, 0.0, Stroke::new(1.0, Color32::DARK_GRAY), StrokeKind::Inside);
    painter.text(title.center(), egui::Align2::CENTER_CENTER, "SolidCraft · A3", egui::FontId::proportional(12.0), Color32::BLACK);
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
            for stroke in strokes {
                let color = if stroke.uncertain { Color32::from_rgb(180, 100, 30) }
                    else if stroke.hidden { Color32::from_gray(145) }
                    else { Color32::BLACK };
                let width = if stroke.hidden { 0.8 } else { 1.2 };
                for pair in stroke.points.windows(2) {
                    let a = Pos2::new(p.x + pair[0].x as f32 * ratio * factor,
                                      p.y - pair[0].y as f32 * ratio * factor);
                    let b = Pos2::new(p.x + pair[1].x as f32 * ratio * factor,
                                      p.y - pair[1].y as f32 * ratio * factor);
                    if stroke.hidden { // short dashed segments, preserving visibility classification
                        let delta = b - a;
                        let len = delta.length();
                        if len > 0.0 {
                            let dir = delta / len;
                            let mut t = 0.0;
                            while t < len {
                                let end = (t + 5.0).min(len);
                                painter.line_segment([a + dir*t, a + dir*end], Stroke::new(width, color));
                                t += 8.0;
                            }
                        }
                    } else {
                        painter.line_segment([a,b], Stroke::new(width,color));
                    }
                }
            }
        }
        if state.selected == Some(view.id) {
            let bounds = view_bounds(state, view, paper, factor);
            painter.rect_stroke(bounds.expand(7.0), 0.0,
                Stroke::new(1.2, Color32::from_rgb(35, 110, 215)), StrokeKind::Outside);
        }
        if !(state.show_view_labels && view.show_label) { continue; }
        let bounds = view_bounds(state, view, paper, factor);
        let label_at = Pos2::new(bounds.center().x, bounds.max.y + 12.0);
        let c = if state.selected == Some(view.id) { Color32::from_rgb(220, 110, 35) } else { Color32::from_rgb(35, 90, 165) };
        painter.text(label_at, egui::Align2::CENTER_TOP,
            format!("V{} {:?} {}", i+1, view.orientation, view.frame.scale()),
            egui::FontId::proportional(12.0), c);
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
            for point in &stroke.points {
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
    state.projection_error = None;
    let model = session.world_state();
    if model.bodies.is_empty() { return; }
    let options = hlr::Options::default();
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
        let view = hlr::View { x: basis.x, y: basis.y, d: basis.x.cross(basis.y) };
        match hlr::hidden_lines(&defs, &placements, &view, &options, &|| false) {
            Ok(result) => {
                let mut strokes = Vec::new();
                for line in result.lines {
                    for piece in line.pieces {
                        if piece.covered { continue; }
                        for seg in piece.geometry {
                            // All supported exact curve kinds are sampled for display only;
                            // geometry and dimensions remain in model millimetres.
                            let n = match &seg { Seg2::Line { .. } => 1, _ => 32 };
                            let points = (0..=n).map(|i| seg.point_at(i as f64/n as f64)).collect();
                            strokes.push(ProjectedStroke {
                                points,
                                hidden: piece.visibility == hlr::Visibility::Hidden,
                                uncertain: piece.visibility == hlr::Visibility::Uncertain,
                            });
                        }
                    }
                }
                state.projected.insert(orientation, strokes);
            }
            Err(e) => { state.projection_error = Some(format!("Projection: {e}")); break; }
        }
    }
}
