SolidCraft Drawing V0.1.28h18 - shortcut routing hotfix

Requires V0.1.28h17 already installed.

From SolidCraft repository root:
  python path/to/apply_h18.py
  cargo check -p solidcraft-ui-egui
  cargo run -p solidcraft

The script edits only crates/ui-egui/src/toolbar.rs and saves a backup.
It prevents global Model shortcuts from firing while Drawing is active.
Drawing shortcuts (including G = Detail View) remain handled by drawing_workspace.rs.
Global shortcut behaviour outside Drawing is unchanged.

Test G, S, D, N in Drawing, then switch to Model and verify Model shortcuts.
Rust compilation has not been verified in this environment.
