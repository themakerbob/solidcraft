//! V0.1.27 Drawing workspace: depth-buffered shaded views and analytic HLR overlays.
use egui::{Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use solidcraft_geom::drawing_view::{NamedView, Scale, ViewFrame};
use solidcraft_geom::Vec2 as ModelVec2;
use solidcraft_engine::geom::{hlr, Seg2, Vec3};
use solidcraft_engine::kernel;
use std::collections::HashMap;


pub struct State {
    pub active: bool,
    drawing_file_path: String,
    drawing_model_path: String,
    /// Identity of the document currently owning this drawing state.
    drawing_session_uid: Option<u64>,
    orientation: NamedView,
    scale_index: usize,
    placed: Vec<PlacedView>,
    place_mode: bool,
    project_mode: bool,
    project_parent: Option<u64>,
    section_mode: bool,
    section_pending: Option<(u64,f64)>,
    section_mesh: Vec<[Vec3;3]>,
    zoom: f32,
    pan: Vec2,
    selected: Option<u64>,
    next_view_id: u64,
    drag_view: Option<u64>,
    drag_section_cut: Option<u64>,
    show_browser: bool,
    show_view_labels: bool,
    model_revision: Option<(u64, u64)>,
    smart_dimensions_stale: bool,
    projected: HashMap<NamedView, Vec<ProjectedStroke>> ,
    hlr_debug: HashMap<NamedView, Vec<DebugSegment>>,
    shaded: HashMap<NamedView, DrawingShade>,
    export_message: Option<String>,
    toast_started_at: f64,
    toast_last_text: String,
    pub(crate) utilities_tab: bool,
    projection_error: Option<String>,
    edit_view: Option<u64>,
    sheet_settings: bool,
    sheets: Vec<NamedSheet>,
    active_sheet: usize,
    add_sheet_dialog: bool,
    new_sheet_name: String,
    new_sheet_size: usize,
    new_sheet_landscape: bool,
    sheet_size: usize,
    landscape: bool,
    show_border: bool,
    show_title: bool,
    title_part: String,
    title_number: String,
    title_material: String,
    title_author: String,
    revisions: Vec<(String,String,String)>,
    show_hidden: bool,
    visible_only_debug: bool,
    projection_first_angle: bool,
    undo: Vec<DrawingSnapshot>,
    redo: Vec<DrawingSnapshot>,
    drag_before: Option<DrawingSnapshot>,
    annotations: Vec<DrawingAnnotation>,
    annotation_tool: AnnotationTool,
    annotation_first: Option<ModelVec2>,
    annotation_text: String,
    annotation_count: u32,
    annotation_pitch: f64,
    annotation_diameter: f64,
    smart_first: Option<SmartPick>,
    centre_first: Option<(u64,ModelVec2,ModelVec2)>,
    smart_pending: Option<SmartPending>,
    smart_drag: Option<(usize, bool)>,
    annotation_selected: Option<usize>,
    tolerance_edit: Option<usize>,

}
#[derive(Clone)]
struct NamedSheet { name: String, drawing: DrawingSnapshot }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AnnotationTool { None, Smart, Horizontal, Vertical, Aligned, Leader, Pattern, Note, CentreMark, CentreLine }
#[derive(Clone)]
enum DrawingAnnotation {
    Dimension { a: ModelVec2, b: ModelVec2, kind: AnnotationTool, offset: f64 },
    Leader { target: ModelVec2, text_at: ModelVec2, text: String },
    Note { at: ModelVec2, text: String, height_mm: f32, align: NoteAlign, bold: bool, italic: bool, underline: bool },
    Smart { view_id: u64, geometry: SmartGeometry, offset: f64, text_shift: ModelVec2, tolerance: DimensionTolerance, binding: Option<DimensionBinding> },
    CentreMark { view_id: u64, center: ModelVec2, radius: f64 },
    CentreLine { view_id: u64, a: ModelVec2, b: ModelVec2 },
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NoteAlign { Left, Center, Right }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ToleranceMode { None, Symmetric, Bilateral, Limits, Basic, Reference }
#[derive(Clone, PartialEq)]
struct DimensionTolerance { mode: ToleranceMode, upper: f64, lower: f64, precision: usize }
impl Default for DimensionTolerance {
    fn default() -> Self { Self { mode: ToleranceMode::None, upper: 0.1, lower: 0.1, precision: 2 } }
}
fn tolerance_label(base: &str, tolerance: &DimensionTolerance) -> String {
    let p = tolerance.precision.min(5);
    let value = base.parse::<f64>();
    match tolerance.mode {
        ToleranceMode::None => base.to_owned(),
        ToleranceMode::Basic => format!("[{base}]"),
        ToleranceMode::Reference => format!("({base})"),
        ToleranceMode::Symmetric => format!("{base} Â±{:.*}", p, tolerance.upper.abs()),
        ToleranceMode::Bilateral => format!("{base} +{:.*}/-{:.*}", p, tolerance.upper.abs(), p, tolerance.lower.abs()),
        ToleranceMode::Limits => match value {
            Ok(v) => format!("{:.*} / {:.*}", p, v + tolerance.upper, p, v - tolerance.lower),
            Err(_) => format!("{base} +{:.*}/-{:.*}", p, tolerance.upper.abs(), p, tolerance.lower.abs()),
        },
    }
}
#[derive(Clone, Copy, PartialEq, Debug)]
enum SmartKind { Horizontal, Vertical, Aligned }
#[derive(Clone)]
enum SmartGeometry {
    Angle { vertex: ModelVec2, a: ModelVec2, b: ModelVec2, sector: u8 },
    Distance { a: ModelVec2, b: ModelVec2, kind: SmartKind },
    Radius { center: ModelVec2, radius: f64, diameter: bool, angle: f64 },
}
#[derive(Clone)]
enum SmartPick {
    Vertex { view_id: u64, at: ModelVec2 },
    Line { view_id: u64, a: ModelVec2, b: ModelVec2 },
    Circle { view_id: u64, center: ModelVec2, radius: f64, diameter: bool },
    Centre { view_id: u64, at: ModelVec2 },
}
#[derive(Clone)]
struct SmartPending { view_id: u64, geometry: SmartGeometry, from_vertices: bool, pair_sources: Option<(Vec<HlrSourcePath>,Vec<HlrSourcePath>)>, section_pair: Option<([ModelVec2;2],[ModelVec2;2])> }
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
// IDs are unique within one generated projection, not persistent topology IDs.
// Source paths retain the HLR provenance even when multiple projected spans
// consolidate into one selectable 2D entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ProjectedEntityId(u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HlrSourcePath { line: usize, piece: usize, segment: usize }
#[derive(Clone, Debug, PartialEq)]
enum DimensionBinding {
    Single(Vec<HlrSourcePath>),
    Pair(Vec<HlrSourcePath>, Vec<HlrSourcePath>),
    // Section edges have no HLR provenance. Preserve their geometric anchors.
    SectionSingle([ModelVec2; 2]),
    SectionPair([ModelVec2; 2], [ModelVec2; 2]),
    SectionVertices(ModelVec2, ModelVec2),
}
#[derive(Clone)]
struct ProjectedStroke {
    id: ProjectedEntityId,
    sources: Vec<HlrSourcePath>,
    curve: Seg2,
    hidden: bool,
    uncertain: bool,
}
struct DrawingShade { texture: egui::TextureHandle, min: ModelVec2, max: ModelVec2 }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DisplayStyle { ExternalOnly, Visible, VisibleHidden, Shaded, ShadedEdges, ShadedHidden }
impl DisplayStyle {
    fn label(self) -> &'static str {
        match self { Self::ExternalOnly => "External edges only (pending silhouette)", Self::Visible => "Visible edges", Self::VisibleHidden => "Visible and hidden edges", Self::Shaded => "Shaded", Self::ShadedEdges => "Shaded with visible edges", Self::ShadedHidden => "Shaded with hidden edges" }
    }
}
#[derive(Clone)]
struct PlacedView {
    id: u64, frame: ViewFrame, orientation: NamedView, at: ModelVec2,
    show_label: bool, visible: bool, display: DisplayStyle,
    parent: Option<u64>,
    label: String,
    tangent_edges: bool,
    centre_marks: bool,
    centre_lines: bool,
    scale_locked: bool,
    section: Option<SectionSpec>,
}
#[derive(Clone)]
struct SectionSpec { cut_x: f64, tag: String }
impl Default for State {
    fn default() -> Self {
        Self { active: false, drawing_file_path: String::new(), drawing_model_path: String::new(), drawing_session_uid: None, orientation: NamedView::Front, scale_index: 0, placed: Vec::new(), place_mode: false, project_mode: false, project_parent: None, section_mode:false, section_pending:None, section_mesh:Vec::new(), zoom: 1.0, pan: Vec2::ZERO, selected: None, next_view_id: 1, drag_view: None, drag_section_cut: None, show_browser: true, show_view_labels: true, model_revision: None, smart_dimensions_stale: false, projected: HashMap::new(), shaded: HashMap::new(), hlr_debug: HashMap::new(), export_message: None, toast_started_at: 0.0, toast_last_text: String::new(), utilities_tab: false, projection_error: None, edit_view: None, sheet_settings: false, sheets: Vec::new(), active_sheet: 0, add_sheet_dialog: false, new_sheet_name: "Sheet 2".into(), new_sheet_size: 1, new_sheet_landscape: true, sheet_size: 1, landscape: true, show_border: true, show_title: true, title_part: String::new(), title_number: String::new(), title_material: String::new(), title_author: String::new(), revisions: Vec::new(), show_hidden: true, visible_only_debug: false, projection_first_angle: false, undo: Vec::new(), redo: Vec::new(), drag_before: None, annotations: Vec::new(), annotation_tool: AnnotationTool::None,
            annotation_first: None, annotation_text: "HOLE".into(), annotation_count: 4,
            annotation_pitch: 25.0, annotation_diameter: 6.0, smart_first: None, centre_first: None, smart_pending: None, smart_drag: None, annotation_selected: None, tolerance_edit: None }
    }
}
/// Drawing-only undo history: no HLR caches, GPU data or session geometry.
#[derive(Clone)]
struct DrawingSnapshot {
    placed: Vec<PlacedView>,
    next_view_id: u64,
    selected: Option<u64>,
    sheet_size: usize,
    landscape: bool,
    show_border: bool,
    show_title: bool,
    title_part: String,
    title_number: String,
    title_material: String,
    title_author: String,
    revisions: Vec<(String,String,String)>,
    projection_first_angle: bool,
    annotations: Vec<DrawingAnnotation>,
}
impl State {
    fn snapshot(&self) -> DrawingSnapshot {
        DrawingSnapshot {
            placed: self.placed.clone(), next_view_id: self.next_view_id,
            selected: self.selected, sheet_size: self.sheet_size,
            landscape: self.landscape, show_border: self.show_border,
            show_title: self.show_title,
            title_part: self.title_part.clone(), title_number: self.title_number.clone(),
            title_material: self.title_material.clone(), title_author: self.title_author.clone(),
            revisions: self.revisions.clone(), projection_first_angle: self.projection_first_angle, annotations: self.annotations.clone(),
        }
    }
    fn restore(&mut self, s: DrawingSnapshot) {
        self.placed = s.placed;
        self.next_view_id = s.next_view_id;
        self.selected = s.selected;
        self.sheet_size = s.sheet_size;
        self.landscape = s.landscape;
        self.show_border = s.show_border;
        self.show_title = s.show_title;
        self.title_part = s.title_part;
        self.title_number = s.title_number;
        self.title_material = s.title_material;
        self.title_author = s.title_author;
        self.revisions = s.revisions;
        self.projection_first_angle = s.projection_first_angle;
        self.annotations = s.annotations;
        self.drag_view = None;
        self.drag_before = None;
        self.smart_drag = None;
        self.annotation_selected = None; self.tolerance_edit = None;
        self.edit_view = self.edit_view.filter(|id| self.placed.iter().any(|v| v.id == *id));
        self.project_mode = false;
        self.project_parent = None;
    }
    fn store_sheet(&mut self) {
        if self.active_sheet < self.sheets.len() {
            self.sheets[self.active_sheet].drawing = self.snapshot();
        }
    }
    fn switch_sheet(&mut self, index: usize) {
        if index == self.active_sheet || index >= self.sheets.len() { return; }
        self.store_sheet();
        self.active_sheet = index;
        let snapshot = self.sheets[index].drawing.clone();
        self.restore(snapshot);
        self.undo.clear(); self.redo.clear();
        self.annotation_tool = AnnotationTool::None;
    }
    fn add_sheet(&mut self) {
        self.store_sheet();
        let mut new_sheet = self.snapshot();
        new_sheet.placed.clear(); new_sheet.annotations.clear();
        new_sheet.selected = None;
        new_sheet.sheet_size = self.new_sheet_size;
        new_sheet.landscape = self.new_sheet_landscape;
        let name = self.new_sheet_name.trim();
        let name = if name.is_empty() { format!("Sheet {}", self.sheets.len()+1) } else { name.to_owned() };
        self.sheets.push(NamedSheet {name, drawing: new_sheet});
        let index = self.sheets.len()-1;
        self.switch_sheet(index);
        self.add_sheet_dialog = false;
    }
    fn push_undo(&mut self, before: DrawingSnapshot) {
        if !same_drawing(&before, &self.snapshot()) {
            if self.undo.len() == 100 { self.undo.remove(0); }
            self.undo.push(before);
            self.redo.clear();
        }
    }
    fn undo_drawing(&mut self) {
        if let Some(before) = self.undo.pop() {
            self.redo.push(self.snapshot());
            self.restore(before);
        }
    }
    fn redo_drawing(&mut self) {
        if let Some(after) = self.redo.pop() {
            self.undo.push(self.snapshot());
            self.restore(after);
        }
    }
}
fn same_drawing(a: &DrawingSnapshot, b: &DrawingSnapshot) -> bool {
    a.next_view_id == b.next_view_id
        && a.sheet_size == b.sheet_size && a.landscape == b.landscape
        && a.show_border == b.show_border && a.show_title == b.show_title
        && a.projection_first_angle == b.projection_first_angle
        && same_annotations(&a.annotations, &b.annotations)
        && a.placed.len() == b.placed.len()
        && a.placed.iter().zip(&b.placed).all(|(x,y)| {
            x.id == y.id && x.orientation == y.orientation
            && x.at.x == y.at.x && x.at.y == y.at.y
            && x.frame.scale().factor() == y.frame.scale().factor()
            && x.show_label == y.show_label && x.visible == y.visible
            && x.display == y.display && x.parent == y.parent
            && x.label == y.label && x.tangent_edges == y.tangent_edges
            && x.centre_marks == y.centre_marks && x.centre_lines == y.centre_lines
            && x.scale_locked == y.scale_locked
        })
}

fn same_annotations(a: &[DrawingAnnotation], b: &[DrawingAnnotation]) -> bool {
    if a.len() != b.len() { return false; }
    a.iter().zip(b).all(|(x,y)| match (x,y) {
        (DrawingAnnotation::Dimension {a: a1,b: b1,kind: k1,offset: o1},
         DrawingAnnotation::Dimension {a: a2,b: b2,kind: k2,offset: o2}) =>
            a1.x==a2.x && a1.y==a2.y && b1.x==b2.x && b1.y==b2.y && k1==k2 && o1==o2,
        (DrawingAnnotation::Smart {view_id: id1,geometry:g1,offset:o1,text_shift:t1,tolerance:tol1,binding:bind1},
         DrawingAnnotation::Smart {view_id: id2,geometry:g2,offset:o2,text_shift:t2,tolerance:tol2,binding:bind2}) =>
            id1==id2 && smart_geometry_equal(g1,g2) && o1==o2 && t1.x==t2.x && t1.y==t2.y && tol1==tol2 && bind1==bind2,
        (DrawingAnnotation::Note {at:a,text:t,height_mm:h,align:al,bold:bo1,italic:it1,underline:un1},DrawingAnnotation::Note {at:b,text:u,height_mm:k,align:bl,bold:bo2,italic:it2,underline:un2}) => a.x==b.x && a.y==b.y && t==u && h==k && al==bl && bo1==bo2 && it1==it2 && un1==un2,
        (DrawingAnnotation::Leader {target: t1,text_at: p1,text: s1},
         DrawingAnnotation::Leader {target: t2,text_at: p2,text: s2}) =>
            t1.x==t2.x && t1.y==t2.y && p1.x==p2.x && p1.y==p2.y && s1==s2,
        (DrawingAnnotation::CentreMark{view_id:i,center:a,radius:r},DrawingAnnotation::CentreMark{view_id:j,center:b,radius:q}) => i==j && a.x==b.x && a.y==b.y && r==q,
        (DrawingAnnotation::CentreLine{view_id:i,a,b},DrawingAnnotation::CentreLine{view_id:j,a:c,b:d}) => i==j && a.x==c.x && a.y==c.y && b.x==d.x && b.y==d.y,
        _ => false,
    })
}

fn smart_geometry_equal(a: &SmartGeometry, b: &SmartGeometry) -> bool {
    match (a,b) {
        (SmartGeometry::Angle {vertex:v1,a:a1,b:b1,sector:s1},SmartGeometry::Angle {vertex:v2,a:a2,b:b2,sector:s2}) =>
            v1.x==v2.x && v1.y==v2.y && a1.x==a2.x && a1.y==a2.y && b1.x==b2.x && b1.y==b2.y && s1==s2,
        (SmartGeometry::Distance {a:a1,b:b1,kind:k1},SmartGeometry::Distance {a:a2,b:b2,kind:k2}) =>
            a1.x==a2.x && a1.y==a2.y && b1.x==b2.x && b1.y==b2.y && k1==k2,
        (SmartGeometry::Radius {center:c1,radius:r1,diameter:d1,angle:a1},SmartGeometry::Radius {center:c2,radius:r2,diameter:d2,angle:a2}) =>
            c1.x==c2.x && c1.y==c2.y && r1==r2 && d1==d2 && a1==a2,
        _ => false,
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
pub(crate) enum DrawingCommand { NewDrawing, OpenDrawing, SaveDrawing, SaveDrawingAs, Base, Projected, Section, Edit, Sheet, Fit, Delete, Cancel, ToggleBrowser, ExportCsv, ExportDxf, CentreMark, CentreLine, SmartDimension, HoleCallout, PatternCallout, LeaderNote, Note }

pub(crate) fn command_enabled(state: &State, command: DrawingCommand) -> bool {
    match command {
        DrawingCommand::Edit | DrawingCommand::Delete => state.selected.is_some(),
        DrawingCommand::Projected | DrawingCommand::Section => !state.placed.is_empty(),
        DrawingCommand::ExportDxf => !state.placed.is_empty(),
        _ => true,
    }
}

pub(crate) fn command_active(state: &State, command: DrawingCommand) -> bool {
    match command {
        DrawingCommand::Base => state.place_mode,
        DrawingCommand::Projected => state.project_mode,
        DrawingCommand::Section => state.section_mode,
        DrawingCommand::Edit => state.edit_view.is_some(),
        DrawingCommand::Sheet => state.sheet_settings,
        DrawingCommand::ToggleBrowser => state.show_browser,
        DrawingCommand::SmartDimension => state.annotation_tool == AnnotationTool::Smart,
        DrawingCommand::HoleCallout | DrawingCommand::PatternCallout => state.annotation_tool == AnnotationTool::Pattern,
        DrawingCommand::LeaderNote => state.annotation_tool == AnnotationTool::Leader,
        DrawingCommand::Note => state.annotation_tool == AnnotationTool::Note,
        DrawingCommand::CentreMark => state.annotation_tool == AnnotationTool::CentreMark,
        DrawingCommand::CentreLine => state.annotation_tool == AnnotationTool::CentreLine,
        _ => false,
    }
}

pub(crate) fn run_command(state: &mut State, command: DrawingCommand) {
    match command {
        DrawingCommand::NewDrawing | DrawingCommand::OpenDrawing | DrawingCommand::SaveDrawing | DrawingCommand::SaveDrawingAs => {
        if matches!(command, DrawingCommand::NewDrawing) {
            let before = state.snapshot();
            state.placed.clear(); state.annotations.clear(); state.selected = None;
            state.next_view_id = 1; state.drawing_file_path.clear();
            state.push_undo(before);
        }
        if matches!(command, DrawingCommand::OpenDrawing) {
            if let Some(path) = drawing_open_dialog() {
                state.export_message = Some(match load_drawing_file(state, &path) {
                    Ok(()) => format!("Drawing loaded: {}", path.display()),
                    Err(e) => format!("Open failed: {e}"),
                });
            }
        }
        if matches!(command, DrawingCommand::SaveDrawing) {
            let path = if state.drawing_file_path.is_empty() {
                drawing_save_dialog(None)
            } else { Some(std::path::PathBuf::from(&state.drawing_file_path)) };
            if let Some(path) = path {
                // A drawing is a sidecar to its .sc model. Ask for the model only
                // when no reference has yet been recorded.
                let model = if state.drawing_model_path.is_empty() {
                    drawing_model_dialog()
                } else { Some(std::path::PathBuf::from(&state.drawing_model_path)) };
                if let Some(model) = model {
                    state.drawing_model_path = model.to_string_lossy().into_owned();
                    state.export_message = Some(match save_drawing_file(state, &path) {
                        Ok(()) => { state.drawing_file_path = path.to_string_lossy().into_owned(); format!("Drawing saved: {}", path.display()) },
                        Err(e) => format!("Save failed: {e}"),
                    });
                }
            }
        }
        if matches!(command, DrawingCommand::SaveDrawingAs) {
            if let Some(path) = drawing_save_dialog(Some(&state.drawing_file_path)) {
                let model = if state.drawing_model_path.is_empty() {
                    drawing_model_dialog()
                } else { Some(std::path::PathBuf::from(&state.drawing_model_path)) };
                if let Some(model) = model {
                    state.drawing_model_path = model.to_string_lossy().into_owned();
                    state.export_message = Some(match save_drawing_file(state, &path) {
                        Ok(()) => { state.drawing_file_path = path.to_string_lossy().into_owned(); format!("Drawing saved: {}", path.display()) },
                        Err(e) => format!("Save failed: {e}"),
                    });
                }
            }
        }
        }
        DrawingCommand::SmartDimension => { state.annotation_tool = AnnotationTool::Smart; state.smart_first = None; state.smart_pending = None; state.place_mode = false; state.project_mode = false; },
        DrawingCommand::HoleCallout | DrawingCommand::PatternCallout => { state.annotation_tool = AnnotationTool::Pattern; state.annotation_first = None; state.place_mode = false; state.project_mode = false; },
        DrawingCommand::CentreMark | DrawingCommand::CentreLine => {
            state.annotation_tool=if matches!(command,DrawingCommand::CentreMark){AnnotationTool::CentreMark}else{AnnotationTool::CentreLine};
            state.centre_first=None;state.smart_pending=None;state.smart_first=None;
            state.place_mode=false;state.project_mode=false;
        },
        DrawingCommand::Note => { state.annotation_tool = AnnotationTool::Note; state.annotation_first = None; state.place_mode = false; state.project_mode = false; },
        DrawingCommand::LeaderNote => { state.annotation_tool = AnnotationTool::Leader; state.annotation_first = None; state.place_mode = false; state.project_mode = false; },
        DrawingCommand::ExportCsv => {
            state.export_message = Some(match export_hlr_debug(state) { Ok(p) => format!("CSV saved: {}", p.display()), Err(e) => format!("CSV export failed: {e}") });
        }
        DrawingCommand::ExportDxf => {
            state.export_message = Some(match export_hlr_dxf(state) { Ok(p) => format!("DXF saved: {}", p.display()), Err(e) => format!("DXF export failed: {e}") });
        }
        DrawingCommand::Section => {
            state.section_mode=true; state.section_pending=None;
            state.project_mode=false; state.place_mode=false;
            state.annotation_tool=AnnotationTool::None;
            state.selected=state.selected.filter(|id|state.placed.iter().any(|v|v.id==*id && v.section.is_none()))
                .or_else(||state.placed.iter().find(|v|v.section.is_none()).map(|v|v.id));
            // Begin at the centre of the actual model, rather than requiring
            // the first click to locate an arbitrary cutting position.
            state.section_pending=state.selected.and_then(|id|{
                let parent=state.placed.iter().find(|v|v.id==id)?;
                let basis=parent.orientation.basis();
                let mut lo=f64::INFINITY;
                let mut hi=f64::NEG_INFINITY;
                for p in state.section_mesh.iter().flat_map(|tri|tri.iter()) {
                    let x=p.dot(basis.x);
                    lo=lo.min(x);hi=hi.max(x);
                }
                (lo.is_finite()&&hi.is_finite()).then_some((id,(lo+hi)*0.5))
            });
            state.export_message=Some("Section starts at model centre. Drag the blue cutting line across the parent view, then click to place the section.".into());
        }
        DrawingCommand::Projected => {
            // The ribbon command must also work when no view was explicitly selected.
            // Prefer the current selection, otherwise the first base view.
            let selected_exists = state.selected.filter(|id| state.placed.iter().any(|v| v.id == *id));
            state.project_parent = selected_exists.or_else(|| state.placed.iter()
                .find(|v| v.parent.is_none()).map(|v| v.id));
            state.selected = state.project_parent;
            state.project_mode = state.project_parent.is_some();
            state.place_mode = false;
            state.edit_view = None;
            state.sheet_settings = false;
        }
        DrawingCommand::Base => {
            state.project_mode = false;
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
        DrawingCommand::Delete => { let before = state.snapshot(); if let Some(i)=state.annotation_selected.take() {if i<state.annotations.len(){state.annotations.remove(i);state.tolerance_edit=None;}} else {delete_selected(state);} state.push_undo(before); },
        DrawingCommand::Cancel => {
            state.annotation_tool = AnnotationTool::None; state.smart_first = None; state.smart_pending = None; state.centre_first=None;
            state.project_mode = false;
            state.section_mode=false;state.section_pending=None;
            state.project_parent = None;
            state.place_mode = false;
            state.edit_view = None;
            state.sheet_settings = false;
        }
        DrawingCommand::ToggleBrowser => state.show_browser = !state.show_browser,
    }
}

pub fn show(state: &mut State, ui: &mut egui::Ui, session: &solidcraft_engine::Session) {
    // A Drawing workspace State can outlive the currently open design.  Never
    // display views or annotations belonging to a different Session.
    // Session::uid changes on New, Open and Recover, but not on Save or edits.
    if state.drawing_session_uid != Some(session.uid) {
        let active = state.active;
        let show_browser = state.show_browser;
        let utilities_tab = state.utilities_tab;
        *state = State::default();
        state.active = active;
        state.show_browser = show_browser;
        state.utilities_tab = utilities_tab;
        state.drawing_session_uid = Some(session.uid);
    }
    if state.sheets.is_empty() {
        state.sheets.push(NamedSheet {name: "Sheet 1".into(), drawing: state.snapshot()});
    }
    // Sheet setup is a modal window, not a permanent toolbar row.
    if state.add_sheet_dialog {
        let mut open = true;
        let mut add = false;
        egui::Window::new("Add sheet")
            .id(egui::Id::new("sc_add_sheet_dialog"))
            .open(&mut open).collapsible(false).resizable(true)
            .default_width(640.0)
            .show(ui.ctx(), |ui| {
                ui.columns(2, |cols| {
                    cols[0].heading("PAPER SIZE");
                    egui::Grid::new("sc_sheet_sizes").num_columns(3).spacing([12.0,12.0]).show(&mut cols[0], |ui| {
                        for (i,(name,short,long)) in SHEETS.iter().enumerate() {
                            let selected = state.new_sheet_size == i;
                            if ui.add_sized([105.0,66.0],egui::Button::new(format!("â–­  {name}\n{long:.0} Ã— {short:.0} mm")).selected(selected)).clicked() { state.new_sheet_size=i; }
                            if i%3==2 { ui.end_row(); }
                        }
                    });
                    cols[1].heading("PREVIEW & SETTINGS");
                    let (name,short,long)=SHEETS[state.new_sheet_size];
                    let (w,h)=if state.new_sheet_landscape {(long,short)} else {(short,long)};
                    let ratio=w/h;
                    let preview_w=230.0_f32;
                    let preview_h=preview_w/ratio;
                    let (rect,_)=cols[1].allocate_exact_size(Vec2::new(preview_w,preview_h),Sense::hover());
                    let painter=cols[1].painter();
                    painter.rect_filled(rect,0.0,Color32::WHITE);
                    painter.rect_stroke(rect,0.0,Stroke::new(1.0,Color32::DARK_GRAY),StrokeKind::Outside);
                    let inset=rect.shrink(9.0);
                    painter.rect_stroke(inset,0.0,Stroke::new(1.0,Color32::DARK_GRAY),StrokeKind::Inside);
                    let title=Rect::from_min_max(Pos2::new(inset.max.x-80.0,inset.max.y-27.0),inset.max);
                    painter.rect_stroke(title,0.0,Stroke::new(1.0,Color32::DARK_GRAY),StrokeKind::Inside);
                    cols[1].label(format!("{name} Â· {w:.0} Ã— {h:.0} mm"));
                    cols[1].horizontal(|ui| {ui.label("Name");ui.text_edit_singleline(&mut state.new_sheet_name);});
                    cols[1].horizontal(|ui| {ui.label("Orientation");ui.selectable_value(&mut state.new_sheet_landscape,true,"Landscape");ui.selectable_value(&mut state.new_sheet_landscape,false,"Portrait");});
                    cols[1].label("Border: ISO-style Â· Title block: SolidCraft standard");
                });
                ui.separator();
                ui.horizontal(|ui| {if ui.button("Cancel").clicked(){state.add_sheet_dialog=false;} if ui.button("Add sheet").clicked(){add=true;}});
            });
        if !open {state.add_sheet_dialog=false;}
        if add {state.add_sheet();}
    }
    // Drawing history is independent of the Design workspace history.
    // Do not steal Ctrl+Z from an active text editor.
    let text_editing = ui.ctx().egui_wants_keyboard_input();
    if !text_editing {
        let undo = ui.input(|i| i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::Z));
        let redo = ui.input(|i| i.modifiers.command && (i.key_pressed(egui::Key::Y) || (i.modifiers.shift && i.key_pressed(egui::Key::Z))));
        if undo { state.undo_drawing(); }
        else if redo { state.redo_drawing(); }
    }
    let frame_before = state.snapshot();
    let was_dragging = state.drag_before.is_some();
    let history_action = false;
    // Undo/Redo are handled by keyboard shortcuts and the main application chrome.
    // No duplicate toolbar or debug controls above the sheet.
    // Temporary foreground toast: no extra row or layout displacement.
    if let Some(message) = state.export_message.as_ref() {
        if state.toast_last_text != *message {
            state.toast_last_text = message.clone();
            state.toast_started_at = ui.input(|i| i.time);
        }
        let duration = if message.to_ascii_lowercase().contains("failed") { 10.0 } else { 4.0 };
        if ui.input(|i| i.time) - state.toast_started_at < duration {
            let screen = ui.ctx().content_rect();
            let pos = Pos2::new(screen.left() + 16.0, screen.bottom() - 62.0);
            let mut dismiss = false;
            egui::Area::new(egui::Id::new("sc_drawing_toast"))
                .order(egui::Order::Foreground).fixed_pos(pos).show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(message);
                            if ui.small_button("Ã—").clicked() { dismiss = true; }
                        });
                    });
                });
            if dismiss { state.export_message = None; }
            else { ui.ctx().request_repaint_after(std::time::Duration::from_millis(200)); }
        }
    }
    refresh_projection(state, session, ui.ctx());
    // Drawing commands live in the native SolidCraft ribbon, not a second toolbar.
    // The browser and sheet canvas begin immediately below the ribbon.
    // Tool guidance is intentionally not a permanent canvas header.
    // The active tool remains visible in the ribbon.
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        if state.section_mode {state.section_mode=false;state.section_pending=None;} else if state.place_mode || state.project_mode || state.edit_view.is_some() || state.sheet_settings { state.project_mode = false; state.project_parent = None; state.place_mode = false; state.edit_view = None; state.sheet_settings = false; } else { state.selected = None; }
        state.drag_view = None;
        if state.annotation_tool == AnnotationTool::Smart && (state.smart_first.is_some() || state.smart_pending.is_some()) {
            state.smart_first = None; state.smart_pending = None;
        } else {
            state.annotation_tool = AnnotationTool::None; state.annotation_first = None; state.smart_first = None; state.smart_pending = None;
        }
    }
    if !ui.ctx().egui_wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::Delete)) {
        if let Some(index)=state.annotation_selected.take() {
            if index<state.annotations.len() {let before=state.snapshot();state.annotations.remove(index);state.tolerance_edit=None;state.push_undo(before);}
        } else { delete_selected(state); }
    }
    if let Some(index)=state.tolerance_edit {
        let before_tolerance=state.snapshot();
        let mut open=true;
        egui::Window::new("Dimension Properties").open(&mut open).resizable(false).show(ui.ctx(), |ui| {
            if let Some(DrawingAnnotation::Smart {tolerance,geometry,..})=state.annotations.get_mut(index) {
                if let SmartGeometry::Radius {angle,..}=geometry {
                    let mut degrees=angle.to_degrees();
                    ui.horizontal(|ui|{ui.label("Leader angle (Â°)");ui.add(egui::DragValue::new(&mut degrees).speed(1.0).suffix("Â°"));});
                    *angle=degrees.to_radians();
                }
                ui.label("Selected dimension â€” properties");
                ui.separator();
                ui.label("Tolerance display");
                egui::ComboBox::from_id_salt("drawing_tolerance_mode").selected_text(match tolerance.mode {
                    ToleranceMode::None=>"Nominal",ToleranceMode::Symmetric=>"Symmetric Â±",ToleranceMode::Bilateral=>"Bilateral + / -",ToleranceMode::Limits=>"Limits",ToleranceMode::Basic=>"Basic",ToleranceMode::Reference=>"Reference"
                }).show_ui(ui,|ui| {
                    for (mode,name) in [(ToleranceMode::None,"Nominal"),(ToleranceMode::Symmetric,"Symmetric Â±"),(ToleranceMode::Bilateral,"Bilateral + / -"),(ToleranceMode::Limits,"Limits"),(ToleranceMode::Basic,"Basic"),(ToleranceMode::Reference,"Reference")] {ui.selectable_value(&mut tolerance.mode,mode,name);}
                });
                if matches!(tolerance.mode,ToleranceMode::Symmetric|ToleranceMode::Bilateral|ToleranceMode::Limits) {
                    ui.horizontal(|ui| {ui.label("Upper");ui.add(egui::DragValue::new(&mut tolerance.upper).speed(0.01).range(0.0..=1000.0));});
                    if tolerance.mode!=ToleranceMode::Symmetric {ui.horizontal(|ui| {ui.label("Lower");ui.add(egui::DragValue::new(&mut tolerance.lower).speed(0.01).range(0.0..=1000.0));});}
                }
                ui.horizontal(|ui|{ui.label("Precision");ui.add(egui::DragValue::new(&mut tolerance.precision).range(0..=5));});
                ui.label("Nominal value comes from selected geometry.");
            }
        });
        state.push_undo(before_tolerance);
        if !open || index>=state.annotations.len() {state.tolerance_edit=None;}
    }
    if let Some(index)=state.annotation_selected {
        if matches!(state.annotations.get(index),Some(DrawingAnnotation::Note {..}|DrawingAnnotation::Leader {..})) {
            let before=state.snapshot();
            egui::Window::new("Note Properties").resizable(true).show(ui.ctx(),|ui| {
                if let Some(annotation)=state.annotations.get_mut(index) {
                    let text=match annotation {DrawingAnnotation::Note{text,..}|DrawingAnnotation::Leader{text,..}=>text,_=>return};
                    ui.label("Text");
                    // Keep the editor focus and show the I-beam over the text field.
                    // Explicitly request focus on click so the caret is visible
                    // even when the drawing canvas was previously active.
                    let text_response = ui.add(
                        egui::TextEdit::multiline(text)
                            .id_source(("drawing_note_editor", index))
                            .desired_width(280.0)
                            .desired_rows(5)
                    );
                    if text_response.clicked() {
                        text_response.request_focus();
                    }
                    if text_response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                    }
                    if let DrawingAnnotation::Note {height_mm,align,bold,italic,underline,..}=annotation {
                        ui.horizontal(|ui| { ui.checkbox(bold,"Bold"); ui.checkbox(italic,"Italic"); ui.checkbox(underline,"Underline"); });
                        ui.horizontal(|ui| {ui.label("Text height (mm)");ui.add(egui::DragValue::new(height_mm).speed(0.1).range(1.0..=25.0));});
                        ui.horizontal(|ui| {
                            ui.label("Alignment");
                            ui.selectable_value(align,NoteAlign::Left,"Left");
                            ui.selectable_value(align,NoteAlign::Center,"Centre");
                            ui.selectable_value(align,NoteAlign::Right,"Right");
                        });
                    }
                }
            });
            state.push_undo(before);
        }
    }
    // Allocate the remaining drawing editor rectangle explicitly. Nested horizontal
    // layouts can shrink-wrap their height to one row, leaving a tiny canvas.
    let area = ui.available_rect_before_wrap();
    if area.width() < 40.0 || area.height() < 40.0 { return; }
    let browser_width = 0.0_f32;
    // Floating palette: the sheet uses the entire editor viewport, including
    // the region underneath the Browser. Do not reserve a sidebar column.
    if state.show_browser {
        let mut browser_open = state.show_browser;
        egui::Window::new("Browser")
            .id(egui::Id::new("drawing_browser_palette_v0124"))
            .default_pos(Pos2::new(area.min.x + 8.0, area.min.y + 8.0))
            .default_size(Vec2::new(248.0, 330.0))
            .resizable(true)
            .collapsible(true)
            .open(&mut browser_open)
            .show(ui.ctx(), |browser| {
                browser.collapsing("Document Settings", |ui| {
                    if ui.add(egui::Button::new("âš™").small()).on_hover_text("Sheet settings").clicked() {
                        state.sheet_settings = true;
                        state.place_mode = false;
                        state.edit_view = None;
                    }
                    ui.label(format!("{} Â· {}", SHEETS[state.sheet_size].0,
                        if state.landscape { "Landscape" } else { "Portrait" }));
                });
                browser.horizontal(|ui| {
                    ui.strong(format!("Sheets ({})", state.sheets.len()));
                    if ui.button("+ Add sheet").clicked() {
                        state.new_sheet_name = format!("Sheet {}",state.sheets.len()+1);
                        state.new_sheet_size = state.sheet_size;
                        state.new_sheet_landscape = state.landscape;
                        state.add_sheet_dialog = true;
                    }
                });
                let mut switch_to = None;
                for (i,sheet) in state.sheets.iter().enumerate() {
                    let label = format!("{} Â· {}",sheet.name, if i == state.active_sheet {SHEETS[state.sheet_size].0} else {SHEETS[sheet.drawing.sheet_size].0});
                    if browser.selectable_label(i==state.active_sheet,label).clicked() {switch_to=Some(i);}
                }
                if let Some(i)=switch_to {state.switch_sheet(i);}
                browser.collapsing("Sheets", |ui| {
                    ui.collapsing(format!("{} Â· {}", state.sheets[state.active_sheet].name, SHEETS[state.sheet_size].0), |ui| {
                        if state.placed.is_empty() { ui.weak("No views placed"); }
                        let mut edit = None;
                        for (i, view) in state.placed.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut view.visible, "");
                                let name = format!("V{} Â· {:?}{}", i + 1, view.orientation,
                                    if view.parent.is_some() { " (Projected)" } else { "" });
                                let item = ui.selectable_label(state.selected == Some(view.id), name);
                                if item.clicked() { state.selected = Some(view.id); state.place_mode = false; }
                                if item.double_clicked() { edit = Some(view.id); }
                            });
                        }
                        if let Some(id) = edit { state.selected = Some(id); state.edit_view = Some(id); state.sheet_settings = false; }
                    });
                });
                browser.collapsing("Dimensions & Callouts", |ui| {
                    ui.small("Smart Dimension selects HLR model edges, arcs and vertices; measurements are in model units.");
                    for (name, tool) in [("Note", AnnotationTool::Note),("Smart Dimension", AnnotationTool::Smart),
                        ("Leader / Feature", AnnotationTool::Leader), ("Hole Pattern", AnnotationTool::Pattern)] {
                        if ui.selectable_label(state.annotation_tool == tool, name).clicked() {
                            state.annotation_tool = tool;
                            state.annotation_first = None;
                            state.place_mode = false; state.project_mode = false;
                        }
                    }
                    if matches!(state.annotation_tool, AnnotationTool::Leader | AnnotationTool::Pattern | AnnotationTool::Note) {
                        if matches!(state.annotation_tool,AnnotationTool::Leader|AnnotationTool::Note) {
                            ui.label("Feature callout text");
                            ui.add(egui::TextEdit::multiline(&mut state.annotation_text).desired_rows(3));
                        } else {
                            ui.add(egui::DragValue::new(&mut state.annotation_count).range(1..=999).prefix("Count "));
                            ui.add(egui::DragValue::new(&mut state.annotation_diameter).speed(0.1).prefix("Ã˜ mm "));
                            ui.add(egui::DragValue::new(&mut state.annotation_pitch).speed(0.1).prefix("Pitch mm "));
                        }
                    }
                    ui.separator();
                    ui.label("Annotations (click to select; Delete to remove)");
                    for i in 0..state.annotations.len() {
                        let title=match &state.annotations[i] {DrawingAnnotation::Smart{..}=>format!("Dimension {}",i+1),DrawingAnnotation::Leader{text,..}=>format!("Callout {}: {}",i+1,text),DrawingAnnotation::Note{text,..}=>format!("Note {}: {}",i+1,text.lines().next().unwrap_or("")),DrawingAnnotation::Dimension{..}=>format!("Reference {}",i+1),DrawingAnnotation::CentreMark{..}=>format!("Centre mark {}",i+1),DrawingAnnotation::CentreLine{..}=>format!("Centre line {}",i+1)};
                        if ui.selectable_label(state.annotation_selected==Some(i),title).clicked(){state.annotation_selected=Some(i);}
                    }
                    if let Some(index)=state.annotation_selected {
                        if index<state.annotations.len() {
                            ui.horizontal(|ui| {
                                if ui.button("Delete selected").clicked() {let before=state.snapshot();state.annotations.remove(index);state.annotation_selected=None;state.tolerance_edit=None;state.push_undo(before);}
                                else if matches!(state.annotations.get(index),Some(DrawingAnnotation::Smart{..})) && ui.button("Toleranceâ€¦").clicked() {state.tolerance_edit=Some(index);}
                            });
                        }
                    }
                    if ui.button("Remove last annotation").clicked() { let before=state.snapshot();state.annotations.pop();state.annotation_selected=None;state.push_undo(before); }
                    if state.annotation_tool != AnnotationTool::None && ui.button("Finish annotation").clicked() {
                        state.annotation_tool = AnnotationTool::None; state.annotation_first = None; state.smart_first = None; state.smart_pending = None;
                    }
                });
                if let Some(err) = &state.projection_error { browser.colored_label(Color32::LIGHT_RED, err); }
            });
        state.show_browser = browser_open;
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
    // Commit property edits as single entries per frame; drag movement is
    // committed once on mouse release by draw_canvas instead.
    if !history_action && !was_dragging && state.drag_before.is_none() && state.drag_view.is_none() {
        state.push_undo(frame_before);
    }
}

