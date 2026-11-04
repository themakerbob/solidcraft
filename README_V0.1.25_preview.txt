SolidCraft Drawing V0.1.25 — shaded-view development preview

Apply from repository root. Only drawing_workspace.rs is replaced.

Added: per-view Shaded, Shaded with visible edges, Shaded with hidden edges.
Shading uses the existing body display triangle mesh, projected to each drawing view, with depth-sorted painter fill. Analytic HLR edges are preserved.

LIMITATIONS: Uncompiled in this environment. Depth sorting is approximate (not a full z-buffer). External-only silhouette and tangent-edge toggles remain unimplemented. Centre marks/lines remain placeholders. Do not treat this as a production-ready release.

Validate:
  cargo check -p solidcraft-ui-egui
  cargo run -p solidcraft

If compilation fails, restore the committed source and share the error output.
