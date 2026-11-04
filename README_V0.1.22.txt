SolidCraft Drawing V0.1.22 — HLR DXF diagnostic export

Extract into SolidCraft repository root, replacing crates/ui-egui/src/drawing_workspace.rs.

Run:
  cargo check -p solidcraft-ui-egui
  cargo run -p solidcraft

Select a drawing view, then click "Export Selected View (DXF)" in Drawing browser.
Output: <repository>/drawing_debug/hlr_Front_<timestamp>.dxf (or selected view orientation).

DXF uses model millimetres (no canvas zoom or sheet placement).
Layers:
  VISIBLE   solid black/white (ACI 7)
  HIDDEN    dashed red (ACI 1)
  UNCERTAIN orange (ACI 30)
  COVERED   magenta (ACI 6), frozen by default

Open in LibreCAD, QCAD, or other DXF-capable CAD application. Toggle layers to
isolate offending radius segments. Upload DXF here for inspection.

This patch does not alter HLR classification, line rendering or CSV export.