/// Choose an orthographic projection from the parent view and cursor direction.
/// Keep the inherited axis aligned; diagonal placement is deferred until a
/// dedicated isometric projection mapping is available.
fn projected_target(state: &State, paper: Rect, factor: f32, pointer: Pos2) -> Option<(u64, NamedView, ModelVec2)> {
    let parent_id = state.project_parent?;
    let parent = state.placed.iter().find(|v| v.id == parent_id)?;
    let parent_px = Pos2::new(paper.min.x + parent.at.x as f32 * factor, paper.max.y - parent.at.y as f32 * factor);
    let delta = pointer - parent_px;
    if delta.length() < 15.0 { return None; }
    // Dominant-axis selection with a central dead zone: slight sideways
    // motion must not turn Top -> Back into a side projection.
    let horizontal = delta.x.abs() > delta.y.abs() * 1.35;
    let vertical = delta.y.abs() > delta.x.abs() * 1.35;
    if !horizontal && !vertical { return None; }
    let positive = if horizontal { delta.x > 0.0 } else { delta.y < 0.0 };
    let invert = state.projection_first_angle;
    let direction = if invert { !positive } else { positive };
    // V0.1.25a: Project relative to the *selected parent orientation*, not
    // a fixed Front view. Both sheet axes must work for every orthographic
    // parent, including views that are themselves projected children.
    // `direction` is the third-angle sheet direction (reversed for first-angle).
    let orientation = match (parent.orientation, horizontal, direction) {
        (NamedView::Front, true, true) => NamedView::Right,
        (NamedView::Front, true, false) => NamedView::Left,
        (NamedView::Front, false, true) => NamedView::Top,
        (NamedView::Front, false, false) => NamedView::Bottom,
        (NamedView::Back, true, true) => NamedView::Left,
        (NamedView::Back, true, false) => NamedView::Right,
        (NamedView::Back, false, true) => NamedView::Top,
        (NamedView::Back, false, false) => NamedView::Bottom,
        (NamedView::Top, true, true) => NamedView::Right,
        (NamedView::Top, true, false) => NamedView::Left,
        (NamedView::Top, false, true) => NamedView::Back,
        (NamedView::Top, false, false) => NamedView::Front,
        (NamedView::Bottom, true, true) => NamedView::Right,
        (NamedView::Bottom, true, false) => NamedView::Left,
        (NamedView::Bottom, false, true) => NamedView::Front,
        (NamedView::Bottom, false, false) => NamedView::Back,
        (NamedView::Right, true, true) => NamedView::Back,
        (NamedView::Right, true, false) => NamedView::Front,
        (NamedView::Right, false, true) => NamedView::Top,
        (NamedView::Right, false, false) => NamedView::Bottom,
        (NamedView::Left, true, true) => NamedView::Front,
        (NamedView::Left, true, false) => NamedView::Back,
        (NamedView::Left, false, true) => NamedView::Top,
        (NamedView::Left, false, false) => NamedView::Bottom,
        _ => return None,
    };
    // The anchor `at` is the HLR model origin, NOT the visual centre of a view.
    // Different orientations have different projected geometry offsets. Locking
    // parent.at.x/y therefore does not align the actual projected outlines.
    let mut at = if horizontal { ModelVec2::new(parent.at.x + delta.x as f64 / factor as f64, parent.at.y) }
                 else { ModelVec2::new(parent.at.x, parent.at.y - delta.y as f64 / factor as f64) };
    let parent_centre = view_bounds(state, parent, paper, factor).center();
    if let Some((min, max)) = projected_model_bounds(state, orientation) {
        let ratio = parent.frame.scale().factor() as f64;
        let projected_centre_x = paper.min.x + (at.x as f32 + ((min.x + max.x) * 0.5 * ratio) as f32) * factor;
        let projected_centre_y = paper.max.y - (at.y as f32 + ((min.y + max.y) * 0.5 * ratio) as f32) * factor;
        if horizontal {
            // Horizontal projections share the parent's visible vertical centre.
            at.y += ((projected_centre_y - parent_centre.y) / factor) as f64;
        } else {
            // Vertical projections share the parent's visible horizontal centre.
            at.x += ((parent_centre.x - projected_centre_x) / factor) as f64;
        }
    }
    Some((parent_id, orientation, at))
}

