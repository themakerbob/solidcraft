SolidCraft Drawing V0.1.23 — Curve-aware drawing

Based on V0.1.22. No HLR changes.
- Keeps original Seg2 curve per projected stroke.
- Adaptive screen-space tessellation (0.20 px) and a single joined AA stroke per curve.
- DXF LINE for lines, ARC/CIRCLE for circular curves; adaptive LWPOLYLINE for other curves.
- Covered HLR curves remain only in debug exports.
- Hidden strokes remain below visible strokes.

Install into repository root; run:
 cargo check -p solidcraft-ui-egui
 cargo run -p solidcraft

NOTE: Rust compilation unavailable in the patch environment.
