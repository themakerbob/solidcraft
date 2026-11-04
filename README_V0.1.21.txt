SolidCraft Drawing V0.1.21 — Safe rollback of broken V0.1.20 stroke consolidation

Restores the V0.1.19 renderer and CSV diagnostics, preserving V0.1.18 line weights.
Includes the Rust Vec<_> type annotation required for V0.1.19 compilation.
Removes the V0.1.20 consolidate_display_strokes() routine, which incorrectly joined edges and removed contours.

Extract into SolidCraft repository root.
Run:
  cargo check -p solidcraft-ui-egui
  cargo run -p solidcraft

This is a rollback and stability patch, not a complete fix for the radius stroke issue.
