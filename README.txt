SolidCraft Drawing V0.1.28f6 — free-angle radial dimensions and free text notes

Install over V0.1.28f5. Replaces drawing_workspace.rs and toolbar.rs only.
- Circle/arc dimension leaders follow cursor angle on placement and leader dragging.
- Selected radial dimensions have editable leader angle in Dimension Properties.
- New Note ribbon action places a free multiline note; click it to edit in Note Properties.
- Notes can be dragged and deleted and participate in drawing undo/redo.
- Existing Leader tool remains available.
- gpu.rs is not modified.

Build locally:
 cargo check -p solidcraft-ui-egui
 cargo run -p solidcraft

Not compiled against the complete repository here. Drawing persistence in .scdraw is not included.
