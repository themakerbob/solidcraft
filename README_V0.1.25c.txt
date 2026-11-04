V0.1.25c SAFETY / PROJECTION PATCH

- Removes experimental egui triangle shading responsible for vertical streaks.
- Shaded modes temporarily render HLR outlines, not true shading.
- Uses dominant-axis projected view placement with diagonal dead zones.
- Shows orientation label in projected-view preview.
- Preserves HLR and Utilities exports.

IMPORTANT: The shared Design GPU shading integration is NOT implemented in this patch.
The existing gpu::render_offscreen is gated behind cfg(test), and a proper GPU render-to-texture drawing integration is required.

No Cargo workspace was built in this environment; compile-test on Windows.
