SolidCraft Drawing V0.1.1 - workspace integration fix
Apply over V0.1.0 (including solidcraft-geom dependency in crates/ui-egui/Cargo.toml).
Replaces the temporary in-canvas Drawing toggle with the existing workspace menu entry.
Drawing and Design switch without resetting the drawing state; Render remains available.
Run cargo check -p solidcraft-ui-egui then cargo run -p solidcraft.
Not compiled in this environment.
