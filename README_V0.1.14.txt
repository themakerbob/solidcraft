SolidCraft Drawing V0.1.14 — conservative high-curvature HLR coverage
Apply over V0.1.12 and V0.1.13.
This patch prevents the chord-based coincidence pass from suppressing projected curves whose maximum sag exceeds the coincidence tolerance. It adds a targeted regression test. This is a geometry-level fix, not a UI line filter.
Test:
 cargo test -p solidcraft-geom high_sag_edge_on_arc_is_not_removed_by_chord_coincidence
 cargo test -p solidcraft-geom hlr
 cargo run -p solidcraft
Note: Not compiled in this environment. Verify the remaining detached left line and isometric hidden edges visually.