/// Bounds of HLR geometry in its own projected model coordinates.
/// Used to align visible outlines, not the arbitrary local view origin.
fn projected_model_bounds(state: &State, orientation: NamedView) -> Option<(ModelVec2, ModelVec2)> {
    let strokes = state.projected.get(&orientation)?;
    let mut min = ModelVec2::new(f64::INFINITY, f64::INFINITY);
    let mut max = ModelVec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for stroke in strokes {
        for point in adaptive_curve_points(&stroke.curve, 1.0, 0.20) {
            if !point.x.is_finite() || !point.y.is_finite() { continue; }
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.y);
            max.x = max.x.max(point.x);
            max.y = max.y.max(point.y);
        }
    }
    (min.x.is_finite() && min.y.is_finite() && max.x.is_finite() && max.y.is_finite())
        .then_some((min, max))
}

fn text_revision(painter:&egui::Painter,x:f32,y:f32,px:f32,rev:&str,date:&str,description:&str){
    painter.text(Pos2::new(x,y),egui::Align2::LEFT_CENTER,
        format!("{rev:<5}  {date:<12}  {description}"),
        egui::FontId::proportional(px),Color32::BLACK);
}


fn section_letter(index:usize)->String {
    let mut n=index;let mut letters=String::new();
    loop {letters.insert(0,(b'A'+(n%26) as u8) as char);if n<26 {break;}n=n/26-1;}
    letters
}
/// Intersect each model mesh triangle with the plane normal to the parent view X axis.
/// Result coordinates: section horizontal = viewing depth; vertical = parent view Y.
/// Display-mesh precision; not exact B-rep section topology.
fn section_segments(mesh:&[[Vec3;3]],orientation:NamedView,cut_x:f64)->Vec<[ModelVec2;2]> {
    let basis=orientation.basis();
    let x=basis.x;let y=basis.y;let depth=basis.y.cross(basis.x);
    let mut out=Vec::new();
    for tri in mesh {
        let mut hits:Vec<Vec3>=Vec::new();
        for i in 0..3 {
            let a=tri[i];let b=tri[(i+1)%3];
            let da=a.dot(x)-cut_x;let db=b.dot(x)-cut_x;
            if da.abs()<1e-8 && db.abs()<1e-8 {continue;}
            if (da<=0.0 && db>=0.0)||(da>=0.0 && db<=0.0) {
                let t=da/(da-db);
                if t.is_finite() && (0.0..=1.0).contains(&t) {
                    let p=a+(b-a)*t;
                    if !hits.iter().any(|q|{ let d = p - *q; d.x*d.x + d.y*d.y + d.z*d.z < 1e-12 }){hits.push(p);}
                }
            }
        }
        if hits.len()==2 {
            let a=ModelVec2::new(hits[0].dot(depth),hits[0].dot(y));
            let b=ModelVec2::new(hits[1].dot(depth),hits[1].dot(y));
            if (a.x-b.x).hypot(a.y-b.y)>1e-6 {out.push([a,b]);}
        }
    }
    // Adjacent triangles can contribute the same cut edge (notably when the
    // cutting plane passes through mesh vertices). Keep one geometric edge.
    // Duplicates otherwise corrupt even/odd hatch parity.
    let mut unique = Vec::with_capacity(out.len());
    let mut seen = std::collections::HashSet::new();
    let key = |p: ModelVec2| ((p.x * 1e5).round() as i64, (p.y * 1e5).round() as i64);
    for edge in out {
        let a = key(edge[0]);
        let b = key(edge[1]);
        if a == b { continue; }
        let ordered = if a <= b { (a, b) } else { (b, a) };
        if seen.insert(ordered) { unique.push(edge); }
    }
    merge_collinear_section_segments(unique)
}

/// Merge mesh-triangle intersection spans into continuous straight section edges.
/// Only degree-two endpoints are eligible: corners, branches, and coincident
/// duplicate edges must remain separate. This also makes Smart Dimension pick
/// the full wall instead of an arbitrary tessellation segment.
fn merge_collinear_section_segments(segments: Vec<[ModelVec2; 2]>) -> Vec<[ModelVec2; 2]> {
    type Key = (i64, i64);
    const SNAP: f64 = 1e-5;
    const ANGLE_TOL: f64 = 1e-6;
    fn key(p: ModelVec2) -> Key {
        ((p.x / SNAP).round() as i64, (p.y / SNAP).round() as i64)
    }
    fn straight(a: ModelVec2, b: ModelVec2, c: ModelVec2) -> bool {
        let ux = b.x - a.x;
        let uy = b.y - a.y;
        let vx = c.x - b.x;
        let vy = c.y - b.y;
        let lu = ux.hypot(uy);
        let lv = vx.hypot(vy);
        lu > SNAP && lv > SNAP
            && (ux * vy - uy * vx).abs() <= ANGLE_TOL * lu * lv
            && ux * vx + uy * vy > 0.0
    }

    let mut adjacency: HashMap<Key, Vec<usize>> = HashMap::new();
    for (i, [a, b]) in segments.iter().enumerate() {
        adjacency.entry(key(*a)).or_default().push(i);
        adjacency.entry(key(*b)).or_default().push(i);
    }
    let mut visited = vec![false; segments.len()];
    let mut merged = Vec::with_capacity(segments.len());
    for start in 0..segments.len() {
        if visited[start] { continue; }
        visited[start] = true;
        let [mut a, mut b] = segments[start];
        // Extend independently at both ends. A degree-two node can be passed
        // only when its two incident spans continue in the same direction.
        for end in 0..2 {
            loop {
                let (tip, inner) = if end == 0 { (a, b) } else { (b, a) };
                let Some(edges) = adjacency.get(&key(tip)) else { break; };
                if edges.len() != 2 { break; }
                let Some(&next) = edges.iter().find(|&&i| !visited[i]) else { break; };
                let [p, q] = segments[next];
                let other = if key(p) == key(tip) { q } else if key(q) == key(tip) { p } else { break; };
                if !straight(inner, tip, other) { break; }
                visited[next] = true;
                if end == 0 { a = other; } else { b = other; }
            }
        }
        merged.push([a, b]);
    }
    merged
}

/// Intersect one 45-degree hatch line with the section contours.
/// Returns material spans only, respecting nested contours and holes.
/// A half-open crossing rule avoids double counting shared polygon vertices.
fn section_hatch_spans(segments: &[[ModelVec2; 2]], c: f64) -> Vec<[ModelVec2; 2]> {
    let mut xs = Vec::new();
    for [a, b] in segments {
        let da = a.x + a.y - c;
        let db = b.x + b.y - c;
        if (da <= 0.0 && db > 0.0) || (db <= 0.0 && da > 0.0) {
            let t = da / (da - db);
            let x = a.x + (b.x - a.x) * t;
            if x.is_finite() { xs.push(x); }
        }
    }
    xs.sort_by(f64::total_cmp);
    // Coincident mesh intersections are common at tessellation seams. Collapse
    // near-equal crossings before pairing; they describe one boundary, not a
    // microscopic region of material. An odd count is invalid: fail closed.
    let mut distinct: Vec<f64> = Vec::with_capacity(xs.len());
    for x in xs {
        if distinct.last().is_none_or(|last| (x - *last).abs() > 1e-5) {
            distinct.push(x);
        }
    }
    if distinct.len() % 2 != 0 { return Vec::new(); }
    distinct.chunks_exact(2).filter_map(|pair| {
        ((pair[1] - pair[0]).abs() > 1e-5).then_some([
            ModelVec2::new(pair[0], c - pair[0]),
            ModelVec2::new(pair[1], c - pair[1]),
        ])
    }).collect()
}

/// Draw a 45-degree section hatch at a constant 3 mm perpendicular spacing
/// on the *paper*, independent of view scale and canvas zoom.
fn draw_section_mesh(painter: &egui::Painter, origin: Pos2, screen_scale: f32,
                     view_scale: f64, segments: &[[ModelVec2; 2]]) {
    if segments.is_empty() || screen_scale.abs() < 1e-6 || view_scale <= 0.0 { return; }
    let point = |p: ModelVec2| Pos2::new(origin.x + p.x as f32 * screen_scale,
                                        origin.y - p.y as f32 * screen_scale);
    let mut min_sum = f64::INFINITY;
    let mut max_sum = f64::NEG_INFINITY;
    for [a, b] in segments {
        for p in [a, b] {
            min_sum = min_sum.min(p.x + p.y);
            max_sum = max_sum.max(p.x + p.y);
        }
    }
    // x+y=c has perpendicular distance |delta c|/sqrt(2).
    // view_scale converts model mm to sheet mm (e.g. 1:2 -> 0.5).
    let step = 3.0 * std::f64::consts::SQRT_2 / view_scale;
    let first = (min_sum / step).ceil() as i64;
    let last = (max_sum / step).floor() as i64;
    if last >= first && last - first < 2000 {
        for k in first..=last {
            for [a, b] in section_hatch_spans(segments, k as f64 * step) {
                painter.line_segment([point(a), point(b)],
                    Stroke::new(0.7, Color32::from_gray(135)));
            }
        }
    }
    // Contours remain visible and available to dimension picking.
    for [a, b] in segments {
        painter.line_segment([point(*a), point(*b)], Stroke::new(1.5, Color32::BLACK));
    }
}

#[cfg(test)]
mod section_hatch_regression_tests {
    use super::*;
    fn rect(x0:f64,y0:f64,x1:f64,y1:f64) -> Vec<[ModelVec2;2]> {
        let p = [ModelVec2::new(x0,y0),ModelVec2::new(x1,y0),
                 ModelVec2::new(x1,y1),ModelVec2::new(x0,y1)];
        (0..4).map(|i| [p[i],p[(i+1)%4]]).collect()
    }
    #[test]
    fn hatch_inside_solid_rectangle() {
        let edges=rect(0.0,0.0,10.0,10.0);
        let spans=section_hatch_spans(&edges,10.0);
        assert_eq!(spans.len(),1);
        assert!((spans[0][0].x-spans[0][1].x).abs()>9.9);
    }
    #[test]
    fn hatch_skips_inner_void() {
        let mut edges=rect(0.0,0.0,20.0,20.0);
        edges.extend(rect(7.0,7.0,13.0,13.0));
        let spans=section_hatch_spans(&edges,20.0);
        assert_eq!(spans.len(),2);
        for [a,b] in spans { assert!(!(a.x<10.0 && b.x>10.0)); }
    }
    #[test]
    fn invalid_odd_crossings_are_not_filled() {
        let edges=vec![[ModelVec2::new(0.0,0.0),ModelVec2::new(10.0,0.0)]];
        assert!(section_hatch_spans(&edges,5.0).is_empty());
    }
}

