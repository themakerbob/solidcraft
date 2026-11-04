SolidCraft Drawing V0.1.18 — slightly heavier drawing strokes

Apply over V0.1.17. Only file changed:
  crates/ui-egui/src/drawing_workspace.rs

Visible contours: 1.2 -> 1.4 screen points
Hidden dashed lines: 0.8 -> 1.0 screen points
Selected view outline: 1.2 -> 1.8 screen points

Retains hidden-first / visible-last rendering, fixed screen-space dash lengths,
and Visible only (debug). No HLR geometry changes.

Build:
  cargo check -p solidcraft-ui-egui
  cargo run -p solidcraft
