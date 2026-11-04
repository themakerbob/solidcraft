SolidCraft Drawing V0.1.24 — DEVELOPMENT PATCH (not a release)

Apply from repository root. Only changed source files are included.

Included: floating browser palette; per-view visibility, display and labels;
linked orthographic projected views (Front/Top/Left/Right basic mapping),
first-/third-angle selection, projected preview and constrained dragging,
parent movement propagation and cascade deletion.

Not implemented: actual shaded face rendering; silhouette-only extraction;
centre marks/lines; tangent-edge per-view HLR recomputation; custom
isometric projection placement; persistent save/load. Their UI controls
are explicitly marked as reserved or unavailable, not claimed functional.

The project was not compiled in this environment (cargo unavailable).
Build locally:
  cargo check -p solidcraft-ui-egui
  cargo run -p solidcraft

Do not commit as a stable release until build and behaviour are validated.