// Drawing typography is specified in paper millimetres, not fixed UI pixels.
// The sheet factor includes fit-to-window and user zoom, so labels stay in
// proportion to A4/A3/A2/A1 sheets and scale with the page.
fn sheet_font(mm: f32, factor: f32) -> egui::FontId {
    egui::FontId::proportional((mm * factor).max(0.5))
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
        let px = (2.8 * factor).max(0.5);
        let left = title.left() + 3.0 * factor;
        let line_h = 6.5 * factor;
        let text = |y:f32, value:&str| {
            painter.text(Pos2::new(left,y),egui::Align2::LEFT_CENTER,value,
                egui::FontId::proportional(px),Color32::BLACK);
        };
        text(title.top()+5.0*factor, &format!("TITLE: {}",state.title_part));
        text(title.top()+5.0*factor+line_h,&format!("DWG: {}    {}",state.title_number,SHEETS[state.sheet_size].0));
        text(title.top()+5.0*factor+2.0*line_h,&format!("MATERIAL: {}",state.title_material));
        text(title.top()+5.0*factor+3.0*line_h,&format!("DRAWN: {}",state.title_author));
        if !state.revisions.is_empty() {
            let rev_w = 120.0 * factor;
            let rev_h = (state.revisions.len().min(6) as f32 + 1.0)*6.0*factor;
            let rev_rect = Rect::from_min_max(
                Pos2::new(title.right()-rev_w,title.top()-rev_h),Pos2::new(title.right(),title.top()));
            painter.rect_stroke(rev_rect,0.0,Stroke::new(0.7,Color32::DARK_GRAY),StrokeKind::Inside);
            let header = Pos2::new(rev_rect.left()+3.0*factor,rev_rect.top()+3.0*factor);
            painter.text(header,egui::Align2::LEFT_CENTER,"REV     DATE           DESCRIPTION",
                egui::FontId::proportional(px),Color32::BLACK);
            for (i,(rev,date,description)) in state.revisions.iter().rev().take(6).enumerate() {
                let y=rev_rect.top()+(i as f32+1.5)*6.0*factor;
                text_revision(&painter,rev_rect.left()+3.0*factor,y,px,rev,date,description);
            }
        }
    }
    // The title block is a sheet entity: double-click to edit its fields.
    // Handle this before view/annotation hit-testing so it cannot select a view.
    if state.show_title && response.double_clicked()
        && response.interact_pointer_pos().is_some_and(|pos| title.contains(pos))
        && !state.place_mode && !state.project_mode
    {
        state.sheet_settings = true;
        state.edit_view = None;
        state.annotation_tool = AnnotationTool::None;
        state.annotation_selected = None;
        state.tolerance_edit = None;
    }
    // Smart Dimension: model geometry wins where it overlaps an annotation.
    // Annotation text remains directly editable, including while dimensioning.
    let annotation_priority_hit = |state: &State, pos: Pos2| -> Option<(usize, bool)> {
        let hit = hit_annotation(state, paper, factor, pos)?;
        if state.annotation_tool == AnnotationTool::Smart && !hit.1
            && pick_smart_geometry(state, paper, factor, pos).is_some() {
            return None;
        }
        Some(hit)
    };
    if response.clicked() && !(state.show_title && response.interact_pointer_pos().is_some_and(|pos| title.contains(pos))) && !state.place_mode && !state.project_mode && !matches!(state.annotation_tool,AnnotationTool::CentreMark|AnnotationTool::CentreLine) {
        if let Some((index, _)) = response.interact_pointer_pos()
            .and_then(|pos| annotation_priority_hit(state, pos)) {
            state.annotation_tool = AnnotationTool::None;
            state.annotation_first = None;
            state.smart_first = None;
            state.smart_pending = None;
            state.annotation_selected = Some(index);
            state.tolerance_edit = if matches!(state.annotations.get(index), Some(DrawingAnnotation::Smart { .. })) {
                Some(index)
            } else { None };
        }
    }
    if response.drag_started_by(egui::PointerButton::Primary) && !state.place_mode && !state.project_mode && !matches!(state.annotation_tool,AnnotationTool::CentreMark|AnnotationTool::CentreLine) {
        if let Some((index, _)) = ui.input(|i| i.pointer.press_origin())
            .and_then(|pos| annotation_priority_hit(state, pos)) {
            state.annotation_tool = AnnotationTool::None;
            state.annotation_first = None;
            state.smart_first = None;
            state.smart_pending = None;
            state.annotation_selected = Some(index);
        }
    }
    if response.clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            if state.annotation_selected.is_some_and(|index|
                annotation_priority_hit(state, pointer).is_some_and(|(hit, _)| hit == index)) {
                // Handled as annotation selection above; don't place a dimension.
            } else if matches!(state.annotation_tool,AnnotationTool::CentreMark|AnnotationTool::CentreLine) && border.contains(pointer) {
                match state.annotation_tool {
                    AnnotationTool::CentreMark => {
                        if let Some(SmartPick::Circle{view_id,center,radius,..})=pick_smart_geometry(state,paper,factor,pointer) {
                            let before=state.snapshot();
                            state.annotations.push(DrawingAnnotation::CentreMark{view_id,center,radius});
                            state.push_undo(before);
                        }
                    }
                    AnnotationTool::CentreLine => {
                        if let Some(SmartPick::Line{view_id,a,b})=pick_smart_geometry(state,paper,factor,pointer) {
                            if let Some((first_id,c,d))=state.centre_first.take() {
                                if first_id==view_id {
                                    if let Some((start,end))=centre_between_lines(c,d,a,b) {
                                        let before=state.snapshot();
                                        state.annotations.push(DrawingAnnotation::CentreLine{view_id,a:start,b:end});
                                        state.push_undo(before);
                                    } else {state.export_message=Some("Centre line requires two distinct parallel edges".into());}
                                } else {state.centre_first=Some((view_id,a,b));}
                            } else {state.centre_first=Some((view_id,a,b));}
                        }
                    }
                    _=>{}
                }
            } else if state.annotation_tool == AnnotationTool::Smart && border.contains(pointer) {
                if let Some(pending) = state.smart_pending.take() {
                    // A second edge selection takes precedence over placing the first edge length.
                    // Clicking clear sheet space still places the previewed dimension.
                    let second = pick_smart_geometry(state, paper, factor, pointer);
                    let paired = match (&pending.geometry, second) {
                        (SmartGeometry::Distance {a,b,kind:SmartKind::Aligned}, Some(SmartPick::Line {view_id,a:c_a,b:c_b})) if view_id == pending.view_id && !pending.from_vertices => {
                            smart_pair_lines(*a,*b,c_a,c_b).map(|geometry| {
                                let pair_sources=stroke_sources_for_line(state,view_id,*a,*b)
                                    .zip(stroke_sources_for_line(state,view_id,c_a,c_b));
                                let section_pair=state.placed.iter().find(|v|v.id==view_id)
                                    .and_then(|v|v.section.as_ref()).map(|_|([*a,*b],[c_a,c_b]));
                                SmartPending {view_id,geometry,from_vertices:false,pair_sources,section_pair}
                            })
                        },
                        _ => None,
                    };
                    if let Some(next) = paired {
                        state.smart_pending = Some(next);
                        state.smart_first = None;
                    } else {
                    let mut pending = pending;
                    if pending.from_vertices {
                        if let SmartGeometry::Distance {a,b,kind} = &mut pending.geometry {
                            if let Some(view)=state.placed.iter().find(|v|v.id==pending.view_id) {
                                let p=smart_to_screen(view,*a,paper,factor);
                                let q=smart_to_screen(view,*b,paper,factor);
                                let middle=p+(q-p)*0.5;
                                let displacement=pointer-middle;
                                *kind=if displacement.y.abs()>displacement.x.abs()*1.6 {SmartKind::Horizontal}
                                    else if displacement.x.abs()>displacement.y.abs()*1.6 {SmartKind::Vertical}
                                    else {SmartKind::Aligned};
                            }
                        }
                    }
                    if let Some(view)=state.placed.iter().find(|v|v.id==pending.view_id) {
                        smart_set_angle_sector(&mut pending.geometry,view,paper,factor,pointer);
                    }
                    smart_set_radius_angle(&mut pending.geometry, state, pending.view_id, paper, factor, pointer);
                    let offset = smart_offset(state, &pending, paper, factor, pointer);
                    let text_shift=ModelVec2::ZERO;
                    let binding = if let Some((first,second))=pending.pair_sources.clone() {
                        Some(DimensionBinding::Pair(first,second))
                    } else {
                        match &pending.geometry {
                            SmartGeometry::Distance {a,b,kind:SmartKind::Aligned} if !pending.from_vertices =>
                                stroke_sources_for_line(state,pending.view_id,*a,*b).map(DimensionBinding::Single),
                            _=>None,
                        }
                    };
                    let section_binding = state.placed.iter().find(|v|v.id==pending.view_id)
                        .is_some_and(|v|v.section.is_some());
                    let binding = if section_binding {
                        if let Some((first,second))=pending.section_pair {
                            Some(DimensionBinding::SectionPair(first,second))
                        } else {
                            match &pending.geometry {
                                SmartGeometry::Distance{a,b,..} if pending.from_vertices =>
                                    Some(DimensionBinding::SectionVertices(*a,*b)),
                                SmartGeometry::Distance{a,b,..} =>
                                    Some(DimensionBinding::SectionSingle([*a,*b])),
                                _ => None,
                            }
                        }
                    } else {binding};
                    state.annotations.push(DrawingAnnotation::Smart { view_id: pending.view_id,
                        geometry: pending.geometry, offset, text_shift, tolerance: DimensionTolerance::default(), binding });
                    state.smart_first = None;
                    }
                } else if let Some(pick) = pick_smart_geometry(state, paper, factor, pointer) {
                    match pick {
                        SmartPick::Line {view_id,a,b} => {
                            state.smart_pending = Some(SmartPending {view_id, geometry: SmartGeometry::Distance {a,b,kind:SmartKind::Aligned}, from_vertices:false, pair_sources:None,section_pair:None });
                            state.smart_first = None;
                        }
                        SmartPick::Circle {view_id,center,radius,diameter} => {
                            state.smart_pending = Some(SmartPending {view_id, geometry: SmartGeometry::Radius {center,radius,diameter,angle:0.0}, from_vertices:false, pair_sources:None,section_pair:None });
                            state.smart_first = None;
                        }
                        SmartPick::Centre {view_id,at} | SmartPick::Vertex {view_id,at} => {
                            if let Some(SmartPick::Vertex {view_id:first_id,at:first}) = state.smart_first.take() {
                                if first_id == view_id && (at.x-first.x).hypot(at.y-first.y)>1e-6 {
                                    state.smart_pending = Some(SmartPending {view_id,geometry:SmartGeometry::Distance {a:first,b:at,kind:SmartKind::Aligned}, from_vertices:true, pair_sources:None,section_pair:None });
                                } else { state.smart_first = Some(SmartPick::Vertex {view_id,at}); }
                            } else { state.smart_first = Some(SmartPick::Vertex {view_id,at}); }
                        }
                    }
                }
            } else if state.annotation_tool == AnnotationTool::Note && border.contains(pointer) {
                let at=ModelVec2::new(((pointer.x-paper.min.x)/factor) as f64,((paper.max.y-pointer.y)/factor) as f64);
                let before=state.snapshot();
                state.annotations.push(DrawingAnnotation::Note {at,text:state.annotation_text.clone(),height_mm:3.5,align:NoteAlign::Left,bold:false,italic:false,underline:false});
                state.annotation_selected=Some(state.annotations.len()-1);
                state.annotation_tool=AnnotationTool::None;
                state.push_undo(before);
            } else if state.annotation_tool != AnnotationTool::None && border.contains(pointer) {
                let point = ModelVec2::new(((pointer.x - paper.min.x) / factor) as f64,
                                           ((paper.max.y - pointer.y) / factor) as f64);
                if let Some(first) = state.annotation_first.take() {
                    match state.annotation_tool {
                        AnnotationTool::Horizontal | AnnotationTool::Vertical | AnnotationTool::Aligned => {
                            let kind = state.annotation_tool;
                            let offset = 10.0;
                            state.annotations.push(DrawingAnnotation::Dimension { a: first, b: point, kind, offset });
                        }
                        AnnotationTool::Leader | AnnotationTool::Pattern => {
                            let text = if state.annotation_tool == AnnotationTool::Pattern {
                                format!("{} Ã— Ã˜{:.2}  PITCH {:.2} mm (REF)", state.annotation_count,
                                    state.annotation_diameter, state.annotation_pitch)
                            } else { state.annotation_text.clone() };
                            state.annotations.push(DrawingAnnotation::Leader { target: first, text_at: point, text });
                        }
                        AnnotationTool::None | AnnotationTool::Smart | AnnotationTool::Note | AnnotationTool::CentreMark | AnnotationTool::CentreLine => {}
                    }
                } else { state.annotation_first = Some(point); }
            } else if state.section_mode && border.contains(pointer) {
                if let Some((parent_id,cut_x))=state.section_pending.take() {
                    // Clicking the parent view repositions the cut; clicking
                    // elsewhere places the resulting section.
                    if let Some(parent)=state.placed.iter().find(|v|v.id==parent_id) {
                        if view_bounds(state,parent,paper,factor).contains(pointer) {
                            let anchor=sheet_to_screen(parent.at,paper,factor);
                            let ratio=parent.frame.scale().factor() as f32*factor;
                            if ratio.abs()>1e-6 {
                                state.section_pending=Some((parent_id,cut_x+(pointer.x-(anchor.x+cut_x as f32*ratio)) as f64/ratio as f64));
                            }
                        }
                    }
                    if state.section_pending.is_some() {
                        // Still editing the cutting position; do not place a view.
                    } else if let Some(parent)=state.placed.iter().find(|v|v.id==parent_id).cloned() {
                        let at=ModelVec2::new(((pointer.x-paper.min.x)/factor) as f64,((paper.max.y-pointer.y)/factor) as f64);
                        let id=state.next_view_id;
                        let tag=section_letter(state.placed.iter().filter(|v|v.section.is_some()).count());
                        let before=state.snapshot();
                        if let Ok(frame)=ViewFrame::new(parent.orientation.basis(),parent.frame.scale(),ModelVec2::ZERO,at) {
                            state.next_view_id+=1;
                            state.placed.push(PlacedView{id,frame,orientation:parent.orientation,at,
                                show_label:true,visible:true,display:DisplayStyle::Visible,parent:Some(parent_id),
                                label:format!("SECTION {tag}-{tag}"),tangent_edges:false,centre_marks:false,
                                centre_lines:false,scale_locked:true,section:Some(SectionSpec{cut_x,tag})});
                            state.push_undo(before);
                        }
                    }
                    if state.section_pending.is_none() {state.section_mode=false;}
                } else if let Some(parent)=state.selected.and_then(|id|state.placed.iter().find(|v|v.id==id && v.section.is_none())) {
                    let ratio=parent.frame.scale().factor();
                    let anchor=sheet_to_screen(parent.at,paper,factor);
                    let x=(pointer.x-anchor.x) as f64/(ratio*factor as f64);
                    state.section_pending=Some((parent.id,x));
                } else {state.export_message=Some("Select a base or projected view before placing a section".into());}
            } else if state.project_mode && border.contains(pointer) {
                if let Some((parent_id, orientation, at)) = projected_target(state, paper, factor, pointer) {
                    if let Some(parent) = state.placed.iter().find(|v| v.id == parent_id) {
                        let scale = parent.frame.scale();
                        if let Ok(frame) = ViewFrame::new(orientation.basis(), scale, ModelVec2::ZERO, at) {
                            let id = state.next_view_id;
                            state.next_view_id += 1;
                            state.placed.push(PlacedView { id, frame, orientation, at, show_label: true, visible: true, display: parent.display, parent: Some(parent_id), label: format!("{:?}", orientation), tangent_edges: false, centre_marks: false, centre_lines: false, scale_locked: true, section:None });
                            // Keep projecting from the original parent until Esc/Cancel.
                            // Do not select the newly placed child as the next source.
                            state.selected = Some(parent_id);
                        }
                    }
                }
            } else if state.place_mode && border.contains(pointer) {
                let at = ModelVec2::new(((pointer.x - paper.min.x) / factor) as f64,
                                         ((paper.max.y - pointer.y) / factor) as f64);
                let (a,b) = SCALES[state.scale_index];
                if let Ok(scale) = Scale::new(a,b) {
                    if let Ok(frame) = ViewFrame::new(state.orientation.basis(), scale, ModelVec2::ZERO, at) {
                        let id = state.next_view_id;
                        state.next_view_id += 1;
                        state.placed.push(PlacedView { id, frame, orientation: state.orientation, at, show_label: true, visible: true, display: if state.show_hidden { DisplayStyle::VisibleHidden } else { DisplayStyle::Visible }, parent: None, label: format!("{:?}", state.orientation), tangent_edges: false, centre_marks: false, centre_lines: false, scale_locked: false, section:None });
                        state.selected = Some(id);
                        state.place_mode = false;
                    }
                }
            } else if !state.place_mode && !state.project_mode {
                if let Some((index,_))=hit_annotation(state,paper,factor,pointer) {
                    state.annotation_selected=Some(index);
                    // Single click opens the non-modal properties palette; double click
                    // keeps the same behaviour for existing users.
                    if matches!(state.annotations.get(index),Some(DrawingAnnotation::Smart {..})) { state.tolerance_edit=Some(index); }
                } else { state.annotation_selected=None; state.tolerance_edit=None; state.selected = hit_view(state, paper, factor, pointer); }
                if response.double_clicked() && state.annotation_selected.is_none() {
                    state.edit_view = state.selected; state.sheet_settings = false;
                    if let Some(v) = state.placed.iter().find(|v| Some(v.id) == state.edit_view) {
                        state.orientation = v.orientation; state.scale_index = scale_index(v.frame.scale());
                    }
                }
            }
        }
    }
    if response.drag_started_by(egui::PointerButton::Primary) && state.annotation_tool == AnnotationTool::None {
        state.drag_section_cut=None;
        if let Some(pos)=ui.input(|i|i.pointer.press_origin()) {
            let candidate=if state.section_mode {
                state.section_pending
            } else {
                state.placed.iter().filter_map(|v|v.section.as_ref().and_then(|spec|v.parent.map(|id|(id,spec.cut_x)))).find(|(id,_)| {
                    state.placed.iter().any(|p|p.id==*id)
                })
            };
            if let Some((parent_id,cut_x))=candidate {
                if let Some(parent)=state.placed.iter().find(|v|v.id==parent_id) {
                    let anchor=sheet_to_screen(parent.at,paper,factor);
                    let scale=parent.frame.scale().factor() as f32*factor;
                    let line_x=anchor.x+cut_x as f32*scale;
                    let bounds=view_bounds(state,parent,paper,factor);
                    if (pos.x-line_x).abs()<=12.0 && pos.y>=bounds.top()-15.0 && pos.y<=bounds.bottom()+15.0 {
                        state.drag_section_cut=Some(parent_id);
                        state.drag_before=Some(state.snapshot());
                    }
                }
            }
        }
        // Resolve the hit at pointer press, not after the pointer has moved.
        // This prevents small text labels from losing their drag target when
        // egui crosses the drag threshold.
        state.smart_drag = if state.drag_section_cut.is_some() {None} else {ui.input(|i| i.pointer.press_origin())
            .or_else(|| response.interact_pointer_pos())
            .and_then(|pos| hit_annotation(state, paper, factor, pos))};
        if let Some((index,_))=state.smart_drag {
            state.annotation_selected=Some(index);
            if matches!(state.annotations.get(index),Some(DrawingAnnotation::Smart {..})) { state.tolerance_edit=Some(index); }
        }
        if state.smart_drag.is_some() { state.drag_before = Some(state.snapshot()); }
    }
    if response.dragged_by(egui::PointerButton::Primary) {
        if let Some(parent_id)=state.drag_section_cut {
            if let Some(parent)=state.placed.iter().find(|v|v.id==parent_id) {
                let ratio=parent.frame.scale().factor() as f32*factor;
                if ratio.abs()>1e-6 {
                    let dx=(ui.input(|i|i.pointer.delta()).x/ratio) as f64;
                    if let Some((id,x))=state.section_pending.as_mut() {
                        if *id==parent_id {*x+=dx;}
                    } else {
                        for child in state.placed.iter_mut().filter(|v|v.parent==Some(parent_id)) {
                            if let Some(spec)=child.section.as_mut() {spec.cut_x+=dx;}
                        }
                    }
                }
            }
        }
        if let Some((index,text)) = state.smart_drag {
            let delta = ui.input(|i| i.pointer.delta());
            if let Some(DrawingAnnotation::Smart {view_id,geometry,offset,text_shift,..}) = state.annotations.get_mut(index) {
                if let Some(view) = state.placed.iter().find(|v| v.id == *view_id) {
                    let ratio = view.frame.scale().factor() as f32 * factor;
                    if ratio.abs()>1e-6 {
                        if text {
                            // Label follows the cursor by the exact screen-space delta.
                            text_shift.x += (delta.x/ratio) as f64;
                            text_shift.y -= (delta.y/ratio) as f64;
                        }
                        else {
                            let (dx,dy)=((delta.x/ratio) as f64,-(delta.y/ratio) as f64);
                            let (nx,ny)=match geometry {
                                SmartGeometry::Distance {kind:SmartKind::Horizontal,..}=>(0.0,1.0),
                                SmartGeometry::Distance {kind:SmartKind::Vertical,..}=>(1.0,0.0),
                                SmartGeometry::Distance {a,b,..}=>{let x=b.x-a.x;let y=b.y-a.y;let l=x.hypot(y).max(1e-8);(-y/l,x/l)},
                                SmartGeometry::Radius {..}=>(1.0,0.0),
                                SmartGeometry::Angle {..}=>(1.0,0.0),
                            };
                            if matches!(geometry,SmartGeometry::Angle {..}) {
                                if let Some(pointer)=ui.input(|i|i.pointer.hover_pos()) {
                                    smart_set_angle_sector(geometry,view,paper,factor,pointer);
                                    let (anchor,scale)=smart_anchor(view,paper,factor);
                                    if let SmartGeometry::Angle {vertex,..}=geometry {
                                        let at=ModelVec2::new(((pointer.x-anchor.x)/scale) as f64,((anchor.y-pointer.y)/scale) as f64);
                                        *offset=(at.x-vertex.x).hypot(at.y-vertex.y).max(1.0);
                                    }
                                }
                            } else if let SmartGeometry::Radius {center,angle,radius,..}=geometry {
                                if let Some(pointer)=ui.input(|i|i.pointer.hover_pos()) {
                                    let (anchor,scale)=smart_anchor(view,paper,factor);
                                    if scale.abs()>1e-6 {
                                        let x=((pointer.x-anchor.x)/scale) as f64-center.x;
                                        let y=((anchor.y-pointer.y)/scale) as f64-center.y;
                                        let distance=x.hypot(y);
                                        if distance>1e-5 { *angle=y.atan2(x); }
                                        // Leader dragging changes direction and reach,
                                        // without modifying the measured radius.
                                        *offset=(distance-*radius).max(0.0);
                                        // Reposition the radial endpoint, retaining any
                                        // user-adjusted text shift independently.
                                    }
                                }
                            } else { *offset += dx*nx+dy*ny; }
                        }
                    }
                }
            } else if let Some(DrawingAnnotation::Note {at,..})=state.annotations.get_mut(index) {
                let delta=ui.input(|i|i.pointer.delta());
                if factor.abs()>1e-6 {at.x+=(delta.x/factor) as f64;at.y-=(delta.y/factor) as f64;}
            } else if let Some(DrawingAnnotation::Leader {target,text_at,..})=state.annotations.get_mut(index) {
                let delta=ui.input(|i|i.pointer.delta());
                if factor.abs()>1e-6 {
                    let dx=(delta.x/factor) as f64;
                    let dy=-(delta.y/factor) as f64;
                    if text {text_at.x+=dx;text_at.y+=dy;} else {target.x+=dx;target.y+=dy;}
                }
            }
        }
    }
    if response.drag_started_by(egui::PointerButton::Primary) && state.smart_drag.is_none() && !state.place_mode && !state.project_mode && state.annotation_tool == AnnotationTool::None && state.drag_section_cut.is_none() && !state.section_mode {
        state.drag_view = response.interact_pointer_pos().and_then(|p| hit_view(state, paper, factor, p));
        if let Some(id) = state.drag_view { state.selected = Some(id); state.drag_before = Some(state.snapshot()); }
    }
    if response.dragged_by(egui::PointerButton::Primary) {
        if let Some(id) = state.drag_view {
            let delta = ui.input(|i| i.pointer.delta());
            if let Some(view) = state.placed.iter_mut().find(|v| v.id == id) {
                let dx = (delta.x / factor) as f64;
                let dy = -(delta.y / factor) as f64;
                if view.section.is_some() {
                    // Section views are independent sheet placements, even though
                    // their geometry remains associated with the parent view.
                    view.at.x += dx;
                    view.at.y += dy;
                } else if view.parent.is_some() {
                    // Projected orthographic views move along their projection axis.
                    if matches!(view.orientation, NamedView::Top | NamedView::Bottom) { view.at.y += dy; }
                    else { view.at.x += dx; }
                } else { view.at.x += dx; view.at.y += dy; }
                // Base-view movement carries projected descendants while retaining their offsets.
                if view.parent.is_none() {
                    let dx = (delta.x / factor) as f64;
                    let dy = -(delta.y / factor) as f64;
                    let mut moved = vec![id];
                    loop {
                        let mut next = Vec::new();
                        for child in state.placed.iter_mut() {
                            if child.parent.is_some_and(|p| moved.contains(&p)) && !moved.contains(&child.id) {
                                child.at.x += dx; child.at.y += dy; next.push(child.id);
                            }
                        }
                        if next.is_empty() { break; }
                        moved.extend(next);
                    }
                }
            }
        }
    }
    if response.drag_stopped_by(egui::PointerButton::Primary) {
        state.smart_drag = None;
        state.drag_view = None;
        state.drag_section_cut = None;
        if let Some(before) = state.drag_before.take() { state.push_undo(before); }
    }
    // Cutting plane indicators on parent views.
    for child in state.placed.iter().filter(|v|v.visible) {
        let Some(spec)=&child.section else {continue};
        let Some(parent)=child.parent.and_then(|id|state.placed.iter().find(|v|v.id==id && v.visible)) else {continue};
        let anchor=sheet_to_screen(parent.at,paper,factor);
        let ratio=parent.frame.scale().factor() as f32*factor;
        let x=anchor.x+spec.cut_x as f32*ratio;
        let ys:Vec<f32>=state.section_mesh.iter().flat_map(|tri|tri.iter()).map(|p| {
            let basis=parent.orientation.basis();
            let q=(*p).dot(basis.y);
            anchor.y-q as f32*ratio
        }).collect();
        if !ys.is_empty() {
            let y0=ys.iter().copied().fold(f32::INFINITY,f32::min)-12.0;
            let y1=ys.iter().copied().fold(f32::NEG_INFINITY,f32::max)+12.0;
            painter.line_segment([Pos2::new(x,y0),Pos2::new(x,y1)],Stroke::new(1.2,Color32::DARK_GRAY));
            for y in [y0,y1] {painter.circle_filled(Pos2::new(x,y),3.5,Color32::from_rgb(20,110,210));}
            for y in [y0,y1] {
                painter.text(Pos2::new(x+5.0,y),egui::Align2::LEFT_CENTER,&spec.tag,sheet_font(3.0,factor),Color32::BLACK);
            }
        }
    }
    if state.section_mode {
        if let Some((parent_id,cut_x))=state.section_pending {
            if let Some(parent)=state.placed.iter().find(|v|v.id==parent_id) {
                let anchor=sheet_to_screen(parent.at,paper,factor);
                let x=anchor.x+cut_x as f32*parent.frame.scale().factor() as f32*factor;
                let bounds=view_bounds(state,parent,paper,factor);
                painter.line_segment([Pos2::new(x,bounds.top()-12.0),Pos2::new(x,bounds.bottom()+12.0)],Stroke::new(2.0,Color32::from_rgb(20,110,210)));
                for y in [bounds.top()-12.0,bounds.bottom()+12.0] {
                    painter.circle_filled(Pos2::new(x,y),4.0,Color32::from_rgb(20,110,210));
                }
            }
        }
    }
    for (i, view) in state.placed.iter().enumerate() {
        if !view.visible { continue; }
        let p = Pos2::new(paper.min.x + view.at.x as f32 * factor, paper.max.y - view.at.y as f32 * factor);
        if view.section.is_none() && matches!(view.display, DisplayStyle::Shaded | DisplayStyle::ShadedEdges | DisplayStyle::ShadedHidden) {
            if let Some(shade) = state.shaded.get(&view.orientation) {
                let scale = view.frame.scale().factor() as f32 * factor;
                let rect = Rect::from_min_max(
                    Pos2::new(p.x + shade.min.x as f32 * scale, p.y - shade.max.y as f32 * scale),
                    Pos2::new(p.x + shade.max.x as f32 * scale, p.y - shade.min.y as f32 * scale),
                );
                if rect.is_positive() {
                    painter.image(shade.texture.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0,1.0)), Color32::WHITE);
                }
            }
        }
        // HLR coordinates are model millimetres. Apply paper:model ratio once,
        // then the canvas pixels/mm zoom. Keep drawing measurements in model units.
        if view.section.is_none() && view.display != DisplayStyle::Shaded {
        if let Some(strokes) = state.projected.get(&view.orientation) {
            let ratio = view.frame.scale().factor() as f32;
            // Draw hidden and uncertain geometry underneath visible contours.
            // HLR output order is not a paint order: a dashed hidden segment
            // painted after a solid contour can make it appear broken at 1:1.
            for pass in 0..2 {
                for stroke in strokes {
                    if state.visible_only_debug && (stroke.hidden || stroke.uncertain) { continue; }
                    if stroke.hidden && !matches!(view.display, DisplayStyle::VisibleHidden | DisplayStyle::ShadedHidden) { continue; }
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
        } // End HLR drawing block.
        if let Some(spec)=&view.section {
            let ratio=view.frame.scale().factor() as f32 * factor;
            let segments=section_segments(&state.section_mesh,view.orientation,spec.cut_x);
            draw_section_mesh(&painter,p,ratio,view.frame.scale().factor() as f64,&segments);
        }
        // Centre marks and axes are derived from actual projected circular edges.
        // They do not create dimensionable model edges or alter the HLR cache.
        if view.section.is_none() && (view.centre_marks || view.centre_lines) && view.display != DisplayStyle::Shaded {
            if let Some(strokes) = state.projected.get(&view.orientation) {
                let mut circles: Vec<(ModelVec2, f64)> = Vec::new();
                for stroke in strokes.iter().filter(|s| !s.hidden && !s.uncertain) {
                    let circle = match &stroke.curve {
                        Seg2::Arc {center,radius,sweep,..}
                            if sweep.abs() >= std::f64::consts::TAU - 0.02 => Some((*center,*radius)),
                        Seg2::Conic {..} | Seg2::Cubic {..} | Seg2::Bezier {..} =>
                            smart_circle_from_curve(&stroke.curve).and_then(|(c,r,closed)| closed.then_some((c,r))),
                        _ => None,
                    };
                    if let Some((center,radius)) = circle {
                        if radius < 0.01 || !radius.is_finite() { continue; }
                        if !circles.iter().any(|(c,r)|
                            (c.x-center.x).hypot(c.y-center.y)<0.05 && (r-radius).abs()<0.05) {
                            circles.push((center,radius));
                        }
                    }
                }
                let scale = view.frame.scale().factor() as f32 * factor;
                let ink = Stroke::new(0.9, Color32::from_rgb(70, 95, 125));
                for (center,radius) in circles {
                    let c = Pos2::new(p.x + center.x as f32*scale,p.y-center.y as f32*scale);
                    if view.centre_marks {
                        let half = (radius as f32*scale*0.45).clamp(6.0,13.0);
                        painter.line_segment([c+egui::vec2(-half,0.0),c+egui::vec2(half,0.0)],ink);
                        painter.line_segment([c+egui::vec2(0.0,-half),c+egui::vec2(0.0,half)],ink);
                    }
                    if view.centre_lines {
                        let half = (radius as f32*scale+9.0).max(12.0);
                        // Short-dashed axes extending beyond the circular feature.
                        for horizontal in [true,false] {
                            let mut t=-half;
                            while t<half {
                                let e=(t+7.0).min(half);
                                let a=if horizontal {c+egui::vec2(t,0.0)} else {c+egui::vec2(0.0,t)};
                                let b=if horizontal {c+egui::vec2(e,0.0)} else {c+egui::vec2(0.0,e)};
                                painter.line_segment([a,b],ink);
                                t+=11.0;
                            }
                        }
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
            format!("V{} {} {}", i+1, view.label, view.frame.scale()),
            sheet_font(3.0,factor), c);
    }
    draw_annotations(state, &painter, paper, factor);
    draw_smart_preview(state, &painter, paper, factor, response.hover_pos());
    draw_centre_tool_hover(state, &painter, paper, factor, response.hover_pos());
    if state.annotation_tool != AnnotationTool::None && state.annotation_tool != AnnotationTool::Smart {
        if let (Some(first), Some(pointer)) = (state.annotation_first, response.hover_pos()) {
            let from = sheet_to_screen(first, paper, factor);
            painter.line_segment([from, pointer], Stroke::new(1.0, Color32::from_rgb(35, 110, 215)));
        }
    }
    // Linked projected-view preview, using the same curve-aware renderer.
    if state.project_mode {
        if let Some(pointer) = response.hover_pos().filter(|p| border.contains(*p)) {
            if let Some((parent_id, orientation, at)) = projected_target(state, paper, factor, pointer) {
                if let Some(parent) = state.placed.iter().find(|v| v.id == parent_id) {
                    let ratio = parent.frame.scale().factor() as f32;
                    let anchor = Pos2::new(paper.min.x + at.x as f32 * factor, paper.max.y - at.y as f32 * factor);
                    painter.text(pointer + Vec2::new(12.0, -18.0), egui::Align2::LEFT_BOTTOM, format!("Projected: {:?}", orientation), sheet_font(3.0,factor), Color32::from_rgb(25, 95, 190));
                    if let Some(strokes) = state.projected.get(&orientation) {
                        for stroke in strokes {
                            if stroke.hidden && parent.display != DisplayStyle::VisibleHidden { continue; }
                            let points: Vec<Pos2> = adaptive_curve_points(&stroke.curve, ratio * factor, 0.20).iter().map(|pt| Pos2::new(anchor.x + pt.x as f32 * ratio * factor, anchor.y - pt.y as f32 * ratio * factor)).collect();
                            if points.len() >= 2 { painter.add(egui::Shape::line(points, Stroke::new(1.1, Color32::from_rgb(40, 115, 215)))); }
                        }
                    }
                }
            }
        }
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

// Model-space geometry picking: only HLR curves, never arbitrary sheet clicks.
fn smart_anchor(view: &PlacedView, paper: Rect, factor: f32) -> (Pos2,f32) {
    (sheet_to_screen(view.at, paper, factor), view.frame.scale().factor() as f32 * factor)
}
fn smart_to_screen(view: &PlacedView, point: ModelVec2, paper: Rect, factor: f32) -> Pos2 {
    let (anchor,scale) = smart_anchor(view,paper,factor);
    Pos2::new(anchor.x + point.x as f32 * scale,anchor.y-point.y as f32 * scale)
}
fn point_segment_distance(p: Pos2,a:Pos2,b:Pos2)->f32 {
    let d=b-a; let len=d.length_sq();
    if len<1e-8 {return p.distance(a)}
    let t=((p-a).dot(d)/len).clamp(0.0,1.0);
    p.distance(a+d*t)
}
// Fit an analytic circle to a projected conic/spline span. HLR often emits
// circular B-rep edges as rational conics rather than Seg2::Arc.
// Reject non-circular curves instead of assigning misleading diameters.
fn smart_circle_from_curve(curve: &Seg2) -> Option<(ModelVec2, f64, bool)> {
    let a = curve.point_at(0.0);
    let b = curve.point_at(0.25);
    let c = curve.point_at(0.5);
    let d = curve.point_at(0.75);
    let e = curve.point_at(1.0);
    let ux = b.x-a.x; let uy = b.y-a.y;
    let vx = c.x-a.x; let vy = c.y-a.y;
    let det = 2.0*(ux*vy-uy*vx);
    if !det.is_finite() || det.abs() < 1e-10 { return None; }
    let u2 = ux*ux+uy*uy; let v2 = vx*vx+vy*vy;
    let center = ModelVec2::new(a.x+(u2*vy-v2*uy)/det, a.y+(ux*v2-vx*u2)/det);
    let radius = (a.x-center.x).hypot(a.y-center.y);
    if !radius.is_finite() || radius < 1e-5 { return None; }
    for pt in [b,c,d,e] {
        let err = ((pt.x-center.x).hypot(pt.y-center.y)-radius).abs();
        if err > (radius*0.002).max(0.002) { return None; }
    }
    let closed = (a.x-e.x).hypot(a.y-e.y) < radius*0.01;
    Some((center,radius,closed))
}
fn pick_smart_geometry(state:&State,paper:Rect,factor:f32,pointer:Pos2)->Option<SmartPick> {
    let mut best:Option<(f32,SmartPick)>=None;
    for ann in &state.annotations {
        match ann {
            DrawingAnnotation::CentreMark{view_id,center,..} => {
                if let Some(v)=state.placed.iter().find(|v|v.id==*view_id && v.visible) {
                    let dist=pointer.distance(smart_to_screen(v,*center,paper,factor));
                    if dist<13.0 {best=Some((dist,SmartPick::Centre{view_id:*view_id,at:*center}));}
                }
            }
            DrawingAnnotation::CentreLine{view_id,a,b} => {
                if let Some(v)=state.placed.iter().find(|v|v.id==*view_id && v.visible) {
                    let dist=point_segment_distance(pointer,smart_to_screen(v,*a,paper,factor),smart_to_screen(v,*b,paper,factor));
                    if dist<9.0 && best.as_ref().is_none_or(|(d,_)|dist<*d) {
                        best=Some((dist,SmartPick::Line{view_id:*view_id,a:*a,b:*b}));
                    }
                }
            }
            _=>{}
        }
    }
    for view in state.placed.iter().filter(|v|v.visible) {
        // A section shares its parent's orientation, but not its HLR edges.
        // Pick the section-plane intersections instead of inheriting the
        // complete parent projection (which caused the long blue ghost edge).
        if let Some(spec) = &view.section {
            for [a,b] in section_segments(&state.section_mesh, view.orientation, spec.cut_x) {
                let p=smart_to_screen(view,a,paper,factor);
                let q=smart_to_screen(view,b,paper,factor);
                let da=pointer.distance(p);
                let db=pointer.distance(q);
                let (distance,pick)=if da<12.0 || db<12.0 {
                    (da.min(db),SmartPick::Vertex {view_id:view.id,at:if da<db {a} else {b}})
                } else {
                    (point_segment_distance(pointer,p,q),SmartPick::Line {view_id:view.id,a,b})
                };
                if distance<16.0 && best.as_ref().is_none_or(|(d,_)|distance+3.0<*d) {
                    best=Some((distance,pick));
                }
            }
            continue;
        }
        let Some(strokes)=state.projected.get(&view.orientation) else {continue};
        for stroke in strokes.iter().filter(|s|!s.uncertain && (!s.hidden || matches!(view.display,DisplayStyle::VisibleHidden|DisplayStyle::ShadedHidden))) {
            let pick=match &stroke.curve {
                Seg2::Line {a,b} => {
                    // Select only real HLR endpoints. Never infer dimension
                    // geometry from a view or silhouette bounding box.
                    let (a,b)=(*a,*b);
                    let p=smart_to_screen(view,a,paper,factor);let q=smart_to_screen(view,b,paper,factor);
                    let da=pointer.distance(p);let db=pointer.distance(q);
                    if da<12.0 || db<12.0 {
                        Some((da.min(db),SmartPick::Vertex {view_id:view.id,at:if da<db {a} else {b}}))
                    } else {Some((point_segment_distance(pointer,p,q),SmartPick::Line {view_id:view.id,a,b}))}
                }
                Seg2::Arc {center,radius,sweep,..} => {
                    // Hit-test the ACTUAL visible arc, not its infinite supporting circle.
                    // This also works for partial arcs and complete circular edges.
                    let scale = (view.frame.scale().factor() as f32 * factor).abs();
                    let points = adaptive_curve_points(&stroke.curve, scale, 0.35);
                    let distance = points.windows(2).map(|pair| {
                        point_segment_distance(pointer,
                            smart_to_screen(view,pair[0],paper,factor),
                            smart_to_screen(view,pair[1],paper,factor))
                    }).fold(f32::INFINITY, f32::min);
                    if radius.is_finite() && *radius > 0.0 && distance.is_finite() {
                        Some((distance,SmartPick::Circle {
                            view_id:view.id,center:*center,radius:*radius,
                            diameter:sweep.abs() >= std::f64::consts::TAU-0.02,
                        }))
                    } else { None }
                }
                // HLR circular edges can be rational conics or spline pieces.
                // Pick the visible curve, then recover its true circle if possible.
                Seg2::Conic {..} | Seg2::Cubic {..} | Seg2::Bezier {..} => {
                    let scale = (view.frame.scale().factor() as f32 * factor).abs();
                    let points = adaptive_curve_points(&stroke.curve, scale, 0.25);
                    let distance = points.windows(2).map(|pair| point_segment_distance(
                        pointer, smart_to_screen(view,pair[0],paper,factor),
                        smart_to_screen(view,pair[1],paper,factor)))
                        .fold(f32::INFINITY,f32::min);
                    smart_circle_from_curve(&stroke.curve).map(|(center,radius,diameter)| (
                        distance, SmartPick::Circle {view_id:view.id,center,radius,diameter}))
                }
            };
            if let Some((distance,entity))=pick {
                if distance<16.0 && best.as_ref().is_none_or(|(d,_)|distance+3.0<*d) {best=Some((distance,entity));}
            }
        }
    }
    best.map(|(_,pick)|pick)
}
fn centre_between_lines(a:ModelVec2,b:ModelVec2,c:ModelVec2,d:ModelVec2)->Option<(ModelVec2,ModelVec2)> {
    let ux=b.x-a.x;let uy=b.y-a.y;let vx=d.x-c.x;let vy=d.y-c.y;
    let ul=ux.hypot(uy);let vl=vx.hypot(vy);
    if ul<1e-6||vl<1e-6{return None;}
    let nx=ux/ul;let ny=uy/ul;
    if ((ux*vy-uy*vx)/(ul*vl)).abs()>0.01{return None;}
    let separation=((c.x-a.x)*ny-(c.y-a.y)*nx).abs();
    if separation<0.01{return None;}
    let project=|p:ModelVec2| p.x*nx+p.y*ny;
    let lo=project(a).min(project(b)).max(project(c).min(project(d)));
    let hi=project(a).max(project(b)).min(project(c).max(project(d)));
    if hi-lo<0.01{return None;}
    let mid=ModelVec2::new((a.x+c.x)*0.5,(a.y+c.y)*0.5);
    let origin=project(mid);
    Some((ModelVec2::new(mid.x+nx*(lo-origin),mid.y+ny*(lo-origin)),
          ModelVec2::new(mid.x+nx*(hi-origin),mid.y+ny*(hi-origin))))
}
fn stroke_sources_for_line(state:&State, view_id:u64, a:ModelVec2,b:ModelVec2)->Option<Vec<HlrSourcePath>> {
    let view=state.placed.iter().find(|v|v.id==view_id)?;
    let strokes=state.projected.get(&view.orientation)?;
    let near=|p:ModelVec2,q:ModelVec2| (p.x-q.x).hypot(p.y-q.y)<0.01;
    let mut found=strokes.iter().filter(|st|match &st.curve {
        Seg2::Line{a:x,b:y} => (near(a,*x)&&near(b,*y))||(near(a,*y)&&near(b,*x)),
        _=>false,
    });
    let first=found.next()?;
    if found.next().is_some() {return None;}
    Some(first.sources.clone())
}

// Infer parallel spacing or included angle from two selected HLR line segments.
fn smart_pair_lines(a:ModelVec2,b:ModelVec2,c:ModelVec2,d:ModelVec2)->Option<SmartGeometry> {
    let (ux,uy)=(b.x-a.x,b.y-a.y);
    let (vx,vy)=(d.x-c.x,d.y-c.y);
    let (ul,vl)=(ux.hypot(uy),vx.hypot(vy));
    if ul<1e-8 || vl<1e-8 {return None}
    let cross=ux*vy-uy*vx;
    let dot=ux*vx+uy*vy;
    if cross.abs()/(ul*vl)<0.015 {
        // Select corresponding points along the perpendicular normal, not endpoint distance.
        let mid=ModelVec2::new((c.x+d.x)*0.5,(c.y+d.y)*0.5);
        let t=((mid.x-a.x)*ux+(mid.y-a.y)*uy)/(ul*ul);
        let foot=ModelVec2::new(a.x+t*ux,a.y+t*uy);
        if (foot.x-mid.x).hypot(foot.y-mid.y)<1e-6 {return None}
        Some(SmartGeometry::Distance {a:foot,b:mid,kind:SmartKind::Aligned})
    } else {
        let t=((c.x-a.x)*vy-(c.y-a.y)*vx)/cross;
        let origin=ModelVec2::new(a.x+t*ux,a.y+t*uy);
        let dir_a=ModelVec2::new(ux/ul,uy/ul);
        let sign=if dot<0.0 {-1.0} else {1.0};
        let dir_b=ModelVec2::new(vx/vl*sign,vy/vl*sign);
        Some(SmartGeometry::Angle {vertex:origin,a:dir_a,b:dir_b,sector:0})
    }
}
// Choose the angular sector from cursor position, independently of HLR edge direction.
// Bits 0/1 reverse the two rays; the four combinations cover all four sectors.
fn smart_angle_rays(a:ModelVec2,b:ModelVec2,sector:u8)->(ModelVec2,ModelVec2) {
    let sa=if sector&1!=0 {-1.0} else {1.0};
    let sb=if sector&2!=0 {-1.0} else {1.0};
    (ModelVec2::new(a.x*sa,a.y*sa),ModelVec2::new(b.x*sb,b.y*sb))
}
fn smart_angle_sector(vertex:ModelVec2,a:ModelVec2,b:ModelVec2,at:ModelVec2)->u8 {
    let direction=ModelVec2::new(at.x-vertex.x,at.y-vertex.y);
    let length=direction.x.hypot(direction.y);
    if length<1e-8 {return 0}
    let mut best=0;
    let mut best_score=f64::NEG_INFINITY;
    for sector in 0..4 {
        let (u,v)=smart_angle_rays(a,b,sector);
        let sum=ModelVec2::new(u.x+v.x,u.y+v.y);
        let norm=sum.x.hypot(sum.y);
        if norm<1e-8 {continue}
        let score=(sum.x*direction.x+sum.y*direction.y)/(norm*length);
        if score>best_score {best=sector;best_score=score;}
    }
    best
}
fn smart_set_angle_sector(geometry:&mut SmartGeometry,view:&PlacedView,paper:Rect,factor:f32,pointer:Pos2) {
    if let SmartGeometry::Angle {vertex,a,b,sector}=geometry {
        let (anchor,scale)=smart_anchor(view,paper,factor);
        if scale.abs()<1e-6 {return}
        let at=ModelVec2::new(((pointer.x-anchor.x)/scale) as f64,((anchor.y-pointer.y)/scale) as f64);
        *sector=smart_angle_sector(*vertex,*a,*b,at);
    }
}
fn smart_set_radius_angle(geometry:&mut SmartGeometry,state:&State,view_id:u64,paper:Rect,factor:f32,pointer:Pos2) {
    let SmartGeometry::Radius {center,angle,..}=geometry else {return};
    let Some(view)=state.placed.iter().find(|v|v.id==view_id) else {return};
    let (anchor,scale)=smart_anchor(view,paper,factor);
    if scale.abs()<1e-6 {return}
    let x=((pointer.x-anchor.x)/scale) as f64-center.x;
    let y=((anchor.y-pointer.y)/scale) as f64-center.y;
    if x.hypot(y)>1e-5 {*angle=y.atan2(x);}
}
fn smart_offset(state:&State,pending:&SmartPending,paper:Rect,factor:f32,pointer:Pos2)->f64 {
    let Some(view)=state.placed.iter().find(|v|v.id==pending.view_id) else {return 10.0};
    let (anchor,scale)=smart_anchor(view,paper,factor);
    if scale.abs()<1e-6 {return 10.0}
    let at=ModelVec2::new(((pointer.x-anchor.x)/scale) as f64,((anchor.y-pointer.y)/scale) as f64);
    // Keep initial dimensions outside the projected part bounds.  This bounding
    // box contains only HLR model curves, never existing annotations.
    // Use actual HLR part geometry, NOT the view selection rectangle.  That
    // rectangle can include empty padding and causes misplaced dimensions.
    let Some((min,max))=projected_model_bounds(state,view.orientation) else {return 10.0};
    let margin=8.0_f64; // paper mm; convert to model coordinates for this view
    let margin_model=margin * factor as f64 / (scale.abs() as f64);
    let corners=[ModelVec2::new(min.x,min.y),ModelVec2::new(min.x,max.y),
                 ModelVec2::new(max.x,min.y),ModelVec2::new(max.x,max.y)];
    match &pending.geometry {
        SmartGeometry::Angle {vertex,..} => (at.x-vertex.x).hypot(at.y-vertex.y).max(1.0),
        SmartGeometry::Distance {a,b,..} => {
            let dx=b.x-a.x;let dy=b.y-a.y;
            let len=dx.hypot(dy).max(1e-8);
            let (nx,ny)=match &pending.geometry {
                SmartGeometry::Distance {kind:SmartKind::Horizontal,..}=>(0.0,1.0),
                SmartGeometry::Distance {kind:SmartKind::Vertical,..}=>(1.0,0.0),
                _=>(-dy/len,dx/len),
            };
            let requested=(at.x-a.x)*nx+(at.y-a.y)*ny;
            let projections=corners.map(|m| (m.x-a.x)*nx+(m.y-a.y)*ny);
            if requested>=0.0 {
                requested.max(projections.into_iter().fold(f64::NEG_INFINITY,f64::max)+margin_model)
            } else {
                requested.min(projections.into_iter().fold(f64::INFINITY,f64::min)-margin_model)
            }
        }
        SmartGeometry::Radius {center,radius,..} => {
            // The mouse is the leader endpoint; never force it outside the
            // projected part bounds (which causes large jumps).
            ((at.x-center.x).hypot(at.y-center.y)-radius).max(0.0)
        },
    }
}
fn smart_dimension_points(view:&PlacedView,geometry:&SmartGeometry,offset:f64,text_shift:ModelVec2,paper:Rect,factor:f32)->(Pos2,Pos2,Pos2,String) {
    match geometry {
        SmartGeometry::Angle {vertex,a,b,sector} => {
            let (a,b)=smart_angle_rays(*a,*b,*sector);
            let radius=offset.abs().max(1.0);
            let p=ModelVec2::new(vertex.x+a.x*radius,vertex.y+a.y*radius);
            let q=ModelVec2::new(vertex.x+b.x*radius,vertex.y+b.y*radius);
            let bis=ModelVec2::new(a.x+b.x,a.y+b.y);
            let norm=bis.x.hypot(bis.y).max(1e-8);
            let t=ModelVec2::new(vertex.x+bis.x/norm*radius+text_shift.x,vertex.y+bis.y/norm*radius+text_shift.y);
            let deg=(a.x*b.y-a.y*b.x).atan2(a.x*b.x+a.y*b.y).abs().to_degrees();
            (smart_to_screen(view,p,paper,factor),smart_to_screen(view,q,paper,factor),smart_to_screen(view,t,paper,factor),format!("{deg:.2}Â°"))
        }
        SmartGeometry::Distance {a,b,kind} => {
            let (dx,dy)=(b.x-a.x,b.y-a.y);
            let len=dx.hypot(dy).max(1e-8);
            let (nx,ny)=match kind { SmartKind::Horizontal=>(0.0,1.0),SmartKind::Vertical=>(1.0,0.0),SmartKind::Aligned=>(-dy/len,dx/len) };
            let shift=ModelVec2::new(nx*offset,ny*offset);
            let u=smart_to_screen(view,ModelVec2::new(a.x+shift.x,a.y+shift.y),paper,factor);
            let v=smart_to_screen(view,ModelVec2::new(b.x+shift.x,b.y+shift.y),paper,factor);
            let text=smart_to_screen(view,ModelVec2::new((a.x+b.x)*0.5+shift.x+text_shift.x,(a.y+b.y)*0.5+shift.y+text_shift.y),paper,factor);
            let value=match kind {SmartKind::Horizontal=>dx.abs(),SmartKind::Vertical=>dy.abs(),SmartKind::Aligned=>len};
            (u,v,text,format!("{value:.2}"))
        }
        SmartGeometry::Radius {center,radius,diameter,angle} => {
            let direction=ModelVec2::new(angle.cos(),angle.sin());
            let a=ModelVec2::new(center.x+*radius*direction.x,center.y+*radius*direction.y);
            let b=ModelVec2::new(center.x+(*radius+offset)*direction.x,center.y+(*radius+offset)*direction.y);
            let t=ModelVec2::new(b.x+text_shift.x,b.y+text_shift.y);
            (smart_to_screen(view,a,paper,factor),smart_to_screen(view,b,paper,factor),smart_to_screen(view,t,paper,factor),
                if *diameter {format!("âŒ€{:.2}",*radius*2.0)} else {format!("R{radius:.2}")})
        }
    }
}
// Choose the landing side from the text's position relative to its arrow.
// This is screen-space presentation only; it never changes model geometry.
fn leader_landing(arrow: Pos2, text: Pos2, length: f32) -> Pos2 {
    let sign = if text.x >= arrow.x { -1.0 } else { 1.0 };
    Pos2::new(text.x + sign * length, text.y)
}

fn paint_smart_dimension(state:&State,painter:&egui::Painter,view:&PlacedView,geometry:&SmartGeometry,offset:f64,text_shift:ModelVec2,paper:Rect,factor:f32,ink:Color32,label_override:Option<&str>) {
    let (u,v,t,label)=smart_dimension_points(view,geometry,offset,text_shift,paper,factor);
    let stroke=Stroke::new(1.0,ink);
    match geometry {
        SmartGeometry::Angle {vertex,a,b,sector} => {
            let (a,b)=smart_angle_rays(*a,*b,*sector);
            let radius=offset.abs().max(1.0);
            let o=smart_to_screen(view,*vertex,paper,factor);
            painter.line_segment([o,u],stroke);
            painter.line_segment([o,v],stroke);
            let theta0=a.y.atan2(a.x);
            let delta=(a.x*b.y-a.y*b.x).atan2(a.x*b.x+a.y*b.y);
            let mut points=Vec::with_capacity(33);
            for i in 0..=32 {
                let angle=theta0+delta*(i as f64)/32.0;
                let p=ModelVec2::new(vertex.x+radius*angle.cos(),vertex.y+radius*angle.sin());
                points.push(smart_to_screen(view,p,paper,factor));
            }
            painter.add(egui::Shape::line(points,stroke));
        }
        SmartGeometry::Distance {a,b,..} => {
            // ISO-style extension origin gap (1.5 mm) and overshoot (2 mm).
            // Screen vectors are derived from the paper geometry, not model scale.
            for (origin, end) in [(smart_to_screen(view,*a,paper,factor),u),
                                  (smart_to_screen(view,*b,paper,factor),v)] {
                let delta = end - origin;
                let length = delta.length();
                if length > 0.1 {
                    let unit = delta / length;
                    let gap = 1.5 * factor;
                    let over = 2.0 * factor;
                    // Two-line dimensions can have an extension ray coincident
                    // with a selected HLR edge. Start beyond the visible edge,
                    // not from its midpoint (which would overdraw the part).
                    let mut start_distance = 0.0_f32;
                    if let Some(strokes) = state.projected.get(&view.orientation) {
                        for edge in strokes.iter().filter(|edge| !edge.hidden && !edge.uncertain) {
                            if let Seg2::Line {a,b} = &edge.curve {
                                let p = smart_to_screen(view,*a,paper,factor);
                                let q = smart_to_screen(view,*b,paper,factor);
                                let tangent = q-p;
                                let tangent_length = tangent.length();
                                if tangent_length < 0.1 { continue; }
                                // Only consider geometry collinear with the
                                // extension ray and overlapping its origin.
                                if (tangent.x*unit.y-tangent.y*unit.x).abs()/tangent_length > 0.015 { continue; }
                                if point_segment_distance(origin,p,q) > 1.5 { continue; }
                                for point in [p,q] {
                                    let along = (point-origin).dot(unit);
                                    if along > 0.0 && along < length {
                                        start_distance = start_distance.max(along);
                                    }
                                }
                            }
                        }
                    }
                    if length > start_distance + gap {
                        painter.line_segment([origin + unit * (start_distance + gap), end + unit * over],stroke);
                    }
                }
            }
            painter.line_segment([u,v],stroke);
            let d=v-u;
            if d.length_sq()>0.1 {
                let dir=d.normalized();let side=Vec2::new(-dir.y,dir.x);
                for (p,arrow) in [(u,dir),(v,-dir)] {
                    painter.add(egui::Shape::convex_polygon(vec![p,p+arrow*8.0+side*2.5,p+arrow*8.0-side*2.5],ink,Stroke::NONE));
                }
            }
        }
        SmartGeometry::Radius {..} => {
            // The arrow stays attached to the measured circumference; the
            // leader follows the independently movable text position.
            let landing = leader_landing(u, t, 8.0);
            painter.line_segment([u,landing],stroke);
            painter.line_segment([landing,t],stroke);
            let direction = (landing-u).normalized();
            if direction.length_sq()>0.01 {
                let side=Vec2::new(-direction.y,direction.x);
                painter.add(egui::Shape::convex_polygon(
                    vec![u,u+direction*8.0+side*2.5,u+direction*8.0-side*2.5],ink,Stroke::NONE));
            }
        }
    }
    painter.text(t+Vec2::new(0.0,-1.25*factor),egui::Align2::CENTER_BOTTOM,label_override.unwrap_or(&label),sheet_font(3.0,factor),ink);
}
fn draw_smart_preview(state:&State,painter:&egui::Painter,paper:Rect,factor:f32,pointer:Option<Pos2>) {
    if state.annotation_tool!=AnnotationTool::Smart {return}
    let Some(pointer)=pointer else {return};
    if let Some(pending)=&state.smart_pending {
        if let Some(view)=state.placed.iter().find(|v|v.id==pending.view_id) {
            // The first picked entity stays highlighted while selecting a second.
            let selected=Color32::from_rgb(20,165,85);
            match &pending.geometry {
                SmartGeometry::Distance {a,b,..} if !pending.from_vertices => {
                    painter.line_segment([smart_to_screen(view,*a,paper,factor),smart_to_screen(view,*b,paper,factor)],Stroke::new(4.0,selected));
                }
                SmartGeometry::Radius {center,radius,..} => {
                    let r=(*radius as f32*view.frame.scale().factor() as f32*factor).abs();
                    painter.circle_stroke(smart_to_screen(view,*center,paper,factor),r,Stroke::new(3.5,selected));
                }
                _=>{}
            }
            if let Some(SmartPick::Line {view_id,a,b})=pick_smart_geometry(state,paper,factor,pointer) {
                if view_id==pending.view_id {
                    painter.line_segment([smart_to_screen(view,a,paper,factor),smart_to_screen(view,b,paper,factor)],Stroke::new(3.5,Color32::from_rgb(0,185,230)));
                }
            }
            let offset=smart_offset(state,pending,paper,factor,pointer);
            let mut preview=pending.geometry.clone();
            smart_set_radius_angle(&mut preview,state,pending.view_id,paper,factor,pointer);
            smart_set_angle_sector(&mut preview,view,paper,factor,pointer);
            paint_smart_dimension(state,painter,view,&preview,offset,ModelVec2::ZERO,paper,factor,Color32::from_rgb(0,125,230),None);
        }
    } else if let Some(pick)=pick_smart_geometry(state,paper,factor,pointer) {
        let blue=Color32::from_rgb(0,125,230);
        match pick {
            SmartPick::Centre {view_id,at} | SmartPick::Vertex {view_id,at} => if let Some(v)=state.placed.iter().find(|v|v.id==view_id) {
                painter.circle_stroke(smart_to_screen(v,at,paper,factor),5.0,Stroke::new(3.0,blue));
            },
            SmartPick::Line {view_id,a,b} => if let Some(v)=state.placed.iter().find(|v|v.id==view_id) {
                painter.line_segment([smart_to_screen(v,a,paper,factor),smart_to_screen(v,b,paper,factor)],Stroke::new(4.0,blue));
            },
            SmartPick::Circle {view_id,center,radius,..} => if let Some(v)=state.placed.iter().find(|v|v.id==view_id) {
                painter.circle_stroke(smart_to_screen(v,center,paper,factor),(radius as f32*v.frame.scale().factor() as f32*factor).abs(),Stroke::new(3.0,blue));
            },
        }
    }
}
fn draw_centre_tool_hover(state:&State,painter:&egui::Painter,paper:Rect,factor:f32,pointer:Option<Pos2>) {
    if !matches!(state.annotation_tool,AnnotationTool::CentreMark|AnnotationTool::CentreLine) {return;}
    let selected=Stroke::new(3.0,Color32::from_rgb(20,165,85));
    let hover=Stroke::new(3.5,Color32::from_rgb(0,185,230));
    if state.annotation_tool==AnnotationTool::CentreLine {
        if let Some((view_id,a,b))=state.centre_first {
            if let Some(v)=state.placed.iter().find(|v|v.id==view_id) {
                painter.line_segment([smart_to_screen(v,a,paper,factor),smart_to_screen(v,b,paper,factor)],selected);
            }
        }
    }
    let Some(pointer)=pointer else {return};
    match pick_smart_geometry(state,paper,factor,pointer) {
        Some(SmartPick::Circle{view_id,center,radius,..}) if state.annotation_tool==AnnotationTool::CentreMark => {
            if let Some(v)=state.placed.iter().find(|v|v.id==view_id) {
                painter.circle_stroke(smart_to_screen(v,center,paper,factor),
                    (radius as f32*v.frame.scale().factor() as f32*factor).abs(),hover);
            }
        }
        Some(SmartPick::Line{view_id,a,b}) if state.annotation_tool==AnnotationTool::CentreLine => {
            if let Some(v)=state.placed.iter().find(|v|v.id==view_id) {
                painter.line_segment([smart_to_screen(v,a,paper,factor),smart_to_screen(v,b,paper,factor)],hover);
            }
        }
        _=>{}
    }
}
fn hit_annotation(state:&State,paper:Rect,factor:f32,pointer:Pos2)->Option<(usize,bool)> {
    for (index,annotation) in state.annotations.iter().enumerate().rev() {
        if let DrawingAnnotation::CentreMark{view_id,center,..}=annotation {
            if let Some(v)=state.placed.iter().find(|v|v.id==*view_id) {
                if pointer.distance(smart_to_screen(v,*center,paper,factor))<12.0 {return Some((index,false));}
            }
        } else if let DrawingAnnotation::CentreLine{view_id,a,b}=annotation {
            if let Some(v)=state.placed.iter().find(|v|v.id==*view_id) {
                if point_segment_distance(pointer,smart_to_screen(v,*a,paper,factor),smart_to_screen(v,*b,paper,factor))<9.0 {return Some((index,false));}
            }
        } else if let DrawingAnnotation::Smart {view_id,geometry,offset,text_shift,..}=annotation {
            if let Some(view)=state.placed.iter().find(|v|v.id==*view_id) {
                let (a,b,text,label)=smart_dimension_points(view,geometry,*offset,*text_shift,paper,factor);
                let text_width=(label.chars().count() as f32 * 7.5).max(35.0);
                let text_rect=Rect::from_center_size(text+Vec2::new(0.0,-11.0),Vec2::new(text_width+24.0,32.0));
                if text_rect.contains(pointer) {return Some((index,true))}
                let line_hit = if matches!(geometry,SmartGeometry::Radius {..}) {
                    let landing=leader_landing(a,text,8.0);
                    point_segment_distance(pointer,a,landing)<10.0 ||
                    point_segment_distance(pointer,landing,text)<10.0
                } else { point_segment_distance(pointer,a,b)<13.0 };
                if line_hit || pointer.distance(a)<12.0 {return Some((index,false))}
                if let SmartGeometry::Distance {a:source_a,b:source_b,..}=geometry {
                    if point_segment_distance(pointer,smart_to_screen(view,*source_a,paper,factor),a)<11.0 ||
                       point_segment_distance(pointer,smart_to_screen(view,*source_b,paper,factor),b)<11.0 {return Some((index,false))}
                }
            }
        } else if let DrawingAnnotation::Note {at,text,height_mm,align,..}=annotation {
            let b=sheet_to_screen(*at,paper,factor);
            let font_px=(*height_mm*factor).clamp(4.0,96.0);
            let width=(text.lines().map(|l|l.chars().count()).max().unwrap_or(1) as f32*font_px*0.7).max(font_px);
            let height=(text.lines().count().max(1) as f32*font_px*1.3)+font_px;
            let left=match align {NoteAlign::Left=>b.x,NoteAlign::Center=>b.x-width/2.0,NoteAlign::Right=>b.x-width};
            if Rect::from_min_size(Pos2::new(left-8.0,b.y-font_px-8.0),Vec2::new(width+16.0,height+16.0)).contains(pointer) {return Some((index,true));}
        } else if let DrawingAnnotation::Leader {target,text_at,text}=annotation {
            let a=sheet_to_screen(*target,paper,factor);
            let b=sheet_to_screen(*text_at,paper,factor);
            let elbow=leader_landing(a,b,12.0);
            let width=(text.chars().count() as f32*7.5).max(35.0);
            if Rect::from_min_size(b+Vec2::new(-8.0,-24.0),Vec2::new(width+24.0,36.0)).contains(pointer) {return Some((index,true));}
            if point_segment_distance(pointer,a,elbow)<13.0 || point_segment_distance(pointer,elbow,b)<13.0 || pointer.distance(a)<15.0 {return Some((index,false));}
        }
    }
    None
}
fn sheet_to_screen(p: ModelVec2, paper: Rect, factor: f32) -> Pos2 {
    Pos2::new(paper.min.x + p.x as f32 * factor, paper.max.y - p.y as f32 * factor)
}
fn draw_annotations(state: &State, painter: &egui::Painter, paper: Rect, factor: f32) {
    let ink = Color32::from_rgb(20, 55, 105);
    let stroke = Stroke::new(1.0, ink);
    for annotation in &state.annotations {
        match annotation {
            DrawingAnnotation::CentreMark{view_id,center,radius} => {
                if let Some(v)=state.placed.iter().find(|v|v.id==*view_id && v.visible) {
                    let p=smart_to_screen(v,*center,paper,factor);
                    let len=((*radius as f32)*v.frame.scale().factor() as f32*factor*0.6).clamp(7.0,16.0);
                    let selected=state.annotation_selected.is_some_and(|i|std::ptr::eq(&state.annotations[i],annotation));
                    let pen=Stroke::new(1.0,if selected{Color32::from_rgb(20,130,220)}else{ink});
                    painter.line_segment([p+egui::vec2(-len,0.0),p+egui::vec2(len,0.0)],pen);
                    painter.line_segment([p+egui::vec2(0.0,-len),p+egui::vec2(0.0,len)],pen);
                }
            }
            DrawingAnnotation::CentreLine{view_id,a,b} => {
                if let Some(v)=state.placed.iter().find(|v|v.id==*view_id && v.visible) {
                    let p=smart_to_screen(v,*a,paper,factor);let q=smart_to_screen(v,*b,paper,factor);
                    let delta=q-p;let length=delta.length();
                    if length>1.0 {
                        let direction=delta/length;
                        let selected=state.annotation_selected.is_some_and(|i|std::ptr::eq(&state.annotations[i],annotation));
                        let pen=Stroke::new(1.0,if selected{Color32::from_rgb(20,130,220)}else{ink});
                        let mut t=0.0_f32;
                        while t<length {let e=(t+10.0).min(length);painter.line_segment([p+direction*t,p+direction*e],pen);t+=14.0;}
                    }
                }
            }
            DrawingAnnotation::Smart {view_id,geometry,offset,text_shift,tolerance,..} => {
                if let Some(view) = state.placed.iter().find(|v| v.id == *view_id) {
                    let index = state.annotations.iter().position(|a| std::ptr::eq(a, annotation));
                    let active = index.is_some_and(|i| state.annotation_selected == Some(i));
                    let hover = painter.ctx().pointer_hover_pos().and_then(|pos| hit_annotation(state,paper,factor,pos)).is_some_and(|(i,_)| Some(i)==index);
                    let highlight = if active { Color32::from_rgb(35,105,220) } else if hover { Color32::from_rgb(0,190,220) } else { ink };
                    let (_,_,_,nominal)=smart_dimension_points(view,geometry,*offset,*text_shift,paper,factor);
                    let label=tolerance_label(&nominal,tolerance);
                    paint_smart_dimension(state,painter, view, geometry, *offset, *text_shift, paper, factor, highlight,Some(&label));
                    if active || hover {
                        let (_,_,text,label)=smart_dimension_points(view,geometry,*offset,*text_shift,paper,factor);
                        let label=tolerance_label(&label,tolerance);
                        painter.rect_stroke(Rect::from_center_size(text+Vec2::new(0.0,-11.0),Vec2::new((label.chars().count() as f32*7.5).max(35.0)+16.0,24.0)),2.0,Stroke::new(1.0,highlight),egui::StrokeKind::Outside);
                    }
                }
            }
            DrawingAnnotation::Note {at,text,height_mm,align,bold,italic,underline} => {
                let pos=sheet_to_screen(*at,paper,factor);
                let index=state.annotations.iter().position(|entry|std::ptr::eq(entry,annotation));
                let color=if index.is_some_and(|i|state.annotation_selected==Some(i)){Color32::from_rgb(35,105,220)}else{ink};
                let font_px=(*height_mm * factor).max(0.5);
                let anchor=match align {NoteAlign::Left=>egui::Align2::LEFT_BOTTOM,NoteAlign::Center=>egui::Align2::CENTER_BOTTOM,NoteAlign::Right=>egui::Align2::RIGHT_BOTTOM};
                for (line_no,line) in text.lines().enumerate() {
                    let line_pos=pos+Vec2::new(0.0,line_no as f32*font_px*1.3);
                    let family=egui::FontFamily::Proportional;
                    // Use standard proportional fallback; extra ink passes provide a
                    // reliable bold treatment without registering application fonts.
                    let font=egui::FontId::new(font_px,family);
                    painter.text(line_pos,anchor,line,font.clone(),color);
                    if *bold { painter.text(line_pos+Vec2::new(0.55,0.0),anchor,line,egui::FontId::proportional(font_px),color); }
                    if *italic { painter.text(line_pos+Vec2::new(0.8,-0.3),anchor,line,egui::FontId::proportional(font_px),color); }
                    if *underline {
                        let width=line.chars().count() as f32*font_px*0.52;
                        let x=match align {NoteAlign::Left=>line_pos.x,NoteAlign::Center=>line_pos.x-width/2.0,NoteAlign::Right=>line_pos.x-width};
                        painter.line_segment([Pos2::new(x,line_pos.y+2.0),Pos2::new(x+width,line_pos.y+2.0)],Stroke::new((font_px/18.0).max(0.7),color));
                    }
                }
            }
            DrawingAnnotation::Leader {target, text_at, text} => {
                let a = sheet_to_screen(*target, paper, factor);
                let b = sheet_to_screen(*text_at, paper, factor);
                let elbow = Pos2::new(b.x - 12.0, b.y);
                let index=state.annotations.iter().position(|entry| std::ptr::eq(entry,annotation));
                let hover=painter.ctx().pointer_hover_pos().and_then(|pos|hit_annotation(state,paper,factor,pos)).is_some_and(|(i,_)|Some(i)==index);
                let selected=index.is_some_and(|i|state.annotation_selected==Some(i));
                let color=if selected {Color32::from_rgb(35,105,220)} else if hover {Color32::from_rgb(0,190,220)} else {ink};
                let callout_stroke=Stroke::new(if selected || hover {2.0}else{1.0},color);
                painter.line_segment([a, elbow], callout_stroke);
                painter.line_segment([elbow, b], callout_stroke);
                let direction = (elbow - a).normalized();
                let side = Vec2::new(-direction.y, direction.x);
                painter.add(egui::Shape::convex_polygon(vec![a, a + direction * 9.0 + side * 3.0,
                    a + direction * 9.0 - side * 3.0], color, Stroke::NONE));
                let right_side = b.x >= a.x;
                painter.text(b + Vec2::new(if right_side {3.0} else {-3.0},-4.0),
                    if right_side {egui::Align2::LEFT_BOTTOM} else {egui::Align2::RIGHT_BOTTOM}, text,
                    sheet_font(3.0,factor), color);
            }
            DrawingAnnotation::Dimension {a,b,kind,offset} => {
                let p = sheet_to_screen(*a, paper, factor);
                let q = sheet_to_screen(*b, paper, factor);
                let (u,v,value) = match kind {
                    AnnotationTool::Horizontal => {
                        let y = p.y.min(q.y) - (*offset as f32 * factor).max(12.0);
                        (Pos2::new(p.x,y), Pos2::new(q.x,y), (b.x-a.x).abs())
                    }
                    AnnotationTool::Vertical => {
                        let x = p.x.max(q.x) + (*offset as f32 * factor).max(12.0);
                        (Pos2::new(x,p.y), Pos2::new(x,q.y), (b.y-a.y).abs())
                    }
                    _ => {
                        let delta = q-p;
                        let normal = if delta.length_sq() > 0.01 { Vec2::new(-delta.y, delta.x).normalized() } else { Vec2::ZERO };
                        let off = normal * (*offset as f32 * factor).max(12.0);
                        (p+off, q+off, ((b.x-a.x).powi(2)+(b.y-a.y).powi(2)).sqrt())
                    }
                };
                // Keep extension lines visually separate from the part geometry.
                // Both distances are in paper millimetres, converted by the sheet factor.
                for (origin, end) in [(p, u), (q, v)] {
                    let delta = end - origin;
                    let length = delta.length();
                    let gap = 1.5_f32 * factor;
                    let over = 2.0_f32 * factor;
                    if length > gap + 0.01 {
                        let direction = delta / length;
                        painter.line_segment(
                            [origin + direction * gap, end + direction * over],
                            Stroke::new(0.7, ink),
                        );
                    }
                }
                painter.line_segment([u,v], stroke);
                let d = v-u;
                if d.length_sq() > 0.01 {
                    let dir = d.normalized();
                    let side = Vec2::new(-dir.y,dir.x);
                    for (point, arrow) in [(u,dir),(v,-dir)] {
                        painter.add(egui::Shape::convex_polygon(vec![point,point+arrow*8.0+side*2.5,
                            point+arrow*8.0-side*2.5],ink,Stroke::NONE));
                    }
                }
                painter.text(u + (v-u)*0.5 + Vec2::new(0.0,-5.0),egui::Align2::CENTER_BOTTOM,
                    format!("{value:.2} REF"),sheet_font(3.0,factor),ink);
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
        if state.show_title {
            ui.separator();
            ui.heading("Title block");
            ui.label("Part / drawing title");
            ui.text_edit_singleline(&mut state.title_part);
            ui.label("Drawing number");
            ui.text_edit_singleline(&mut state.title_number);
            ui.label("Material");
            ui.text_edit_singleline(&mut state.title_material);
            ui.label("Drawn by");
            ui.text_edit_singleline(&mut state.title_author);
            ui.separator();
            ui.heading("Revision history");
            let mut remove=None;
            for (i,(rev,date,description)) in state.revisions.iter_mut().enumerate() {
                ui.horizontal(|ui|{
                    ui.add(egui::TextEdit::singleline(rev).desired_width(34.0).hint_text("Rev"));
                    ui.add(egui::TextEdit::singleline(date).desired_width(78.0).hint_text("Date"));
                    ui.add(egui::TextEdit::singleline(description).desired_width(145.0).hint_text("Description"));
                    if ui.small_button("Ã—").clicked(){remove=Some(i);}
                });
            }
            if let Some(i)=remove {state.revisions.remove(i);}
            if ui.button("+ Add revision").clicked() {
                state.revisions.push((String::new(),String::new(),String::new()));
            }
        }

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
    if editing {
        ui.label("Display style (per view)");
        if let Some(id) = state.edit_view {
            if let Some(view) = state.placed.iter_mut().find(|v| v.id == id) {
                egui::ComboBox::from_id_salt("drawing_view_display_v0124")
                    .selected_text(view.display.label())
                    .show_ui(ui, |ui| {
                        for style in [DisplayStyle::ExternalOnly, DisplayStyle::Visible, DisplayStyle::VisibleHidden, DisplayStyle::Shaded, DisplayStyle::ShadedEdges, DisplayStyle::ShadedHidden] {
                            ui.selectable_value(&mut view.display, style, style.label());
                        }
                    });
                ui.checkbox(&mut view.visible, "View visible");
                ui.checkbox(&mut view.tangent_edges, "Tangent edges (requires HLR refresh)");
                ui.checkbox(&mut view.centre_marks, "Centre marks");
                ui.checkbox(&mut view.centre_lines, "Centre lines");
                ui.checkbox(&mut view.scale_locked, "Inherit parent scale");
                ui.label("View label");
                ui.text_edit_singleline(&mut view.label);
                ui.checkbox(&mut view.show_label, "Show label");
                ui.small("Shaded modes use a separate depth-buffered mesh image; exact HLR curves remain unchanged.");
                if view.display == DisplayStyle::ExternalOnly {
                    ui.small("External-only silhouette extraction is not yet implemented; showing visible edges.");
                }
            }
        }
    } else {
        ui.checkbox(&mut state.show_hidden, "Show hidden lines in new views");
    }
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
        // Remove linked descendants to avoid orphaned projected views.
        let mut removed = vec![id];
        loop {
            let before = removed.len();
            for v in &state.placed {
                if v.parent.is_some_and(|p| removed.contains(&p)) && !removed.contains(&v.id) { removed.push(v.id); }
            }
            if removed.len() == before { break; }
        }
        state.placed.retain(|view| !removed.contains(&view.id));
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
    if let Some(spec) = &view.section {
        // Hit-test the actual section geometry, not the original parent
        // projection. The latter can be much wider than the cut profile.
        for [a,b] in section_segments(&state.section_mesh,view.orientation,spec.cut_x) {
            for point in [a,b] {
                let pos=Pos2::new(p.x+point.x as f32*ratio,p.y-point.y as f32*ratio);
                if !pos.x.is_finite() || !pos.y.is_finite() {continue;}
                if !has_geometry {min=pos;max=pos;has_geometry=true;}
                else {min.x=min.x.min(pos.x);min.y=min.y.min(pos.y);
                      max.x=max.x.max(pos.x);max.y=max.y.max(pos.y);}
            }
        }
        if !has_geometry {return Rect::from_center_size(p,Vec2::splat(28.0));}
        return Rect::from_min_max(min,max).expand(10.0);
    }
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

// Drawing-view geometry cleanup. A 3D arc may project to an exactly straight
// 2D silhouette. Only collapse it when *all* samples lie on the chord and
// advance monotonically along it; this rejects bowed and reversing curves.
// No view bounding boxes are used to construct selectable geometry.
fn straight_projected_stroke(stroke: &mut ProjectedStroke) {
    if matches!(stroke.curve, Seg2::Line {..}) { return; }
    const FLAT_MM: f64 = 0.005;
    let a = stroke.curve.point_at(0.0);
    let b = stroke.curve.point_at(1.0);
    let dx = b.x-a.x;
    let dy = b.y-a.y;
    let len = dx.hypot(dy);
    if !len.is_finite() || len < FLAT_MM { return; }
    let mut previous = -FLAT_MM;
    for k in 0..=32 {
        let p = stroke.curve.point_at(k as f64/32.0);
        let along = ((p.x-a.x)*dx+(p.y-a.y)*dy)/len;
        let across = ((p.x-a.x)*dy-(p.y-a.y)*dx).abs()/len;
        if !along.is_finite() || across > FLAT_MM
            || along < previous-FLAT_MM || along > len+FLAT_MM { return; }
        previous = along;
    }
    stroke.curve = Seg2::Line { a, b };
}

// Merge contiguous, collinear 2D strokes of identical HLR visibility.
// Projected flattened curves and true straight edges are treated equally.
fn consolidate_projected_lines(mut strokes: Vec<ProjectedStroke>) -> Vec<ProjectedStroke> {
    const GAP_MM: f64 = 0.005;
    const ANGLE_SIN: f64 = 0.0001;
    for stroke in &mut strokes { straight_projected_stroke(stroke); }
    let mut changed = true;
    while changed {
        changed = false;
        'pairs: for i in 0..strokes.len() {
            let Seg2::Line { a, b } = &strokes[i].curve else { continue };
            let (a,b) = (*a,*b);
            let ux = b.x-a.x; let uy = b.y-a.y;
            let len = ux.hypot(uy);
            if len <= GAP_MM { continue; }
            for j in i+1..strokes.len() {
                if strokes[i].hidden != strokes[j].hidden
                    || strokes[i].uncertain != strokes[j].uncertain { continue; }
                let Seg2::Line { a:c,b:d } = &strokes[j].curve else {continue};
                let (c,d)=(*c,*d);
                let vx=d.x-c.x; let vy=d.y-c.y;
                let vlen=vx.hypot(vy);
                if vlen<=GAP_MM || (ux*vy-uy*vx).abs() > ANGLE_SIN*len*vlen {continue;}
                if ((c.x-a.x)*uy-(c.y-a.y)*ux).abs()/len > GAP_MM
                    || ((d.x-a.x)*uy-(d.y-a.y)*ux).abs()/len > GAP_MM {continue;}
                let project=|p:ModelVec2| ((p.x-a.x)*ux+(p.y-a.y)*uy)/len;
                let lo=project(c).min(project(d));
                let hi=project(c).max(project(d));
                if lo>len+GAP_MM || hi< -GAP_MM {continue;}
                let start=lo.min(0.0);
                let end=hi.max(len);
                let from=ModelVec2::new(a.x+ux/len*start,a.y+uy/len*start);
                let to=ModelVec2::new(a.x+ux/len*end,a.y+uy/len*end);
                strokes[i].curve=Seg2::Line {a:from,b:to};
                let merged = strokes.remove(j);
                strokes[i].sources.extend(merged.sources);
                changed=true;
                break 'pairs;
            }
        }
    }
    strokes
}

// Cached per active-document revision: the exact kernel HLR is expensive and
// must never be recomputed on every egui repaint.
// Geometric reassociation of mesh-section edges. A score based on position and
// direction is used only when there is a unique, plausible candidate.
fn resolve_section_edge(old:[ModelVec2;2], edges:&[[ModelVec2;2]]) -> Option<[ModelVec2;2]> {
    let dist=|a:ModelVec2,b:ModelVec2| (a.x-b.x).hypot(a.y-b.y);
    let odx=old[1].x-old[0].x; let ody=old[1].y-old[0].y;
    let olen=odx.hypot(ody); if olen<1e-7 {return None;}
    let mut candidates=Vec::new();
    for edge in edges {
        let dx=edge[1].x-edge[0].x;let dy=edge[1].y-edge[0].y;
        let len=dx.hypot(dy);if len<1e-7 {continue;}
        if (odx*dy-ody*dx).abs()/(olen*len)>0.025 {continue;}
        let direct=dist(old[0],edge[0])+dist(old[1],edge[1]);
        let flipped=dist(old[0],edge[1])+dist(old[1],edge[0]);
        let candidate=if direct<=flipped {*edge} else {[edge[1],edge[0]]};
        candidates.push((direct.min(flipped),candidate));
    }
    candidates.sort_by(|a,b|a.0.total_cmp(&b.0));
    let (score,best)=*candidates.first()?;
    // Do not jump to a different feature if the original one disappeared.
    if score>olen.max(10.0)*1.5 {return None;}
    if candidates.get(1).is_some_and(|c| (c.0-score).abs()<1e-4) {return None;}
    Some(best)
}

fn resolve_section_vertex(old:ModelVec2,edges:&[[ModelVec2;2]])->Option<ModelVec2>{
    let mut points:Vec<ModelVec2>=Vec::new();
    for edge in edges {for &p in edge {if !points.iter().any(|q|(q.x-p.x).hypot(q.y-p.y)<1e-5){points.push(p);}}}
    points.sort_by(|a,b| (a.x-old.x).hypot(a.y-old.y).total_cmp(&(b.x-old.x).hypot(b.y-old.y)));
    let best=*points.first()?;
    if (best.x-old.x).hypot(best.y-old.y)>15.0{return None;}
    if points.get(1).is_some_and(|p| ((p.x-old.x).hypot(p.y-old.y)-(best.x-old.x).hypot(best.y-old.y)).abs()<1e-4){return None;}
    Some(best)
}

fn refresh_projection(state: &mut State, session: &solidcraft_engine::Session, ctx: &egui::Context) {
    let key = (session.uid, session.revision);
    if state.model_revision == Some(key) { return; }
    let previous_revision = state.model_revision;
    // Upgrade existing 9j section dimensions while the OLD mesh still exists.
    // This permits simple edge dimensions created before 9m to follow edits.
    if previous_revision.is_some() {
        let placed=&state.placed;
        let mesh=&state.section_mesh;
        for ann in &mut state.annotations {
            let DrawingAnnotation::Smart{view_id,geometry,binding,..}=ann else {continue};
            if binding.is_some(){continue;}
            let Some(view)=placed.iter().find(|v|v.id==*view_id) else {continue};
            let Some(spec)=&view.section else {continue};
            let SmartGeometry::Distance{a,b,kind:SmartKind::Aligned}=geometry else {continue};
            let edges=section_segments(mesh,view.orientation,spec.cut_x);
            let old=[*a,*b];
            if let Some(matched)=resolve_section_edge(old,&edges) {
                let error=(matched[0].x-a.x).hypot(matched[0].y-a.y)
                    +(matched[1].x-b.x).hypot(matched[1].y-b.y);
                if error<1e-3 {*binding=Some(DimensionBinding::SectionSingle(old));}
            }
        }
    }
    // Do not silently present cached dimensions as valid after model edits.
    // The current SmartGeometry only stores 2D coordinates, not stable
    // topology IDs. Until those IDs exist, automatic reassociation is unsafe.
    state.model_revision = Some(key);
    state.projected.clear();
    state.section_mesh.clear();
    state.shaded.clear();
    state.hlr_debug.clear();
    state.projection_error = None;
    let model = session.world_state();
    if model.bodies.is_empty() { return; }
    for body in &model.bodies {
        let mesh=body.mesh();
        for ids in &mesh.triangles {
            if let (Some(&a),Some(&b),Some(&c))=(mesh.positions.get(ids[0] as usize),mesh.positions.get(ids[1] as usize),mesh.positions.get(ids[2] as usize)) {
                state.section_mesh.push([a,b,c]);
            }
        }
    }
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
    // Rasterize the existing display meshes into a dedicated depth-buffered
    // texture. This does not touch Design's GPU renderer or HLR geometry.
    for orientation in NamedView::ALL {
        let mut triangles = Vec::new();
        for body in &model.bodies {
            let mesh = body.mesh();
            for ids in &mesh.triangles {
                if let (Some(&a), Some(&b), Some(&c)) = (mesh.positions.get(ids[0] as usize), mesh.positions.get(ids[1] as usize), mesh.positions.get(ids[2] as usize)) {
                    triangles.push([a, b, c]);
                }
            }
        }
        if let Some(shade) = raster_shaded_view(&triangles, orientation, ctx) {
            state.shaded.insert(orientation, shade);
        }
    }
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
                                id: ProjectedEntityId(strokes.len() as u64),
                                sources: vec![HlrSourcePath { line: line_index, piece: piece_index, segment: segment_index }],
                                curve: seg,
                                hidden: piece.visibility == hlr::Visibility::Hidden,
                                uncertain: piece.visibility == hlr::Visibility::Uncertain,
                            });
                        }
                    }
                }
                state.projected.insert(orientation, consolidate_projected_lines(strokes));
                state.hlr_debug.insert(orientation, debug_segments);
            }
            Err(e) => { state.projection_error = Some(format!("Projection: {e}")); break; }
        }
    }
    // Best-effort reattachment for dimensions created from a single projected
    // straight edge. HLR source paths are NOT stable topological IDs; therefore
    // fail closed if the source set is ambiguous, missing, or changes direction.
    // Do not change legacy dimensions without a recorded binding.
    if let Some((old_uid, old_revision)) = previous_revision {
        if old_uid == session.uid && old_revision != session.revision {
            let mut unresolved=0usize;
            for ann in &mut state.annotations {
                let DrawingAnnotation::Smart {view_id,geometry,binding,..}=ann else {continue};
                let Some(binding)=binding.as_ref() else {unresolved+=1;continue};
                let Some(view)=state.placed.iter().find(|v|v.id==*view_id) else {unresolved+=1;continue};
                if let Some(spec)=&view.section {
                    let edges=section_segments(&state.section_mesh,view.orientation,spec.cut_x);
                    let replacement=match binding {
                        DimensionBinding::SectionSingle(old)=>resolve_section_edge(*old,&edges).map(|[a,b]|SmartGeometry::Distance{a,b,kind:SmartKind::Aligned}),
                        DimensionBinding::SectionPair(first,second)=>resolve_section_edge(*first,&edges)
                            .zip(resolve_section_edge(*second,&edges))
                            .and_then(|([a,b],[c,d])|smart_pair_lines(a,b,c,d)),
                        DimensionBinding::SectionVertices(a,b)=>resolve_section_vertex(*a,&edges)
                            .zip(resolve_section_vertex(*b,&edges))
                            .and_then(|(a,b)|match geometry {SmartGeometry::Distance{kind,..}=>Some(SmartGeometry::Distance{a,b,kind:*kind}),_=>None}),
                        _=>None,
                    };
                    if let Some(next)=replacement {
                        if std::mem::discriminant(&next)==std::mem::discriminant(geometry) {*geometry=next;}
                        else {unresolved+=1;}
                    } else {unresolved+=1;}
                    continue;
                }
                let Some(strokes)=state.projected.get(&view.orientation) else {unresolved+=1;continue};
                let resolve=|paths:&Vec<HlrSourcePath>| -> Option<(ModelVec2,ModelVec2)> {
                    let mut candidates=strokes.iter().filter(|st| &st.sources==paths);
                    let stroke=candidates.next()?;
                    if candidates.next().is_some(){return None;}
                    match &stroke.curve {Seg2::Line{a,b}=>Some((*a,*b)),_=>None}
                };
                let replacement=match binding {
                    DimensionBinding::Single(paths) => {
                        let Some((na,nb))=resolve(paths) else {unresolved+=1;continue};
                        let SmartGeometry::Distance{a,b,kind:SmartKind::Aligned}=geometry else {unresolved+=1;continue};
                        let old_dx=b.x-a.x;let old_dy=b.y-a.y;
                        let new_dx=nb.x-na.x;let new_dy=nb.y-na.y;
                        let ol=old_dx.hypot(old_dy);let nl=new_dx.hypot(new_dy);
                        if ol<1e-7||nl<1e-7||(old_dx*new_dy-old_dy*new_dx).abs()/(ol*nl)>0.01 {
                            unresolved+=1;continue;
                        }
                        let same=(a.x-na.x).hypot(a.y-na.y)+(b.x-nb.x).hypot(b.y-nb.y);
                        let flipped=(a.x-nb.x).hypot(a.y-nb.y)+(b.x-na.x).hypot(b.y-na.y);
                        if same<=flipped {Some(SmartGeometry::Distance{a:na,b:nb,kind:SmartKind::Aligned})}
                        else {Some(SmartGeometry::Distance{a:nb,b:na,kind:SmartKind::Aligned})}
                    }
                    DimensionBinding::Pair(first,second) => {
                        let Some((a,b))=resolve(first) else {unresolved+=1;continue};
                        let Some((c,d))=resolve(second) else {unresolved+=1;continue};
                        let Some(new_geometry)=smart_pair_lines(a,b,c,d) else {unresolved+=1;continue};
                        // Keep angle dimensions as angles, spacing dimensions as distances.
                        if std::mem::discriminant(&new_geometry)!=std::mem::discriminant(geometry) {
                            unresolved+=1;continue;
                        }
                        Some(new_geometry)
                    }
                    _ => {unresolved+=1;continue;}
                };
                if let Some(next)=replacement {*geometry=next;}
            }
            state.smart_dimensions_stale=unresolved>0;
            if unresolved>0 {state.export_message=Some(format!(
                "Model changed: {unresolved} dimension(s) need manual verification (no stable topology reference)."));}
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

/// Independent software Z-buffer: robust against overlapping and crossing faces.
/// Mesh rasterization runs only when the document revision changes.
fn raster_shaded_view(triangles: &[[Vec3; 3]], orientation: NamedView, ctx: &egui::Context) -> Option<DrawingShade> {
    let basis = orientation.basis();
    let depth_axis = basis.x.cross(basis.y);
    let mut projected = Vec::new();
    let (mut xmin, mut ymin) = (f64::INFINITY, f64::INFINITY);
    let (mut xmax, mut ymax) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    let light = Vec3::new(0.35, 0.45, 0.82);
    for tri in triangles {
        let normal = (tri[1] - tri[0]).cross(tri[2] - tri[0]);
        let norm = normal.dot(normal).sqrt();
        if !norm.is_finite() || norm < 1e-12 { continue; }
        let n = normal / norm;
        let pts = tri.map(|v| [v.dot(basis.x), v.dot(basis.y), v.dot(depth_axis)]);
        if pts.iter().flatten().any(|v| !v.is_finite()) { continue; }
        for p in pts { xmin = xmin.min(p[0]); xmax = xmax.max(p[0]); ymin = ymin.min(p[1]); ymax = ymax.max(p[1]); }
        let illumination = (0.58 + 0.35 * n.dot(light).abs()).clamp(0.0, 1.0);
        projected.push((pts, (illumination * 225.0) as u8));
    }
    if projected.is_empty() || xmax - xmin < 1e-9 || ymax - ymin < 1e-9 { return None; }
    let spanx = xmax - xmin;
    let spany = ymax - ymin;
    let max_dim = 1024.0;
    let width = ((max_dim * spanx / spanx.max(spany)).round() as usize).clamp(2, 1024);
    let height = ((max_dim * spany / spanx.max(spany)).round() as usize).clamp(2, 1024);
    let mut depth = vec![f64::NEG_INFINITY; width * height];
    let mut image = egui::ColorImage::new([width, height], vec![Color32::TRANSPARENT; width * height]);
    for (pts, shade) in projected {
        let v = pts.map(|p| [
            (p[0] - xmin) / spanx * (width - 1) as f64,
            (ymax - p[1]) / spany * (height - 1) as f64,
            p[2],
        ]);
        let area = (v[1][0]-v[0][0])*(v[2][1]-v[0][1])-(v[1][1]-v[0][1])*(v[2][0]-v[0][0]);
        if area.abs() < 1e-9 { continue; }
        let minx = v.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min).floor().max(0.0) as usize;
        let maxx = v.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max).ceil().min((width-1) as f64) as usize;
        let miny = v.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min).floor().max(0.0) as usize;
        let maxy = v.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max).ceil().min((height-1) as f64) as usize;
        if minx > maxx || miny > maxy { continue; }
        for y in miny..=maxy { for x in minx..=maxx {
            let px = x as f64 + 0.5;
            let py = y as f64 + 0.5;
            let e0 = (v[1][0]-px)*(v[2][1]-py)-(v[1][1]-py)*(v[2][0]-px);
            let e1 = (v[2][0]-px)*(v[0][1]-py)-(v[2][1]-py)*(v[0][0]-px);
            let e2 = (v[0][0]-px)*(v[1][1]-py)-(v[0][1]-py)*(v[1][0]-px);
            let a = e0 / area; let b = e1 / area; let c = e2 / area;
            if a < -1e-7 || b < -1e-7 || c < -1e-7 { continue; }
            let z = a*v[0][2] + b*v[1][2] + c*v[2][2];
            let idx = y * width + x;
            if z > depth[idx] {
                depth[idx] = z;
                image.pixels[idx] = Color32::from_rgb(shade, shade, shade.saturating_add(5));
            }
        }}
    }
    let texture = ctx.load_texture(format!("drawing-shade-{orientation:?}"), image, egui::TextureOptions::LINEAR);
    Some(DrawingShade { texture, min: ModelVec2::new(xmin, ymin), max: ModelVec2::new(xmax, ymax) })
}


