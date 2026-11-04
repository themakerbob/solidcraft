SolidCraft Drawing V0.1.24d — ribbon projected-view and export restoration

Changed: crates/ui-egui/src/drawing_workspace.rs

- Projected View ribbon command enabled when at least one view exists.
- If no view is selected, automatically selects first base view as parent.
- Restores HLR Debug CSV and Selected View DXF export as compact buttons in Browser.
- Removes unused browser_open and views_open state fields.
- No HLR or curve rendering changes.

Extract at repository root; cargo check -p solidcraft-ui-egui
Not compiled in this environment.
