SolidCraft Drawing V0.1.10 — base view placement and properties

Extract into the repository root, replacing crates/ui-egui/src/drawing_workspace.rs.

Features:
- Base View opens properties palette; cached exact HLR preview follows cursor on sheet.
- Edit selected view orientation, scale and label from same palette.
- Browser Sheet Settings: A0-A4, landscape/portrait, first/third angle preference,
  border and title-block toggles.
- Existing selection, dragging, delete, labels and geometry retained.
- HLR direction correction from V0.1.9 retained.

Known limitations:
- Changes are session-local, not persisted to document.
- First/third angle preference stored in UI but not yet used for projected views.
- Hidden lines toggle currently applies globally, not per view.
- Palette edits are immediate; Done closes, not a transactional Apply/Cancel.
- Rounded-corner HLR artifacts are not fixed in this UI patch.
- Rust compilation not available in this environment; run cargo check -p solidcraft-ui-egui.