// V0.1.28f7c: native Windows file dialogs; no manual path fields.
fn drawing_open_dialog() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new().add_filter("SolidCraft drawing", &["scdraw"]).pick_file()
}
fn drawing_save_dialog(current: Option<&str>) -> Option<std::path::PathBuf> {
    let mut dialog = rfd::FileDialog::new().add_filter("SolidCraft drawing", &["scdraw"]);
    if let Some(path) = current.filter(|p| !p.is_empty()) {
        let p = std::path::Path::new(path);
        if let Some(parent) = p.parent() { dialog = dialog.set_directory(parent); }
        if let Some(name) = p.file_name() { dialog = dialog.set_file_name(name.to_string_lossy().to_string()); }
    }
    dialog.save_file().map(|mut p| { if p.extension().is_none() {p.set_extension("scdraw");} p })
}
fn drawing_model_dialog() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new().add_filter("SolidCraft model", &["sc"]).pick_file()
}

// V0.1.28f7b: sidecar persistence. HLR and GPU caches intentionally excluded.
fn save_drawing_file(state: &State, path: &std::path::Path) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    if path.as_os_str().is_empty() || path.extension().and_then(|e|e.to_str()) != Some("scdraw") {
        return Err(Error::new(ErrorKind::InvalidInput, "Specify a .scdraw filename"));
    }
    if state.drawing_model_path.trim().is_empty() {
        return Err(Error::new(ErrorKind::InvalidInput, "Specify the referenced .sc model path"));
    }
    let model_path=std::path::Path::new(&state.drawing_model_path);
    let drawing_dir=path.parent().unwrap_or_else(||std::path::Path::new("."));
    if model_path.parent() != Some(drawing_dir) && model_path.canonicalize().ok().and_then(|p|p.parent().map(|d|d.to_owned())) != drawing_dir.canonicalize().ok() {
        return Err(Error::new(ErrorKind::InvalidInput, "Model and drawing must be in the same folder"));
    }
    let model_name=model_path.file_name().and_then(|s|s.to_str()).ok_or_else(||Error::new(ErrorKind::InvalidInput,"Invalid model filename"))?;
    let sheets: Vec<_> = if state.sheets.is_empty() {
        vec![serde_json::json!({"name":"Sheet 1","drawing":serialize_sheet(&state.snapshot())})]
    } else {
        state.sheets.iter().enumerate().map(|(i,s)|serde_json::json!({
            "name":s.name,"drawing":serialize_sheet(&if i==state.active_sheet {state.snapshot()} else {s.drawing.clone()})
        })).collect()
    };
    let document=serde_json::json!({"format":"solidcraft-drawing","version":2,
        "model":{"path":model_name},"active_sheet":state.active_sheet,"sheets":sheets});
    let bytes=serde_json::to_vec_pretty(&document).map_err(std::io::Error::other)?;
    let temp=path.with_extension("scdraw.tmp");
    std::fs::write(&temp,bytes)?;
    // Do not delete a valid drawing before replacement. On Windows rename cannot overwrite.
    if path.exists() {
        let backup=path.with_extension("scdraw.bak");
        if backup.exists() { std::fs::remove_file(&backup)?; }
        std::fs::rename(path,&backup)?;
        if let Err(e)=std::fs::rename(&temp,path) { let _=std::fs::rename(&backup,path); return Err(e); }
    } else { std::fs::rename(&temp,path)?; }
    Ok(())
}
fn load_drawing_file(state: &mut State, path: &std::path::Path) -> std::io::Result<()> {
    use std::io::{Error,ErrorKind};
    let doc:serde_json::Value=serde_json::from_slice(&std::fs::read(path)?).map_err(Error::other)?;
    let invalid=||Error::new(ErrorKind::InvalidData,"Invalid .scdraw file");
    if doc["format"]!="solidcraft-drawing" {return Err(invalid());}
    let version=doc["version"].as_u64().ok_or_else(invalid)?;
    if version!=1 && version!=2 {return Err(invalid());}
    let model=doc["model"]["path"].as_str().ok_or_else(invalid)?;
    let model_name=std::path::Path::new(model);
    if model_name.file_name().is_none() {return Err(invalid());}
    let source=path.parent().unwrap_or_else(||std::path::Path::new(".")).join(model_name.file_name().unwrap());
    if !source.is_file() {return Err(Error::new(ErrorKind::NotFound,format!("Referenced model not found: {}",source.display())));}
    let mut sheets=Vec::new();
    if version==1 {
        sheets.push(NamedSheet{name:"Sheet 1".into(),drawing:deserialize_sheet(&doc)?});
    } else {
        for s in doc["sheets"].as_array().ok_or_else(invalid)? {
            let name=s["name"].as_str().ok_or_else(invalid)?;
            sheets.push(NamedSheet{name:name.into(),drawing:deserialize_sheet(&s["drawing"])?});
        }
    }
    if sheets.is_empty() {return Err(invalid());}
    let active=(doc["active_sheet"].as_u64().unwrap_or(0) as usize).min(sheets.len()-1);
    let snapshot=sheets[active].drawing.clone();
    state.sheets=sheets;
    state.active_sheet=active;
    state.restore(snapshot);
    state.undo.clear();state.redo.clear();
    state.projected.clear();state.hlr_debug.clear();state.shaded.clear();state.model_revision=None;state.smart_dimensions_stale=false;
    state.drawing_model_path=source.to_string_lossy().into_owned();
    state.drawing_file_path=path.to_string_lossy().into_owned();
    Ok(())
}

