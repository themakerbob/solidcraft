SolidCraft Drawing V0.1.9 — HLR sight-direction correction

Root cause:
NamedView::basis() uses paper axes whose x cross y normal points toward
the viewer. The HLR API requires d to point into the scene, and uses -d
for occlusion rays. V0.1.8 supplied x cross y, reversing the rays.

Fix:
d = basis.y.cross(basis.x)

Install:
Extract ZIP into the SolidCraft repository root, preserving paths.

Verify:
cargo check -p solidcraft-ui-egui
cargo run -p solidcraft

Compare Front/Back/Top/Bottom and isometric views against the model.
This patch does not change geometry generation or other UI behavior.
Rust compilation was not performed in this environment.
