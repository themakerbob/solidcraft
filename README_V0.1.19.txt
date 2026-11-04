SolidCraft Drawing V0.1.19 — HLR Debug CSV Export

Copy crates/ui-egui/src/drawing_workspace.rs into the corresponding
location in your SolidCraft checkout (over V0.1.18).

In Drawing, click "Export HLR Debug (CSV)" in the left browser.
A file is written to <current working directory>/drawing_debug/hlr_debug_<timestamp>.csv.

The CSV contains every HLR piece, including covered ones, with:
view, line/piece/segment indices, visibility, covered flag, sampled
projected model-mm coordinates, zoom, view scale and renderer toggles.
Each sampled point has a row; segment identity groups the points.

To diagnose the rounded corners:
1. Open Sample Plate.sc and enter Drawing.
2. Export with hidden lines enabled, at a zoom where artifacts appear.
3. Zoom in until the artifacts disappear; export again.
4. Share both CSV files. The HLR model-mm geometry should be identical
   if the problem is purely rendering-related.

No HLR geometry or visibility calculations are changed by this patch.
Rust compilation not verified in this environment.

Build:
  cargo check -p solidcraft-ui-egui
  cargo run -p solidcraft