fn serialize_sheet(snapshot: &DrawingSnapshot) -> serde_json::Value {
    let views: Vec<_> = snapshot.placed.iter().map(|v| serde_json::json!({
        "id":v.id,"orientation":format!("{:?}",v.orientation),"x":v.at.x,"y":v.at.y,
        "scale":v.frame.scale().factor(),"display":format!("{:?}",v.display),
        "parent":v.parent,"label":v.label,"visible":v.visible,"show_label":v.show_label,
        "tangent_edges":v.tangent_edges,"centre_marks":v.centre_marks,
        "centre_lines":v.centre_lines,"scale_locked":v.scale_locked,
        "section":v.section.as_ref().map(|section|serde_json::json!({"cut_x":section.cut_x,"tag":section.tag}))
    })).collect();
    let annotations: Vec<_> = snapshot.annotations.iter().map(|a| match a {
        DrawingAnnotation::CentreMark{view_id,center,radius} => serde_json::json!({"kind":"centre_mark","view_id":view_id,"center":[center.x,center.y],"radius":radius}),
        DrawingAnnotation::CentreLine{view_id,a,b} => serde_json::json!({"kind":"centre_line","view_id":view_id,"a":[a.x,a.y],"b":[b.x,b.y]}),
        DrawingAnnotation::Note {at,text,height_mm,align,bold,italic,underline} => serde_json::json!({"kind":"note","at":[at.x,at.y],"text":text,"height_mm":height_mm,"align":format!("{:?}",align),"bold":bold,"italic":italic,"underline":underline}),
        DrawingAnnotation::Leader {target,text_at,text} => serde_json::json!({"kind":"leader","target":[target.x,target.y],"text_at":[text_at.x,text_at.y],"text":text}),
        DrawingAnnotation::Dimension {a,b,kind,offset} => serde_json::json!({"kind":"dimension","a":[a.x,a.y],"b":[b.x,b.y],"mode":format!("{:?}",kind),"offset":offset}),
        DrawingAnnotation::Smart {view_id,geometry,offset,text_shift,tolerance,binding} => {
            let geo=match geometry {
                SmartGeometry::Radius {center,radius,diameter,angle} => serde_json::json!({"kind":"radius","center":[center.x,center.y],"radius":radius,"diameter":diameter,"angle":angle}),
                SmartGeometry::Distance {a,b,kind} => serde_json::json!({"kind":"distance","a":[a.x,a.y],"b":[b.x,b.y],"mode":format!("{:?}",kind)}),
                SmartGeometry::Angle {vertex,a,b,sector} => serde_json::json!({"kind":"angle","vertex":[vertex.x,vertex.y],"a":[a.x,a.y],"b":[b.x,b.y],"sector":sector})
            };
            serde_json::json!({"kind":"smart","view_id":view_id,"geometry":geo,"offset":offset,
                "text_shift":[text_shift.x,text_shift.y],"binding":binding.as_ref().map(|binding|match binding {
                    DimensionBinding::Single(sources)=>serde_json::json!({"type":"single","sources":sources.iter().map(|p|[p.line,p.piece,p.segment]).collect::<Vec<_>>()}),
                    DimensionBinding::Pair(first,second)=>serde_json::json!({"type":"pair","first":first.iter().map(|p|[p.line,p.piece,p.segment]).collect::<Vec<_>>(),"second":second.iter().map(|p|[p.line,p.piece,p.segment]).collect::<Vec<_>>()}),
                    DimensionBinding::SectionSingle([a,b])=>serde_json::json!({"type":"section_single","a":[a.x,a.y],"b":[b.x,b.y]}),
                    DimensionBinding::SectionPair([a,b],[c,d])=>serde_json::json!({"type":"section_pair","a":[a.x,a.y],"b":[b.x,b.y],"c":[c.x,c.y],"d":[d.x,d.y]}),
                    DimensionBinding::SectionVertices(a,b)=>serde_json::json!({"type":"section_vertices","a":[a.x,a.y],"b":[b.x,b.y]})
                }),"tolerance":{"mode":format!("{:?}",tolerance.mode),"upper":tolerance.upper,"lower":tolerance.lower,"precision":tolerance.precision}})
        }
    }).collect();
    serde_json::json!({"sheet":{"size":snapshot.sheet_size,"landscape":snapshot.landscape,
        "border":snapshot.show_border,"title":snapshot.show_title,
        "title_part":snapshot.title_part,"title_number":snapshot.title_number,
        "title_material":snapshot.title_material,"title_author":snapshot.title_author,
        "revisions":snapshot.revisions,"first_angle":snapshot.projection_first_angle},
        "views":views,"annotations":annotations})
}

fn deserialize_sheet(sheet: &serde_json::Value) -> std::io::Result<DrawingSnapshot> {
    use std::io::{Error,ErrorKind};
    let invalid=||Error::new(ErrorKind::InvalidData,"Invalid drawing sheet");
    let point=|v:&serde_json::Value|->std::io::Result<ModelVec2>{
        Ok(ModelVec2::new(v[0].as_f64().ok_or_else(invalid)?,v[1].as_f64().ok_or_else(invalid)?))
    };
    let mut views=Vec::new();
    for v in sheet["views"].as_array().ok_or_else(invalid)? {
        let orientation=match v["orientation"].as_str().ok_or_else(invalid)? {
            "Front"=>NamedView::Front,"Back"=>NamedView::Back,"Left"=>NamedView::Left,"Right"=>NamedView::Right,
            "Top"=>NamedView::Top,"Bottom"=>NamedView::Bottom,_=>return Err(invalid())};
        let factor=v["scale"].as_f64().ok_or_else(invalid)?;
        let scale=Scale::new(factor,1.0).map_err(|_|invalid())?;
        let at=ModelVec2::new(v["x"].as_f64().ok_or_else(invalid)?,v["y"].as_f64().ok_or_else(invalid)?);
        let frame=ViewFrame::new(orientation.basis(),scale,ModelVec2::ZERO,at).map_err(|_|invalid())?;
        let display=match v["display"].as_str().unwrap_or("Visible") {
            "ExternalOnly"=>DisplayStyle::ExternalOnly,"VisibleHidden"=>DisplayStyle::VisibleHidden,
            "Shaded"=>DisplayStyle::Shaded,"ShadedEdges"=>DisplayStyle::ShadedEdges,
            "ShadedHidden"=>DisplayStyle::ShadedHidden,_=>DisplayStyle::Visible};
        views.push(PlacedView{id:v["id"].as_u64().ok_or_else(invalid)?,frame,orientation,at,
            show_label:v["show_label"].as_bool().unwrap_or(true),visible:v["visible"].as_bool().unwrap_or(true),
            display,parent:v["parent"].as_u64(),label:v["label"].as_str().unwrap_or("").into(),
            tangent_edges:v["tangent_edges"].as_bool().unwrap_or(true),centre_marks:v["centre_marks"].as_bool().unwrap_or(false),
            centre_lines:v["centre_lines"].as_bool().unwrap_or(false),scale_locked:v["scale_locked"].as_bool().unwrap_or(false),
            section:v["section"]["cut_x"].as_f64().map(|cut_x|SectionSpec{cut_x,tag:v["section"]["tag"].as_str().unwrap_or("A").to_owned()})});
    }
    let mut annotations=Vec::new();
    for a in sheet["annotations"].as_array().ok_or_else(invalid)? {
        let ann=match a["kind"].as_str().ok_or_else(invalid)? {
            "centre_mark"=>DrawingAnnotation::CentreMark{view_id:a["view_id"].as_u64().ok_or_else(invalid)?,center:point(&a["center"])?,radius:a["radius"].as_f64().ok_or_else(invalid)?},
            "centre_line"=>DrawingAnnotation::CentreLine{view_id:a["view_id"].as_u64().ok_or_else(invalid)?,a:point(&a["a"])?,b:point(&a["b"])?},
            "note"=>DrawingAnnotation::Note{at:point(&a["at"])?,text:a["text"].as_str().ok_or_else(invalid)?.into(),height_mm:a["height_mm"].as_f64().unwrap_or(3.5).clamp(1.0,25.0) as f32,align:match a["align"].as_str().unwrap_or("Left") {"Center"=>NoteAlign::Center,"Right"=>NoteAlign::Right,_=>NoteAlign::Left},bold:a["bold"].as_bool().unwrap_or(false),italic:a["italic"].as_bool().unwrap_or(false),underline:a["underline"].as_bool().unwrap_or(false)},
            "leader"=>DrawingAnnotation::Leader{target:point(&a["target"])?,text_at:point(&a["text_at"])?,text:a["text"].as_str().ok_or_else(invalid)?.into()},
            "dimension"=>DrawingAnnotation::Dimension{a:point(&a["a"])?,b:point(&a["b"])?,kind:match a["mode"].as_str().unwrap_or("Aligned") {"Horizontal"=>AnnotationTool::Horizontal,"Vertical"=>AnnotationTool::Vertical,_=>AnnotationTool::Aligned},offset:a["offset"].as_f64().ok_or_else(invalid)?},
            "smart"=>{
                let g=&a["geometry"];
                let geometry=match g["kind"].as_str().ok_or_else(invalid)? {
                    "radius"=>SmartGeometry::Radius{center:point(&g["center"])?,radius:g["radius"].as_f64().ok_or_else(invalid)?,diameter:g["diameter"].as_bool().ok_or_else(invalid)?,angle:g["angle"].as_f64().ok_or_else(invalid)?},
                    "distance"=>SmartGeometry::Distance{a:point(&g["a"])?,b:point(&g["b"])?,kind:match g["mode"].as_str().unwrap_or("Aligned") {"Horizontal"=>SmartKind::Horizontal,"Vertical"=>SmartKind::Vertical,_=>SmartKind::Aligned}},
                    "angle"=>SmartGeometry::Angle{vertex:point(&g["vertex"])?,a:point(&g["a"])?,b:point(&g["b"])?,sector:g["sector"].as_u64().ok_or_else(invalid)? as u8},
                    _=>return Err(invalid())};
                let t=&a["tolerance"];
                let mode=match t["mode"].as_str().unwrap_or("None") {"Symmetric"=>ToleranceMode::Symmetric,"Bilateral"=>ToleranceMode::Bilateral,"Limits"=>ToleranceMode::Limits,"Basic"=>ToleranceMode::Basic,"Reference"=>ToleranceMode::Reference,_=>ToleranceMode::None};
                DrawingAnnotation::Smart{view_id:a["view_id"].as_u64().ok_or_else(invalid)?,geometry,
                    offset:a["offset"].as_f64().ok_or_else(invalid)?,text_shift:point(&a["text_shift"])?,
                    tolerance:DimensionTolerance{mode,upper:t["upper"].as_f64().ok_or_else(invalid)?,lower:t["lower"].as_f64().ok_or_else(invalid)?,precision:t["precision"].as_u64().ok_or_else(invalid)? as usize},binding:{
                        let parse=|v:&serde_json::Value| -> Option<Vec<HlrSourcePath>> {
                            v.as_array()?.iter().map(|p|Some(HlrSourcePath{
                                line:p[0].as_u64()? as usize,piece:p[1].as_u64()? as usize,
                                segment:p[2].as_u64()? as usize,
                            })).collect()
                        };
                        let v=&a["binding"];
                        if v.is_array(){parse(v).map(DimensionBinding::Single)}
                        else if v["type"]=="single" {parse(&v["sources"]).map(DimensionBinding::Single)}
                        else if v["type"]=="pair" {parse(&v["first"]).zip(parse(&v["second"])).map(|(x,y)|DimensionBinding::Pair(x,y))}
                        else if v["type"]=="section_single" {point(&v["a"]).ok().zip(point(&v["b"]).ok()).map(|(a,b)|DimensionBinding::SectionSingle([a,b]))}
                        else if v["type"]=="section_pair" {point(&v["a"]).ok().zip(point(&v["b"]).ok()).zip(point(&v["c"]).ok().zip(point(&v["d"]).ok())).map(|((a,b),(c,d))|DimensionBinding::SectionPair([a,b],[c,d]))}
                        else if v["type"]=="section_vertices" {point(&v["a"]).ok().zip(point(&v["b"]).ok()).map(|(a,b)|DimensionBinding::SectionVertices(a,b))}
                        else {None}
                    }}
            },_=>return Err(invalid())};
        annotations.push(ann);
    }
    Ok(DrawingSnapshot {
        next_view_id: views.iter().map(|v|v.id).max().unwrap_or(0)+1,
        placed:views, annotations, selected:None,
        sheet_size:(sheet["sheet"]["size"].as_u64().unwrap_or(1) as usize).min(SHEETS.len()-1),
        landscape:sheet["sheet"]["landscape"].as_bool().unwrap_or(true),
        show_border:sheet["sheet"]["border"].as_bool().unwrap_or(true),
        show_title:sheet["sheet"]["title"].as_bool().unwrap_or(true),
        title_part:sheet["sheet"]["title_part"].as_str().unwrap_or("").into(),
        title_number:sheet["sheet"]["title_number"].as_str().unwrap_or("").into(),
        title_material:sheet["sheet"]["title_material"].as_str().unwrap_or("").into(),
        title_author:sheet["sheet"]["title_author"].as_str().unwrap_or("").into(),
        revisions:sheet["sheet"]["revisions"].as_array().map(|items|
            items.iter().filter_map(|v|Some((
                v[0].as_str()?.to_owned(),v[1].as_str()?.to_owned(),v[2].as_str()?.to_owned()
            ))).collect()).unwrap_or_default(),
        projection_first_angle:sheet["sheet"]["first_angle"].as_bool().unwrap_or(true),
    })
}
